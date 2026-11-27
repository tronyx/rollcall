use anyhow::Context;
use rollcall::{AppState, config::Config, db, routes, scraper::ScraperService};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rollcall=info,tower_http=info".into()),
        )
        .init();

    let config = Config::from_env()?;
    let pool = db::connect(&config.database_url).await?;
    db::migrate(&pool).await?;

    tokio::fs::create_dir_all(&config.upload_dir)
        .await
        .with_context(|| format!("creating UPLOAD_DIR {}", config.upload_dir.display()))?;
    match rollcall::repo::photos::remove_orphan_files(&pool, &config.upload_dir).await {
        Ok(0) => {}
        Ok(n) => {
            tracing::info!("removed {n} photo files whose spool, filament or equipment was deleted")
        }
        Err(e) => tracing::warn!("could not clean up photo files: {e}"),
    }

    let state = AppState {
        pool,
        scraper: Arc::new(ScraperService::new(&config.scraper, config.llm.clone())?),
        uploads: Arc::new(config.upload_dir.clone()),
    };
    let app = routes::router(state, &config);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .with_context(|| format!("binding {}", config.bind_addr))?;
    tracing::info!("Rollcall listening on http://{}", config.bind_addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}
