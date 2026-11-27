//! End-to-end API tests. `#[sqlx::test]` creates a fresh database per test
//! and applies the migrations, so these need `DATABASE_URL` pointing at a
//! PostgreSQL 18 server (CI provides one).

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use rollcall::{AppState, config::ScraperConfig, routes, scraper::ScraperService};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

fn app(pool: PgPool) -> Router {
    let state = AppState {
        pool,
        scraper: Arc::new(ScraperService::new(&ScraperConfig::default(), None).unwrap()),
    };
    routes::api(state)
}

async fn call(app: &Router, method: Method, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(body.map_or_else(Body::empty, |b| Body::from(b.to_string())))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, json)
}

fn new_spool(color: &str, hex: &str, weight: f64) -> Value {
    json!({
        "filament": {
            "brand_name": "bambu lab",
            "material_code": "petg",
            "product_name": "PETG HF",
            "color_name": color,
            "color_hex": hex,
            "nozzle_temp_min_c": 230,
            "nozzle_temp_max_c": 260
        },
        "initial_weight_g": 1000,
        "remaining_weight_g": weight,
        "tare_weight_g": 250,
        "purchase_price": 19.99
    })
}

#[sqlx::test]
async fn spool_lifecycle(pool: PgPool) {
    let app = app(pool);

    let (status, spool) = call(
        &app,
        Method::POST,
        "/spools",
        Some(new_spool("Black", "000", 1000.0)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{spool}");
    let id = spool["id"].as_str().unwrap().to_string();
    assert_eq!(
        spool["brand_name"], "Bambu Lab",
        "known brands are canonicalized"
    );
    assert_eq!(spool["material_code"], "PETG");
    assert_eq!(spool["color_hex"], "#000000");
    assert_eq!(spool["status"], "sealed");
    assert_eq!(spool["currency"], "USD");

    // Same product again reuses the filament row.
    let (_, second) = call(
        &app,
        Method::POST,
        "/spools",
        Some(new_spool("black", "#000000", 400.0)),
    )
    .await;
    assert_eq!(second["filament_id"], spool["filament_id"]);

    // Quick adjust opens the spool.
    let (status, change) = call(
        &app,
        Method::POST,
        &format!("/spools/{id}/weight"),
        Some(json!({ "kind": "quick_adjust", "delta_g": -50 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{change}");
    assert_eq!(change["spool"]["remaining_weight_g"], 950.0);
    assert_eq!(change["spool"]["status"], "active");
    assert_eq!(change["event"]["delta_g"], -50.0);

    // Weigh-in subtracts tare: 400 g gross - 250 g tare = 150 g → low stock.
    let (_, change) = call(
        &app,
        Method::POST,
        &format!("/spools/{id}/weight"),
        Some(json!({ "kind": "weigh_in", "gross_reading_g": 400 })),
    )
    .await;
    assert_eq!(change["spool"]["remaining_weight_g"], 150.0);
    assert_eq!(change["spool"]["is_low_stock"], true);

    let (_, stats) = call(&app, Method::GET, "/stats", None).await;
    assert_eq!(stats["spools_in_stock"], 2);
    assert_eq!(stats["low_stock_count"], 1);
    assert_eq!(stats["remaining_kg"], 0.55);

    // Logging a print larger than what's left empties the spool.
    let (_, change) = call(
        &app,
        Method::POST,
        &format!("/spools/{id}/weight"),
        Some(json!({ "kind": "print", "delta_g": -500, "print_name": "Benchy" })),
    )
    .await;
    assert_eq!(change["spool"]["status"], "empty");
    assert_eq!(change["spool"]["remaining_weight_g"], 0.0);
    assert!(change["spool"]["emptied_at"].is_string());

    // Empty spools are hidden by default.
    let (_, list) = call(&app, Method::GET, "/spools", None).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    let (_, list) = call(&app, Method::GET, "/spools?include_empty=true", None).await;
    assert_eq!(list.as_array().unwrap().len(), 2);

    let (_, history) = call(
        &app,
        Method::GET,
        &format!("/spools/{id}/weight-events"),
        None,
    )
    .await;
    let kinds: Vec<&str> = history
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["print", "weigh_in", "quick_adjust", "initial"]);
}

#[sqlx::test]
async fn filters_and_search(pool: PgPool) {
    let app = app(pool);
    let (_, loc) = call(
        &app,
        Method::POST,
        "/locations",
        Some(json!({ "name": "AMS Slot 2", "kind": "ams_slot" })),
    )
    .await;
    let mut body = new_spool("Jade White", "#F4F4F0", 800.0);
    body["location_id"] = loc["id"].clone();
    body["filament"]["material_code"] = json!("PLA");
    call(&app, Method::POST, "/spools", Some(body)).await;
    call(
        &app,
        Method::POST,
        "/spools",
        Some(new_spool("Red", "#FF0000", 900.0)),
    )
    .await;

    let count = |v: &Value| v.as_array().unwrap().len();
    let (_, r) = call(&app, Method::GET, "/spools?material=pla", None).await;
    assert_eq!(count(&r), 1);
    let (_, r) = call(&app, Method::GET, "/spools?material_family=PETG", None).await;
    assert_eq!(count(&r), 1);
    let (_, r) = call(
        &app,
        Method::GET,
        &format!("/spools?location_id={}", loc["id"].as_str().unwrap()),
        None,
    )
    .await;
    assert_eq!(r[0]["location_name"], "AMS Slot 2");
    let (_, r) = call(&app, Method::GET, "/spools?q=jade", None).await;
    assert_eq!(count(&r), 1);
    let (_, r) = call(&app, Method::GET, "/spools?color=re", None).await;
    assert_eq!(r[0]["color_name"], "Red");
    // Wildcards in user input are literal.
    let (_, r) = call(&app, Method::GET, "/spools?q=%25", None).await;
    assert_eq!(count(&r), 0);
}

#[sqlx::test]
async fn validation_errors(pool: PgPool) {
    let app = app(pool);
    let mut bad = new_spool("Blue", "#0000FF", 1000.0);
    bad["filament"]["material_code"] = json!("UNOBTAINIUM");
    let (status, body) = call(&app, Method::POST, "/spools", Some(bad)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "bad_request");

    let mut bad = new_spool("Blue", "#0000FF", 1000.0);
    bad["filament"]["nozzle_temp_min_c"] = json!(900);
    let (status, _) = call(&app, Method::POST, "/spools", Some(bad)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, _) = call(
        &app,
        Method::GET,
        "/spools/00000000-0000-0000-0000-000000000000",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (_, spool) = call(
        &app,
        Method::POST,
        "/spools",
        Some(new_spool("Blue", "#0000FF", 1000.0)),
    )
    .await;
    let (status, body) = call(
        &app,
        Method::POST,
        &format!("/spools/{}/weight", spool["id"].as_str().unwrap()),
        Some(json!({ "kind": "print", "delta_g": 25 })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    let (status, _) = call(
        &app,
        Method::POST,
        "/filaments/parse-url",
        Some(json!({ "url": "http://127.0.0.1/admin" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// The built frontend is served from STATIC_DIR, and any other path that
/// isn't a file gets index.html with 200, so links to client routes work.
#[sqlx::test]
async fn serves_frontend_with_spa_fallback(pool: PgPool) {
    let dir = std::env::temp_dir().join(format!("rollcall-static-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("_app")).unwrap();
    std::fs::write(dir.join("index.html"), "<title>Rollcall</title>").unwrap();
    std::fs::write(dir.join("_app/app.js"), "console.log(1)").unwrap();

    let config = rollcall::config::Config {
        database_url: String::new(),
        bind_addr: String::new(),
        static_dir: Some(dir.clone()),
        cors_origin: None,
        scraper: ScraperConfig::default(),
        llm: None,
    };
    let state = AppState {
        pool,
        scraper: Arc::new(ScraperService::new(&ScraperConfig::default(), None).unwrap()),
    };
    let app = routes::router(state, &config);

    for (uri, expected) in [
        ("/", "<title>Rollcall</title>"),
        ("/spools/some-id", "<title>Rollcall</title>"),
        ("/_app/app.js", "console.log(1)"),
    ] {
        let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "{uri}");
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(String::from_utf8_lossy(&bytes), expected, "{uri}");
    }
    let (status, body) = call(&app, Method::GET, "/api/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");

    std::fs::remove_dir_all(dir).ok();
}
