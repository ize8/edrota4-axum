use axum::{extract::State, http::StatusCode, response::IntoResponse};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::sync::Arc;

use crate::AppState;

pub struct MetricsState {
    pub handle: PrometheusHandle,
}

/// Set up the Prometheus metrics recorder
pub fn setup_metrics_recorder() -> MetricsState {
    let builder = PrometheusBuilder::new();

    // Configure histogram buckets for latency (in seconds)
    let builder = builder
        .set_buckets_for_metric(
            Matcher::Full("http_request_duration_seconds".to_string()),
            &[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0],
        )
        .expect("failed to set histogram buckets");

    let handle = builder
        .install_recorder()
        .expect("failed to install Prometheus recorder");

    MetricsState { handle }
}

/// Handler for the /metrics endpoint
pub async fn metrics_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Render metrics in Prometheus format
    let metrics = state.metrics.handle.render();
    (StatusCode::OK, metrics)
}
