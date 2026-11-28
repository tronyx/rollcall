use crate::{AppState, config::Config};
use axum::{Json, Router, http::HeaderValue, middleware, routing::get};
use serde_json::json;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

mod auth;
mod equipment;
mod filaments;
mod humidity;
mod lookups;
mod photos;
mod prints;
mod settings;
mod spools;
mod stats;
mod tuned;

/// API routes only; used by `router` and by integration tests. Everything
/// but health and signing in needs a session cookie or an API token.
pub fn api(state: AppState) -> Router {
    let protected = Router::new()
        .merge(auth::routes())
        .merge(spools::routes())
        .merge(filaments::routes())
        .merge(lookups::routes())
        .merge(stats::routes())
        .merge(settings::routes())
        .merge(tuned::routes())
        .merge(humidity::routes())
        .merge(equipment::routes())
        .merge(photos::routes())
        .merge(prints::routes())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            crate::auth::require_user,
        ));
    Router::new()
        .route("/health", get(|| async { Json(json!({ "status": "ok" })) }))
        .merge(auth::public_routes())
        .merge(protected)
        .with_state(state)
}

pub fn router(state: AppState, config: &Config) -> Router {
    let mut app = Router::new().nest("/api", api(state));

    // Serve the built SvelteKit app, falling back to index.html for client routes.
    if let Some(dir) = &config.static_dir {
        let index = dir.join("index.html");
        app = app.fallback_service(ServeDir::new(dir).fallback(ServeFile::new(index)));
    }

    let cors = match config
        .cors_origin
        .as_deref()
        .and_then(|o| o.parse::<HeaderValue>().ok())
    {
        Some(origin) => CorsLayer::new()
            .allow_origin(origin)
            .allow_methods(Any)
            .allow_headers(Any),
        None => CorsLayer::new(),
    };

    app.layer(cors)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
}
