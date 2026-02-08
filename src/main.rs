mod auth;
mod config;
mod db;
mod error;
mod extractors;
mod handlers;
mod middleware;
mod models;
mod openapi;
mod startup;

use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub use auth::JwksCache;
pub use config::AppConfig;
pub use error::{AppError, AppResult};
pub use handlers::MetricsState;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwks_cache: Arc<JwksCache>,
    pub user_cache: Cache<String, String>, // clerk_user_id → email
    pub profile_cache: Cache<String, (i32, bool, String)>, // clerk_user_id → (profile_id, is_super_admin, email)
    pub config: AppConfig,
    pub metrics: Arc<MetricsState>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with conditional JSON/text output
    let use_json = std::env::var("LOG_FORMAT")
        .unwrap_or_else(|_| "text".to_string()) == "json";

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,edrota4_axum=debug,tower_http=debug".into());

    if use_json {
        // Structured JSON logging for production
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        // Human-readable for development
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    // Load environment variables
    dotenvy::dotenv().ok();

    // Load configuration
    let config = AppConfig::from_env().map_err(|e| {
        tracing::error!("❌ Configuration error: {}", e);
        e
    })?;

    // Create database pool
    let db = db::create_pool(&config.database_url).await.map_err(|e| {
        tracing::error!("❌ Failed to create database pool: {}", e);
        e
    })?;

    tracing::info!("✅ Database pool created successfully");

    // Initialize metrics recorder
    let metrics_state = Arc::new(handlers::setup_metrics_recorder());
    tracing::info!("✅ Metrics recorder initialized");

    // Create JWKS cache
    let jwks_cache = Arc::new(JwksCache::new(&config.clerk_domain));

    // Create user cache (clerk_user_id → email) with 5-minute TTL
    let user_cache = Cache::builder()
        .time_to_live(Duration::from_secs(300))
        .max_capacity(10_000)
        .build();

    // Create profile cache (clerk_user_id → profile data) with 60-second TTL
    let profile_cache = Cache::builder()
        .time_to_live(Duration::from_secs(60))
        .max_capacity(10_000)
        .build();

    // Create application state
    let state = Arc::new(AppState {
        db,
        jwks_cache,
        user_cache,
        profile_cache,
        config,
        metrics: metrics_state,
    });

    // Build router
    let app = startup::build_router(state);

    // Start server
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    tracing::info!("🚀 Server listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
