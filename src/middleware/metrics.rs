use axum::{extract::Request, middleware::Next, response::Response};
use metrics::{counter, histogram};
use std::time::Instant;

/// Middleware that collects HTTP request metrics
pub async fn metrics_middleware(request: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    // Extract route template (e.g., /api/users/:id -> /api/users/{id})
    // This is better for metrics cardinality than using raw paths
    let route = request
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| path.clone());

    let response = next.run(request).await;

    let status = response.status().as_u16().to_string();
    let duration = start.elapsed().as_secs_f64();

    // Record metrics with labels
    counter!(
        "http_requests_total",
        "method" => method.to_string(),
        "route" => route.clone(),
        "status" => status.clone()
    )
    .increment(1);

    histogram!(
        "http_request_duration_seconds",
        "route" => route,
        "method" => method.to_string()
    )
    .record(duration);

    response
}
