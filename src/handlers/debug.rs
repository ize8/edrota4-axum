use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;
use std::time::SystemTime;

use crate::AppState;

#[derive(Serialize)]
pub struct DebugInfo {
    pub version: String,
    pub git_sha: String,
    pub environment: String,
    pub uptime_seconds: u64,
    pub database_status: String,
    pub database_connections: u32,
    pub timestamp: u64,
}

/// Global start time for uptime calculation
static START_TIME: once_cell::sync::Lazy<SystemTime> =
    once_cell::sync::Lazy::new(SystemTime::now);

/// Handler for the /debug endpoint
pub async fn debug_handler(State(state): State<Arc<AppState>>) -> Json<DebugInfo> {
    // Check DB connectivity
    let db_status = match sqlx::query("SELECT 1").fetch_one(&state.db).await {
        Ok(_) => "connected".to_string(),
        Err(e) => format!("error: {}", e),
    };

    // Get pool stats
    let pool_size = state.db.size();

    // Calculate uptime
    let uptime = START_TIME.elapsed().unwrap_or_default().as_secs();

    let info = DebugInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        git_sha: option_env!("GIT_SHA").unwrap_or("unknown").to_string(),
        environment: std::env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string()),
        uptime_seconds: uptime,
        database_status: db_status,
        database_connections: pool_size,
        timestamp: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    Json(info)
}
