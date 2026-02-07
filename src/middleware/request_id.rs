use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

/// Extension type for request ID
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

/// Middleware that generates a unique request ID for each request
pub async fn request_id_middleware(
    mut request: Request,
    next: Next,
) -> Response {
    let request_id = Uuid::new_v4().to_string();

    // Add to request extensions for handlers to access
    request.extensions_mut().insert(RequestId(request_id.clone()));

    // Add span field for correlation in logs
    tracing::Span::current().record("request_id", &request_id.as_str());

    let mut response = next.run(request).await;

    // Add to response header for client-side correlation
    response.headers_mut().insert(
        "X-Request-ID",
        request_id.parse().unwrap(),
    );

    response
}
