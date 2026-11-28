//! End-to-end API tests. `#[sqlx::test]` creates a fresh database per test
//! and applies the migrations, so these need `DATABASE_URL` pointing at a
//! PostgreSQL 18 server (CI provides one).

use axum::{
    Router,
    body::Body,
    http::{HeaderMap, HeaderValue, Method, Request, StatusCode, header},
};
use http_body_util::BodyExt;
use rollcall::{AppState, config::ScraperConfig, routes, scraper::ScraperService};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

/// A fresh, empty folder for one test's uploads.
fn test_upload_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("rollcall-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn state(pool: PgPool) -> AppState {
    AppState {
        pool,
        scraper: Arc::new(ScraperService::new(&ScraperConfig::default(), None).unwrap()),
        uploads: Arc::new(test_upload_dir()),
    }
}

async fn app(pool: PgPool) -> Router {
    signed_in(state(pool)).await
}

/// The API, signed in as a new admin through an API token on every request.
async fn signed_in(state: AppState) -> Router {
    let pool = state.pool.clone();
    let hash = rollcall::auth::hash_password("test password")
        .await
        .unwrap();
    let user = rollcall::repo::accounts::create_user(&pool, "tester", &hash, false)
        .await
        .unwrap();
    let secret = rollcall::auth::new_secret();
    rollcall::repo::accounts::create_token(
        &pool,
        user.id,
        "tests",
        &rollcall::auth::hash_secret(&secret),
    )
    .await
    .unwrap();
    let bearer = HeaderValue::from_str(&format!("Bearer {secret}")).unwrap();
    routes::api(state).layer(axum::middleware::map_request(
        move |mut req: Request<Body>| {
            let bearer = bearer.clone();
            async move {
                req.headers_mut()
                    .entry(header::AUTHORIZATION)
                    .or_insert(bearer);
                req
            }
        },
    ))
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
    let app = app(pool).await;

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
    let app = app(pool).await;
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
    let app = app(pool).await;
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
        upload_dir: test_upload_dir(),
    };
    let state = AppState {
        pool,
        scraper: Arc::new(ScraperService::new(&ScraperConfig::default(), None).unwrap()),
        uploads: Arc::new(config.upload_dir.clone()),
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

#[sqlx::test]
async fn edit_filament(pool: PgPool) {
    let app = app(pool).await;
    let (status, spool) = call(
        &app,
        Method::POST,
        "/spools",
        Some(new_spool("Black", "#000000", 800.0)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let filament_id = spool["filament_id"].as_str().unwrap().to_string();
    let uri = format!("/filaments/{filament_id}");

    let edit = json!({
        "brand_name": "polymaker",
        "material_code": "pla",
        "product_name": "PolyTerra",
        "color_name": " Charcoal Black ",
        "color_hex": "222",
        "diameter_mm": 1.75,
        "nozzle_temp_min_c": 190,
        "nozzle_temp_max_c": 220,
        "td_value": 1.2
    });
    let (status, f) = call(&app, Method::PUT, &uri, Some(edit.clone())).await;
    assert_eq!(status, StatusCode::OK, "{f}");
    assert_eq!(f["brand_name"], "Polymaker");
    assert_eq!(f["material_code"], "PLA");
    assert_eq!(f["color_name"], "Charcoal Black");
    assert_eq!(f["color_hex"], "#222222");
    assert_eq!(f["nozzle_temp_max_c"], 220);
    // Fields left out are cleared, like the spool PUT.
    assert_eq!(f["bed_temp_min_c"], Value::Null);

    // The spool shows the new details.
    let (_, s) = call(
        &app,
        Method::GET,
        &format!("/spools/{}", spool["id"].as_str().unwrap()),
        None,
    )
    .await;
    assert_eq!(s["brand_name"], "Polymaker");
    assert_eq!(s["color_name"], "Charcoal Black");

    // Bad input is a 400, a missing filament a 404.
    let mut bad = edit.clone();
    bad["material_code"] = json!("NOPE");
    let (status, _) = call(&app, Method::PUT, &uri, Some(bad)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let mut bad = edit.clone();
    bad["nozzle_temp_min_c"] = json!(260);
    let (status, _) = call(&app, Method::PUT, &uri, Some(bad)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = call(
        &app,
        Method::PUT,
        "/filaments/00000000-0000-7000-8000-000000000000",
        Some(edit),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn settings_printers_and_locations(pool: PgPool) {
    let app = app(pool).await;

    // Settings: defaults, update, and the dashboard follows the threshold.
    let (status, s) = call(&app, Method::GET, "/settings", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(s["low_stock_threshold_g"], 200.0);
    assert_eq!(s["default_currency"], "USD");
    call(
        &app,
        Method::POST,
        "/spools",
        Some(new_spool("Red", "#FF0000", 300.0)),
    )
    .await;
    let (_, stats) = call(&app, Method::GET, "/stats", None).await;
    assert_eq!(stats["low_stock_count"], 0);
    let (status, s) = call(
        &app,
        Method::PUT,
        "/settings",
        Some(json!({ "low_stock_threshold_g": 350, "default_currency": " eur " })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{s}");
    assert_eq!(s["default_currency"], "EUR");
    let (_, stats) = call(&app, Method::GET, "/stats", None).await;
    assert_eq!(stats["low_stock_count"], 1);
    let (status, _) = call(
        &app,
        Method::PUT,
        "/settings",
        Some(json!({ "low_stock_threshold_g": -1, "default_currency": "USD" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Printers: only one default at a time.
    let (status, x1) = call(
        &app,
        Method::POST,
        "/printers",
        Some(json!({ "name": " X1C ", "model": "Bambu Lab X1 Carbon", "is_default": true })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{x1}");
    assert_eq!(x1["name"], "X1C");
    assert_eq!(x1["nozzle_diameter_mm"], 0.4);
    let (_, mk4) = call(
        &app,
        Method::POST,
        "/printers",
        Some(json!({ "name": "MK4", "nozzle_diameter_mm": 0.6, "is_default": true })),
    )
    .await;
    let (_, list) = call(&app, Method::GET, "/printers", None).await;
    let defaults: Vec<_> = list
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["is_default"] == true)
        .map(|p| p["name"].clone())
        .collect();
    assert_eq!(defaults, vec![json!("MK4")]);
    let x1_uri = format!("/printers/{}", x1["id"].as_str().unwrap());
    let (status, x1b) = call(
        &app,
        Method::PUT,
        &x1_uri,
        Some(json!({ "name": "X1C", "notes": "AMS", "is_default": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(x1b["is_default"], true);
    let (_, list) = call(&app, Method::GET, "/printers", None).await;
    assert_eq!(list[0]["name"], "X1C");
    assert_eq!(list[1]["is_default"], false);
    let (status, _) = call(
        &app,
        Method::POST,
        "/printers",
        Some(json!({ "name": "x1c" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let mk4_uri = format!("/printers/{}", mk4["id"].as_str().unwrap());
    let (status, _) = call(&app, Method::DELETE, &mk4_uri, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&app, Method::DELETE, &mk4_uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Locations: rename, and deleting one leaves its spools without a location.
    let (_, loc) = call(
        &app,
        Method::POST,
        "/locations",
        Some(json!({ "name": "Dry Box 1", "kind": "dry_box" })),
    )
    .await;
    let loc_uri = format!("/locations/{}", loc["id"].as_str().unwrap());
    let (status, loc2) = call(
        &app,
        Method::PUT,
        &loc_uri,
        Some(
            json!({ "name": "Dry Box A", "kind": "dry_box", "description": "  ", "sort_order": 3 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(loc2["name"], "Dry Box A");
    assert_eq!(loc2["description"], Value::Null);
    assert_eq!(loc2["sort_order"], 3);
    let mut body = new_spool("Blue", "#0000FF", 900.0);
    body["location_id"] = loc["id"].clone();
    let (_, spool) = call(&app, Method::POST, "/spools", Some(body)).await;
    assert_eq!(spool["location_name"], "Dry Box A");
    let (status, _) = call(&app, Method::DELETE, &loc_uri, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, spool) = call(
        &app,
        Method::GET,
        &format!("/spools/{}", spool["id"].as_str().unwrap()),
        None,
    )
    .await;
    assert_eq!(spool["location_id"], Value::Null);
    let (status, _) = call(
        &app,
        Method::PUT,
        &loc_uri,
        Some(json!({ "name": "Gone", "kind": "other" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn tuned_settings_per_printer(pool: PgPool) {
    let app = app(pool).await;
    let (_, spool) = call(
        &app,
        Method::POST,
        "/spools",
        Some(new_spool("Red", "#FF0000", 1000.0)),
    )
    .await;
    let fid = spool["filament_id"].as_str().unwrap().to_string();
    let (_, x1) = call(
        &app,
        Method::POST,
        "/printers",
        Some(json!({ "name": "X1C", "is_default": true })),
    )
    .await;
    let x1_id = x1["id"].as_str().unwrap().to_string();
    let uri = format!("/filaments/{fid}/tuned");

    let (status, list) = call(&app, Method::GET, &uri, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list, json!([]));

    // One row for the printer, one for any printer.
    let (status, t) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({
            "printer_id": x1_id, "nozzle_temp_c": 215, "flow_ratio": 0.98,
            "pressure_advance": 0.025, "notes": "  matte finish  ", "verified": true
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{t}");
    assert_eq!(t["printer_name"], "X1C");
    assert_eq!(t["flow_ratio"], 0.98);
    assert_eq!(t["notes"], "matte finish");
    let verified_at = t["verified_at"].clone();
    assert!(verified_at.is_string());
    let (_, any) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "printer_id": null, "bed_temp_c": 60 })),
    )
    .await;
    assert!(any["printer_name"].is_null());
    assert!(any["verified_at"].is_null());

    // Saving again replaces the row and keeps the first verified date.
    let (_, t2) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "printer_id": x1_id, "nozzle_temp_c": 220, "verified": true })),
    )
    .await;
    assert_eq!(t2["id"], t["id"]);
    assert_eq!(t2["nozzle_temp_c"], 220);
    assert!(t2["flow_ratio"].is_null());
    assert_eq!(t2["verified_at"], verified_at);
    let (_, t3) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "printer_id": x1_id, "nozzle_temp_c": 220 })),
    )
    .await;
    assert!(t3["verified_at"].is_null());

    // The any-printer row is listed first.
    let (_, list) = call(&app, Method::GET, &uri, None).await;
    let list = list.as_array().unwrap();
    assert_eq!(list.len(), 2);
    assert!(list[0]["printer_id"].is_null());
    assert_eq!(list[1]["printer_id"], x1_id.as_str());

    // Bad values, an unknown printer, and an unknown filament.
    let (status, _) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "printer_id": x1_id, "flow_ratio": 3 })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "printer_id": "0193b0c4-8a7e-7c1e-9d2f-5b1a2c3d4e5f" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "not one of your printers");
    let missing = "/filaments/0193b0c4-8a7e-7c1e-9d2f-5b1a2c3d4e5f/tuned";
    let (status, _) = call(&app, Method::GET, missing, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(&app, Method::PUT, missing, Some(json!({}))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Deleting a printer removes its settings; deleting a row works once.
    let (status, _) = call(&app, Method::DELETE, &format!("/printers/{x1_id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, list) = call(&app, Method::GET, &uri, None).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    let any_id = any["id"].as_str().unwrap();
    let (status, _) = call(&app, Method::DELETE, &format!("/tuned/{any_id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&app, Method::DELETE, &format!("/tuned/{any_id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn acquisition_and_discount(pool: PgPool) {
    let app = app(pool).await;

    // Defaults: bought, no discount.
    let (_, plain) = call(
        &app,
        Method::POST,
        "/spools",
        Some(new_spool("Red", "#FF0000", 1000.0)),
    )
    .await;
    assert_eq!(plain["acquisition"], "bought");
    assert_eq!(plain["discounted"], false);
    assert!(plain["discount_note"].is_null());

    // A discounted purchase keeps its note; a gift can have no price.
    let mut body = new_spool("Blue", "#0000FF", 1000.0);
    body["discounted"] = json!(true);
    body["discount_note"] = json!("  Black Friday, 30% off  ");
    body["vendor"] = json!("Micro Center");
    body["purchase_date"] = json!("2026-11-27");
    let (status, s) = call(&app, Method::POST, "/spools", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{s}");
    assert_eq!(s["discount_note"], "Black Friday, 30% off");
    assert_eq!(s["vendor"], "Micro Center");
    assert_eq!(s["purchase_date"], "2026-11-27");

    let id = s["id"].as_str().unwrap();
    let edit = json!({
        "filament_id": s["filament_id"], "status": "sealed", "initial_weight_g": 1000,
        "currency": "USD", "acquisition": "gift", "vendor": "Sam",
        "discounted": false, "discount_note": "ignored without a discount"
    });
    let (status, g) = call(&app, Method::PUT, &format!("/spools/{id}"), Some(edit)).await;
    assert_eq!(status, StatusCode::OK, "{g}");
    assert_eq!(g["acquisition"], "gift");
    assert!(g["purchase_price"].is_null());
    assert_eq!(g["discounted"], false);
    assert!(g["discount_note"].is_null());
}

#[sqlx::test]
async fn humidity_readings(pool: PgPool) {
    let app = app(pool).await;
    let (_, spool) = call(
        &app,
        Method::POST,
        "/spools",
        Some(new_spool("Red", "#FF0000", 1000.0)),
    )
    .await;
    let uri = format!("/spools/{}/humidity", spool["id"].as_str().unwrap());

    let (status, list) = call(&app, Method::GET, &uri, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list, json!([]));

    let (status, older) = call(
        &app,
        Method::POST,
        &uri,
        Some(
            json!({ "measured_at": "2026-11-20T12:00:00Z", "humidity_pct": 42.5,
                     "temperature_c": 21, "note": " in the dry box " }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{older}");
    assert_eq!(older["humidity_pct"], 42.5);
    assert_eq!(older["note"], "in the dry box");
    // No date means now.
    let (_, newer) = call(
        &app,
        Method::POST,
        &uri,
        Some(json!({ "humidity_pct": 18 })),
    )
    .await;
    assert!(newer["temperature_c"].is_null());

    let (_, list) = call(&app, Method::GET, &uri, None).await;
    let ids: Vec<_> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].clone())
        .collect();
    assert_eq!(ids, vec![newer["id"].clone(), older["id"].clone()]);

    let (status, _) = call(
        &app,
        Method::POST,
        &uri,
        Some(json!({ "humidity_pct": 120 })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let missing = "/spools/0193b0c4-8a7e-7c1e-9d2f-5b1a2c3d4e5f/humidity";
    let (status, _) = call(&app, Method::GET, missing, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(
        &app,
        Method::POST,
        missing,
        Some(json!({ "humidity_pct": 30 })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let del = format!("/humidity/{}", older["id"].as_str().unwrap());
    let (status, _) = call(&app, Method::DELETE, &del, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&app, Method::DELETE, &del, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Deleting the spool deletes its readings.
    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("/spools/{}", spool["id"].as_str().unwrap()),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn equipment_and_accessories(pool: PgPool) {
    let app = app(pool).await;
    let (_, x1) = call(
        &app,
        Method::POST,
        "/printers",
        Some(json!({ "name": "X1C", "is_default": true })),
    )
    .await;
    let (_, mk4) = call(
        &app,
        Method::POST,
        "/printers",
        Some(json!({ "name": "MK4" })),
    )
    .await;
    let x1_id = x1["id"].as_str().unwrap().to_string();

    let (status, nozzle) = call(
        &app,
        Method::POST,
        "/equipment",
        Some(json!({
            "category": "equipment", "kind": "Nozzle", "name": " Hardened steel 0.4 ",
            "brand_name": "bambu lab", "printer_id": x1_id, "quantity": 2,
            "acquired_on": "2026-11-01", "acquired_from": "store.bambulab.com", "price": 29.99,
            "discounted": true, "discount_note": "Bundle"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{nozzle}");
    assert_eq!(nozzle["name"], "Hardened steel 0.4");
    assert_eq!(nozzle["brand_name"], "bambu lab");
    assert_eq!(nozzle["printer_name"], "X1C");
    assert_eq!(nozzle["quantity"], 2);
    assert_eq!(nozzle["currency"], "USD");
    assert_eq!(nozzle["acquisition"], "bought");
    assert_eq!(nozzle["discount_note"], "Bundle");

    // The brand is shared with filaments, matched without case.
    let (_, spool) = call(
        &app,
        Method::POST,
        "/spools",
        Some(new_spool("Red", "#FF0000", 1000.0)),
    )
    .await;
    assert_eq!(spool["brand_id"], nozzle["brand_id"]);

    let (_, plate) = call(
        &app,
        Method::POST,
        "/equipment",
        Some(
            json!({ "category": "accessory", "kind": "Build plate", "name": "Textured PEI",
                     "brand_name": "Prusa", "printer_id": mk4["id"], "acquisition": "gift" }),
        ),
    )
    .await;
    assert_eq!(plate["acquisition"], "gift");
    let (_, tool) = call(
        &app,
        Method::POST,
        "/equipment",
        Some(json!({ "category": "accessory", "kind": "Tool", "name": "Flush cutters" })),
    )
    .await;
    assert!(tool["brand_id"].is_null());
    assert!(tool["printer_id"].is_null());

    async fn names(app: &Router, uri: &str) -> Vec<String> {
        let (status, v) = call(app, Method::GET, uri, None).await;
        assert_eq!(status, StatusCode::OK, "{uri}: {v}");
        v.as_array()
            .unwrap()
            .iter()
            .map(|e| e["name"].as_str().unwrap().to_string())
            .collect()
    }
    assert_eq!(names(&app, "/equipment").await.len(), 3);
    assert_eq!(
        names(&app, "/equipment?category=accessory").await,
        ["Textured PEI", "Flush cutters"]
    );
    let brand = nozzle["brand_id"].as_str().unwrap();
    assert_eq!(
        names(&app, &format!("/equipment?brand_id={brand}")).await,
        ["Hardened steel 0.4"]
    );
    assert_eq!(
        names(&app, &format!("/equipment?printer={x1_id}")).await,
        ["Hardened steel 0.4"]
    );
    assert_eq!(
        names(&app, "/equipment?printer=none").await,
        ["Flush cutters"]
    );
    assert_eq!(names(&app, "/equipment?q=pei").await, ["Textured PEI"]);
    let (status, _) = call(&app, Method::GET, "/equipment?printer=x1", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Edit: retire the nozzle and clear its discount. Retired items are hidden by default.
    let id = nozzle["id"].as_str().unwrap();
    let (status, edited) = call(
        &app,
        Method::PUT,
        &format!("/equipment/{id}"),
        Some(
            json!({ "category": "equipment", "kind": "Nozzle", "name": "Hardened steel 0.4",
                     "brand_name": "Bambu Lab", "printer_id": x1_id, "retired": true,
                     "discount_note": "dropped" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{edited}");
    assert_eq!(edited["retired"], true);
    assert_eq!(edited["quantity"], 1);
    assert!(edited["discount_note"].is_null());
    assert_eq!(names(&app, "/equipment").await.len(), 2);
    assert_eq!(
        names(&app, "/equipment?include_retired=true").await.len(),
        3
    );
    let (status, _) = call(&app, Method::GET, &format!("/equipment/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);

    // Errors.
    let (status, _) = call(
        &app,
        Method::POST,
        "/equipment",
        Some(json!({ "category": "accessory", "name": "  " })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = call(
        &app,
        Method::POST,
        "/equipment",
        Some(json!({ "category": "accessory", "name": "Spare", "quantity": -1 })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let missing = "/equipment/0193b0c4-8a7e-7c1e-9d2f-5b1a2c3d4e5f";
    let (status, _) = call(
        &app,
        Method::PUT,
        missing,
        Some(json!({ "category": "accessory", "name": "Ghost" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Deleting a printer unlinks its items; deleting an item works once.
    call(
        &app,
        Method::DELETE,
        &format!("/printers/{}", mk4["id"].as_str().unwrap()),
        None,
    )
    .await;
    let (_, p) = call(
        &app,
        Method::GET,
        &format!("/equipment/{}", plate["id"].as_str().unwrap()),
        None,
    )
    .await;
    assert!(p["printer_id"].is_null());
    let (status, _) = call(&app, Method::DELETE, &format!("/equipment/{id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&app, Method::DELETE, &format!("/equipment/{id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// A `multipart/form-data` upload: text fields, then an optional file.
async fn upload(
    app: &Router,
    fields: &[(&str, &str)],
    file: Option<(&str, &[u8])>,
) -> (StatusCode, Value) {
    const B: &str = "rollcall-test-boundary";
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(
            format!("--{B}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n")
                .as_bytes(),
        );
    }
    if let Some((file_name, bytes)) = file {
        body.extend_from_slice(
            format!(
                "--{B}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{file_name}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{B}--\r\n").as_bytes());
    let req = Request::builder()
        .method(Method::POST)
        .uri("/photos")
        .header("content-type", format!("multipart/form-data; boundary={B}"))
        .body(Body::from(body))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[sqlx::test]
async fn photos(pool: PgPool) {
    let dir = test_upload_dir();
    let app = signed_in(AppState {
        pool: pool.clone(),
        scraper: Arc::new(ScraperService::new(&ScraperConfig::default(), None).unwrap()),
        uploads: Arc::new(dir.clone()),
    })
    .await;
    let files = || std::fs::read_dir(&dir).unwrap().count();
    let png: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR fake png body";
    let jpg: &[u8] = b"\xFF\xD8\xFF\xE0\0\x10JFIF fake jpeg body";

    let (_, spool) = call(
        &app,
        Method::POST,
        "/spools",
        Some(new_spool("Red", "#ff0000", 900.0)),
    )
    .await;
    let spool_id = spool["id"].as_str().unwrap().to_string();
    let filament_id = spool["filament_id"].as_str().unwrap().to_string();
    let (_, item) = call(
        &app,
        Method::POST,
        "/equipment",
        Some(json!({ "category": "accessory", "name": "Textured PEI plate" })),
    )
    .await;
    let item_id = item["id"].as_str().unwrap().to_string();

    // One photo for each kind of owner.
    let (status, p1) = upload(
        &app,
        &[("spool_id", &spool_id), ("caption", " On the shelf ")],
        Some(("IMG_1.png", png)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{p1}");
    assert_eq!(p1["content_type"], "image/png");
    assert_eq!(p1["caption"], "On the shelf");
    assert_eq!(p1["original_name"], "IMG_1.png");
    assert_eq!(p1["size_bytes"], png.len());
    let p1_id = p1["id"].as_str().unwrap().to_string();
    assert_eq!(p1["url"], format!("/api/photos/{p1_id}/file"));
    let (status, p2) = upload(
        &app,
        &[("filament_id", &filament_id)],
        Some(("swatch.jpg", jpg)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{p2}");
    assert_eq!(p2["content_type"], "image/jpeg");
    let (status, p3) = upload(
        &app,
        &[("equipment_id", &item_id)],
        Some(("plate.jpg", jpg)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{p3}");
    assert_eq!(files(), 3);

    // The type comes from the bytes, not the name.
    let (status, _) = upload(
        &app,
        &[("spool_id", &spool_id)],
        Some(("evil.png", b"<svg onload=alert(1)>")),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = upload(&app, &[("spool_id", &spool_id)], None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = upload(&app, &[], Some(("a.png", png))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = upload(
        &app,
        &[("spool_id", &spool_id), ("equipment_id", &item_id)],
        Some(("a.png", png)),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = upload(&app, &[("spool_id", "not-an-id")], Some(("a.png", png))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let missing = uuid::Uuid::now_v7().to_string();
    let (status, _) = upload(&app, &[("equipment_id", &missing)], Some(("a.png", png))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(files(), 3, "rejected uploads leave no files");

    // Listing needs exactly one owner.
    let (status, list) = call(
        &app,
        Method::GET,
        &format!("/photos?spool_id={spool_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["id"], p1_id.as_str());
    let (_, list) = call(
        &app,
        Method::GET,
        &format!("/photos?filament_id={filament_id}"),
        None,
    )
    .await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    let (status, _) = call(&app, Method::GET, "/photos", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // The file itself, with headers that keep it an image.
    let req = Request::builder()
        .uri(format!("/photos/{p1_id}/file"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let h = resp.headers();
    assert_eq!(h["content-type"], "image/png");
    assert_eq!(h["x-content-type-options"], "nosniff");
    assert!(
        h["content-security-policy"]
            .to_str()
            .unwrap()
            .contains("sandbox")
    );
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], png);

    // Captions can be changed and cleared.
    let (status, p) = call(
        &app,
        Method::PATCH,
        &format!("/photos/{p1_id}"),
        Some(json!({ "caption": "Opened" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(p["caption"], "Opened");
    let (_, p) = call(
        &app,
        Method::PATCH,
        &format!("/photos/{p1_id}"),
        Some(json!({ "caption": "  " })),
    )
    .await;
    assert_eq!(p["caption"], Value::Null);

    // Deleting a photo removes its file.
    let (status, _) = call(&app, Method::DELETE, &format!("/photos/{p1_id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(files(), 2);
    let (status, _) = call(&app, Method::GET, &format!("/photos/{p1_id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // The first photo is the equipment card's picture.
    let (_, e) = call(&app, Method::GET, &format!("/equipment/{item_id}"), None).await;
    assert_eq!(e["photo_url"], p3["url"]);

    // Deleting equipment takes its photos and their files with it.
    let (status, _) = call(&app, Method::DELETE, &format!("/equipment/{item_id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(files(), 1);
    let (status, _) = call(
        &app,
        Method::GET,
        &format!("/photos?equipment_id={item_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // The startup sweep removes stray files the app wrote, and nothing else.
    std::fs::write(dir.join(format!("{}.png", uuid::Uuid::now_v7())), png).unwrap();
    std::fs::write(dir.join("keep-me.txt"), b"not ours").unwrap();
    let removed = rollcall::repo::photos::remove_orphan_files(&pool, &dir)
        .await
        .unwrap();
    assert_eq!(removed, 1);
    assert!(dir.join("keep-me.txt").exists());
    assert_eq!(files(), 2);
    std::fs::remove_dir_all(&dir).unwrap();
}

async fn post_gcode(app: &Router, name: &str, gcode: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/prints?name={name}"))
        .header("content-type", "text/plain")
        .body(Body::from(gcode.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn orca_gcode(slots: &[(&str, &str)]) -> String {
    let profiles: Vec<String> = slots.iter().map(|(p, _)| format!("\"{p}\"")).collect();
    let grams: Vec<&str> = slots.iter().map(|(_, g)| *g).collect();
    format!(
        "; HEADER_BLOCK_START\n; generated by OrcaSlicer 2.3.1 on 2026-11-27 at 10:00:00 UTC\n; HEADER_BLOCK_END\n\
         ; CONFIG_BLOCK_START\n; filament_settings_id = {}\n; filament_type = PETG;PETG\n\
         ; filament_colour = #FF0000;#0000FF\n; printer_settings_id = \"X1C 0.4\"\n; CONFIG_BLOCK_END\n\
         ; filament used [g] = {}\n",
        profiles.join(";"),
        grams.join(", ")
    )
}

#[sqlx::test]
async fn prints_from_the_slicer(pool: PgPool) {
    let app = app(pool).await;
    let weight = |s: &Value| s["remaining_weight_g"].as_f64().unwrap();
    let (_, red) = call(
        &app,
        Method::POST,
        "/spools",
        Some(new_spool("Red", "#ff0000", 900.0)),
    )
    .await;
    let (_, blue) = call(
        &app,
        Method::POST,
        "/spools",
        Some(new_spool("Blue", "#0000ff", 500.0)),
    )
    .await;
    let red_id = red["id"].as_str().unwrap().to_string();
    let blue_id = blue["id"].as_str().unwrap().to_string();
    let red_filament = red["filament_id"].as_str().unwrap().to_string();
    let spool = |id: String| {
        let app = app.clone();
        async move {
            call(&app, Method::GET, &format!("/spools/{id}"), None)
                .await
                .1
        }
    };

    // Exporting a preset from Rollcall records which filament it is.
    let (status, _) = call(
        &app,
        Method::PUT,
        "/slicer-profiles",
        Some(json!({ "name": "Red PETG (Rollcall)", "filament_id": red_filament })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // A known preset is logged by itself; an unknown one waits for a spool.
    let gcode = orca_gcode(&[
        ("Red PETG (Rollcall)", "12.34"),
        ("Generic PETG @System", "1.26"),
    ]);
    let (status, print) = post_gcode(&app, "Benchy_PETG.gcode", &gcode).await;
    assert_eq!(status, StatusCode::CREATED, "{print}");
    assert_eq!(print["name"], "Benchy_PETG");
    assert_eq!(print["slicer"], "OrcaSlicer 2.3.1");
    assert_eq!(print["printer_preset"], "X1C 0.4");
    let id = print["id"].as_str().unwrap().to_string();
    let slots = print["slots"].as_array().unwrap();
    assert_eq!(slots.len(), 2);
    assert_eq!(slots[0]["status"], "logged");
    assert_eq!(slots[0]["spool_id"], red_id.as_str());
    assert_eq!(slots[0]["spool"]["color_name"], "Red");
    assert_eq!(slots[0]["profile_filament_id"], red_filament.as_str());
    assert_eq!(slots[1]["status"], "pending");
    assert_eq!(slots[1]["color_hex"], "#0000FF");
    let red_now = spool(red_id.clone()).await;
    assert_eq!(weight(&red_now), 887.7);
    assert_eq!(red_now["status"], "active");

    // The same upload again (a slicer can run its scripts twice) isn't logged twice.
    let (status, again) = post_gcode(&app, "Benchy_PETG.gcode", &gcode).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(again["id"], id.as_str());
    assert_eq!(weight(&spool(red_id.clone()).await), 887.7);

    let (_, pending) = call(&app, Method::GET, "/prints?pending=true", None).await;
    assert_eq!(pending.as_array().unwrap().len(), 1);

    // Picking the spool logs it and remembers the preset.
    let (status, p) = call(
        &app,
        Method::PUT,
        &format!("/prints/{id}/slots/2"),
        Some(json!({ "spool_id": blue_id })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{p}");
    assert_eq!(p["slots"][1]["status"], "logged");
    assert_eq!(weight(&spool(blue_id.clone()).await), 498.7);
    let (_, pending) = call(&app, Method::GET, "/prints?pending=true", None).await;
    assert_eq!(pending.as_array().unwrap().len(), 0);
    let (status, _) = call(
        &app,
        Method::PUT,
        &format!("/prints/{id}/slots/2"),
        Some(json!({ "spool_id": blue_id })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (_, profiles) = call(&app, Method::GET, "/slicer-profiles", None).await;
    assert_eq!(profiles.as_array().unwrap().len(), 2);

    // Undo gives the filament back; skip and bad requests.
    let (status, p) = call(
        &app,
        Method::POST,
        &format!("/prints/{id}/slots/1/undo"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(p["slots"][0]["status"], "pending");
    assert_eq!(weight(&spool(red_id.clone()).await), 900.0);
    let (_, p) = call(
        &app,
        Method::PUT,
        &format!("/prints/{id}/slots/1"),
        Some(json!({ "skip": true })),
    )
    .await;
    assert_eq!(p["slots"][0]["status"], "skipped");
    let (status, _) = call(
        &app,
        Method::PUT,
        &format!("/prints/{id}/slots/1"),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = call(
        &app,
        Method::PUT,
        &format!("/prints/{id}/slots/9"),
        Some(json!({ "skip": true })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // The next print with the remembered preset is logged by itself.
    let (_, p2) = post_gcode(
        &app,
        "Cube.gcode",
        &orca_gcode(&[("Generic PETG @System", "5")]),
    )
    .await;
    assert_eq!(p2["slots"][0]["status"], "logged");
    assert_eq!(weight(&spool(blue_id.clone()).await), 493.7);

    // Two open spools of the same filament: Rollcall can't tell, so it waits.
    let (_, _) = call(
        &app,
        Method::POST,
        "/spools",
        Some(json!({ "filament_id": red_filament, "initial_weight_g": 1000, "status": "active" })),
    )
    .await;
    let (_, p3) = post_gcode(
        &app,
        "Vase.gcode",
        &orca_gcode(&[("Red PETG (Rollcall)", "30")]),
    )
    .await;
    assert_eq!(p3["slots"][0]["status"], "pending");

    // Deleting a print gives back what it took.
    let (status, _) = call(&app, Method::DELETE, &format!("/prints/{id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(weight(&spool(blue_id.clone()).await), 495.0);
    let (status, _) = call(&app, Method::GET, &format!("/prints/{id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, all) = call(&app, Method::GET, "/prints", None).await;
    assert_eq!(all.as_array().unwrap().len(), 2);

    // Not slicer G-code.
    let (status, _) = post_gcode(&app, "x.gcode", "G1 X0\nG1 X1\n").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (_, mine) = call(
        &app,
        Method::GET,
        &format!("/slicer-profiles?filament_id={red_filament}"),
        None,
    )
    .await;
    assert_eq!(mine.as_array().unwrap().len(), 1);
    let (status, _) = call(
        &app,
        Method::DELETE,
        "/slicer-profiles/Red%20PETG%20(Rollcall)",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// A request with optional `Cookie` or `Authorization` header, returning the response headers too.
async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    auth: Option<(header::HeaderName, String)>,
) -> (StatusCode, HeaderMap, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some((name, value)) = auth {
        req = req.header(name, value);
    }
    let req = req
        .body(body.map_or_else(Body::empty, |b| Body::from(b.to_string())))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, headers, json)
}

/// The `rollcall_session=...` pair from a response's Set-Cookie.
fn cookie(headers: &HeaderMap) -> (header::HeaderName, String) {
    let set = headers[header::SET_COOKIE].to_str().unwrap();
    (header::COOKIE, set.split(';').next().unwrap().to_string())
}

#[sqlx::test]
async fn accounts_and_sign_in(pool: PgPool) {
    let app = routes::api(state(pool));
    let creds = |u: &str, p: &str| Some(json!({ "username": u, "password": p }));

    // Nothing but health and the sign-in routes without an account.
    let (status, _, _) = send(&app, Method::GET, "/spools", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _, _) = send(&app, Method::GET, "/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let (_, _, st) = send(&app, Method::GET, "/auth/status", None, None).await;
    assert_eq!(st["setup_needed"], true);
    assert_eq!(st["registration_enabled"], false);
    assert!(st["user"].is_null());

    // The first account is the admin, even with registration off.
    let (status, _, body) = send(
        &app,
        Method::POST,
        "/auth/register",
        creds("ad", "short"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    let (status, h, admin) = send(
        &app,
        Method::POST,
        "/auth/register",
        creds("Chris", "correct horse"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{admin}");
    assert_eq!(admin["is_admin"], true);
    let set = h[header::SET_COOKIE].to_str().unwrap();
    assert!(
        set.contains("HttpOnly") && set.contains("SameSite=Lax"),
        "{set}"
    );
    let chris = cookie(&h);
    let (status, _, _) = send(&app, Method::GET, "/spools", None, Some(chris.clone())).await;
    assert_eq!(status, StatusCode::OK);
    let (_, _, st) = send(&app, Method::GET, "/auth/status", None, Some(chris.clone())).await;
    assert_eq!(st["setup_needed"], false);
    assert_eq!(st["user"]["username"], "Chris");

    // Registration is off until the admin turns it on.
    let (status, _, _) = send(
        &app,
        Method::POST,
        "/auth/register",
        creds("sam", "sam password"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, _) = send(
        &app,
        Method::PUT,
        "/admin/settings",
        Some(json!({ "registration_enabled": true })),
        Some(chris.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, h, sam) = send(
        &app,
        Method::POST,
        "/auth/register",
        creds("sam", "sam password"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{sam}");
    assert_eq!(sam["is_admin"], false);
    let sam_cookie = cookie(&h);
    let (status, _, _) = send(
        &app,
        Method::POST,
        "/auth/register",
        creds("SAM", "another one"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Only an admin manages accounts.
    let (status, _, _) = send(&app, Method::GET, "/users", None, Some(sam_cookie.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, users) = send(&app, Method::GET, "/users", None, Some(chris.clone())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(users.as_array().unwrap().len(), 2);

    // Signing in: wrong password, any-case username, then sign out.
    let (status, _, _) = send(
        &app,
        Method::POST,
        "/auth/login",
        creds("chris", "wrong password"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _, _) = send(
        &app,
        Method::POST,
        "/auth/login",
        creds("nobody", "correct horse"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, h, _) = send(
        &app,
        Method::POST,
        "/auth/login",
        creds("chris", "correct horse"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let second = cookie(&h);
    let (status, _, _) = send(
        &app,
        Method::POST,
        "/auth/logout",
        None,
        Some(second.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _, _) = send(&app, Method::GET, "/spools", None, Some(second)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // API tokens work as a Bearer header and can be revoked.
    let (status, _, tok) = send(
        &app,
        Method::POST,
        "/account/tokens",
        Some(json!({ "name": "OrcaSlicer" })),
        Some(chris.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{tok}");
    let secret = tok["secret"].as_str().unwrap().to_string();
    assert!(secret.starts_with("rc_"));
    let bearer = (header::AUTHORIZATION, format!("Bearer {secret}"));
    let (status, _, _) = send(&app, Method::GET, "/stats", None, Some(bearer.clone())).await;
    assert_eq!(status, StatusCode::OK);
    let (_, _, list) = send(
        &app,
        Method::GET,
        "/account/tokens",
        None,
        Some(chris.clone()),
    )
    .await;
    assert_eq!(list[0]["name"], "OrcaSlicer");
    assert!(list[0]["secret"].is_null());
    assert!(!list[0]["last_used_at"].is_null());
    let id = tok["id"].as_str().unwrap();
    let (status, _, _) = send(
        &app,
        Method::DELETE,
        &format!("/account/tokens/{id}"),
        None,
        Some(sam_cookie.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "someone else's token");
    let (status, _, _) = send(
        &app,
        Method::DELETE,
        &format!("/account/tokens/{id}"),
        None,
        Some(chris.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _, _) = send(&app, Method::GET, "/stats", None, Some(bearer)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Changing a password signs out other sessions, not this one.
    let (status, _, _) = send(
        &app,
        Method::PUT,
        "/account/password",
        Some(json!({ "current_password": "nope", "new_password": "new password" })),
        Some(sam_cookie.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (_, h, _) = send(
        &app,
        Method::POST,
        "/auth/login",
        creds("sam", "sam password"),
        None,
    )
    .await;
    let sam_other = cookie(&h);
    let (status, _, _) = send(
        &app,
        Method::PUT,
        "/account/password",
        Some(json!({ "current_password": "sam password", "new_password": "new password" })),
        Some(sam_cookie.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _, _) = send(&app, Method::GET, "/spools", None, Some(sam_cookie.clone())).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = send(&app, Method::GET, "/spools", None, Some(sam_other)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // There's always an admin, and admins can't remove themselves.
    let chris_id = admin["id"].as_str().unwrap();
    let sam_id = sam["id"].as_str().unwrap();
    let (status, _, _) = send(
        &app,
        Method::PUT,
        &format!("/users/{chris_id}"),
        Some(json!({ "is_admin": false })),
        Some(chris.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _, _) = send(
        &app,
        Method::DELETE,
        &format!("/users/{chris_id}"),
        None,
        Some(chris.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _, _) = send(
        &app,
        Method::PUT,
        &format!("/users/{sam_id}/password"),
        Some(json!({ "password": "reset by admin" })),
        Some(chris.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _, _) = send(&app, Method::GET, "/spools", None, Some(sam_cookie.clone())).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "a reset signs the account out"
    );
    let (status, _, _) = send(
        &app,
        Method::POST,
        "/users",
        creds("kid", "kid password"),
        Some(chris.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, _, _) = send(
        &app,
        Method::DELETE,
        &format!("/users/{sam_id}"),
        None,
        Some(chris.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, _, users) = send(&app, Method::GET, "/users", None, Some(chris)).await;
    assert_eq!(users.as_array().unwrap().len(), 2);
}

/// The API signed in as a new account (the first one made is the admin).
async fn as_user(pool: &PgPool, username: &str) -> (Router, String) {
    let hash = rollcall::auth::hash_password("test password")
        .await
        .unwrap();
    let user = rollcall::repo::accounts::create_user(pool, username, &hash, true)
        .await
        .unwrap();
    let secret = rollcall::auth::new_secret();
    rollcall::repo::accounts::create_token(
        pool,
        user.id,
        "tests",
        &rollcall::auth::hash_secret(&secret),
    )
    .await
    .unwrap();
    let bearer = HeaderValue::from_str(&format!("Bearer {secret}")).unwrap();
    let app = routes::api(state(pool.clone())).layer(axum::middleware::map_request(
        move |mut req: Request<Body>| {
            let bearer = bearer.clone();
            async move {
                req.headers_mut().insert(header::AUTHORIZATION, bearer);
                req
            }
        },
    ));
    (app, user.id.to_string())
}

#[sqlx::test]
async fn separate_inventories_and_shared_locations(pool: PgPool) {
    // Data from before accounts belongs to the first account.
    sqlx::query("INSERT INTO locations (name, kind) VALUES ('Old shelf', 'shelf')")
        .execute(&pool)
        .await
        .unwrap();
    let (alice, _) = as_user(&pool, "alice").await;
    let (bob, bob_id) = as_user(&pool, "bob").await;
    let (_, locs) = call(&alice, Method::GET, "/locations", None).await;
    assert_eq!(locs[0]["name"], "Old shelf");
    assert_eq!(locs[0]["access"], "owner");
    let (_, locs) = call(&bob, Method::GET, "/locations", None).await;
    assert_eq!(locs.as_array().unwrap().len(), 0);

    // Alice's spool in her location is invisible to Bob.
    let (_, shelf) = call(
        &alice,
        Method::POST,
        "/locations",
        Some(json!({ "name": "Shelf", "kind": "shelf" })),
    )
    .await;
    let shelf_id = shelf["id"].as_str().unwrap().to_string();
    let mut body = new_spool("Black", "000", 800.0);
    body["location_id"] = json!(shelf_id);
    body["label_code"] = json!("R-1");
    let (status, spool) = call(&alice, Method::POST, "/spools", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{spool}");
    assert_eq!(spool["access"], "owner");
    assert!(spool["owner_username"].is_null());
    let spool_uri = format!("/spools/{}", spool["id"].as_str().unwrap());
    let filament_id = spool["filament_id"].as_str().unwrap().to_string();

    let (_, list) = call(&bob, Method::GET, "/spools", None).await;
    assert_eq!(list.as_array().unwrap().len(), 0);
    let (status, _) = call(&bob, Method::GET, &spool_uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(
        &bob,
        Method::GET,
        &format!("/filaments/{filament_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, stats) = call(&bob, Method::GET, "/stats", None).await;
    assert_eq!(stats["spools_in_stock"], 0);

    // Bob's own copy of the same filament and label code is his own.
    let mut body = new_spool("Black", "000", 500.0);
    body["label_code"] = json!("R-1");
    let (status, bobs) = call(&bob, Method::POST, "/spools", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{bobs}");
    assert_ne!(bobs["filament_id"].as_str().unwrap(), filament_id);
    let (status, _) = call(
        &bob,
        Method::PUT,
        &format!("/filaments/{filament_id}"),
        Some(json!({ "brand_name": "x", "material_code": "PLA", "color_name": "x" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let mut body = new_spool("White", "fff", 500.0);
    body["location_id"] = json!(shelf_id);
    let (status, _) = call(&bob, Method::POST, "/spools", Some(body)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "not his location");

    // Shared to view: Bob sees the spool but can't change it.
    let shares = format!("/locations/{shelf_id}/shares");
    let (status, _) = call(
        &alice,
        Method::PUT,
        &shares,
        Some(json!({ "username": "nobody", "access": "view" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, list) = call(
        &alice,
        Method::PUT,
        &shares,
        Some(json!({ "username": "BOB", "access": "view" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{list}");
    assert_eq!(list[0]["username"], "bob");
    let (status, _) = call(&bob, Method::GET, &shares, None).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "only the owner sees the shares"
    );
    let (_, locs) = call(&bob, Method::GET, "/locations", None).await;
    assert_eq!(locs[0]["name"], "Shelf");
    assert_eq!(locs[0]["access"], "view");
    assert_eq!(locs[0]["owner_username"], "alice");
    let (_, locs) = call(&alice, Method::GET, "/locations", None).await;
    let shelf = locs
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["name"] == "Shelf")
        .unwrap();
    assert_eq!(shelf["share_count"], 1);

    let (status, seen) = call(&bob, Method::GET, &spool_uri, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(seen["access"], "view");
    assert_eq!(seen["owner_username"], "alice");
    let (_, list) = call(&bob, Method::GET, "/spools", None).await;
    assert_eq!(list.as_array().unwrap().len(), 2);
    let (_, stats) = call(&bob, Method::GET, "/stats", None).await;
    assert_eq!(stats["spools_in_stock"], 2);
    let (status, _) = call(
        &bob,
        Method::GET,
        &format!("/filaments/{filament_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        &bob,
        Method::GET,
        &format!("/filaments/{filament_id}/tuned"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let deduct = json!({ "kind": "quick_adjust", "delta_g": -10 });
    let (status, _) = call(
        &bob,
        Method::POST,
        &format!("{spool_uri}/weight"),
        Some(deduct.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = call(
        &bob,
        Method::POST,
        &format!("{spool_uri}/humidity"),
        Some(json!({ "humidity_pct": 30 })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = call(
        &bob,
        Method::GET,
        &format!("/photos?spool_id={}", spool["id"].as_str().unwrap()),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Shared to edit: Bob can use and add spools there; they stay Alice's.
    call(
        &alice,
        Method::PUT,
        &shares,
        Some(json!({ "username": "bob", "access": "edit" })),
    )
    .await;
    let (status, change) = call(
        &bob,
        Method::POST,
        &format!("{spool_uri}/weight"),
        Some(deduct),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{change}");
    assert_eq!(change["spool"]["remaining_weight_g"], 790.0);
    let mut body = new_spool("White", "fff", 500.0);
    body["location_id"] = json!(shelf_id);
    let (status, added) = call(&bob, Method::POST, "/spools", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{added}");
    assert_eq!(added["access"], "edit");
    assert_eq!(added["owner_username"], "alice");
    let (status, mine) = call(
        &alice,
        Method::GET,
        &format!("/spools/{}", added["id"].as_str().unwrap()),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(mine["access"], "owner");

    let (_, current) = call(&bob, Method::GET, &spool_uri, None).await;
    let mut edit = current.clone();
    edit["location_id"] = Value::Null;
    let (status, _) = call(&bob, Method::PUT, &spool_uri, Some(edit)).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "can't take it out of the share"
    );
    let mut edit = current.clone();
    let (_, bob_box) = call(
        &bob,
        Method::POST,
        "/locations",
        Some(json!({ "name": "Bob's box" })),
    )
    .await;
    edit["location_id"] = bob_box["id"].clone();
    edit["notes"] = json!("used by Bob");
    let (status, saved) = call(&bob, Method::PUT, &spool_uri, Some(edit)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{saved}");
    let mut edit = current.clone();
    edit["notes"] = json!("used by Bob");
    let (status, saved) = call(&bob, Method::PUT, &spool_uri, Some(edit)).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["notes"], "used by Bob");
    let (status, _) = call(&bob, Method::DELETE, &spool_uri, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Settings are per account.
    call(
        &bob,
        Method::PUT,
        "/settings",
        Some(json!({ "low_stock_threshold_g": 900, "default_currency": "EUR" })),
    )
    .await;
    let (_, a) = call(&alice, Method::GET, "/settings", None).await;
    assert_eq!(a["low_stock_threshold_g"], 200.0);
    assert_eq!(a["default_currency"], "USD");

    // Bob can leave; Alice can stop sharing.
    let (status, _) = call(&bob, Method::DELETE, &format!("{shares}/{bob_id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&bob, Method::GET, &spool_uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    call(
        &alice,
        Method::PUT,
        &shares,
        Some(json!({ "username": "bob", "access": "view" })),
    )
    .await;
    let (status, _) = call(&alice, Method::DELETE, &format!("{shares}/{bob_id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, list) = call(&bob, Method::GET, "/spools", None).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
}
