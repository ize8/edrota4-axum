use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use subtle::ConstantTimeEq;

use crate::AppState;

/// Middleware that requires a valid X-Debug-Key header
pub async fn require_debug_key(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Get expected key from config
    let expected_key = state.config.debug_key.as_bytes();

    // Get header value
    let provided_key = request
        .headers()
        .get("X-Debug-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Constant-time comparison to prevent timing attacks
    if expected_key.ct_eq(provided_key.as_bytes()).into() {
        Ok(next.run(request).await)
    } else {
        tracing::warn!("Unauthorized debug endpoint access attempt");
        Err(StatusCode::UNAUTHORIZED)
    }
}
