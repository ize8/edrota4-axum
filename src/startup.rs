use axum::{
    http::{header, HeaderValue, Method},
    response::Html,
    routing::{delete, get, post, put},
    Json, Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;

use crate::{handlers, openapi::ApiDoc};

pub fn build_router(state: Arc<crate::AppState>) -> Router {
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT])
        .allow_credentials(true);

    // Auth routes
    let auth_routes = Router::new()
        .route("/me", get(handlers::auth_handler::get_me))
        .route("/verify-pin", post(handlers::auth_handler::verify_pin));

    // Reference routes
    let reference_routes = Router::new().route(
        "/time-off-categories",
        get(handlers::references_handler::get_time_off_categories),
    );

    // Role routes
    let role_routes = Router::new()
        .route("/", get(handlers::roles_handler::get_roles))
        .route("/", post(handlers::roles_handler::create_role))
        .route("/{id}", put(handlers::roles_handler::update_role))
        .route("/{id}", delete(handlers::roles_handler::delete_role));

    // Workplace routes
    let workplace_routes = Router::new()
        .route("/", get(handlers::workplaces_handler::get_workplaces))
        .route("/", post(handlers::workplaces_handler::create_workplace))
        .route("/{id}", put(handlers::workplaces_handler::update_workplace))
        .route("/{id}", delete(handlers::workplaces_handler::delete_workplace));

    // User Role routes
    let user_role_routes = Router::new()
        .route("/", get(handlers::user_roles_handler::get_user_roles))
        .route("/", post(handlers::user_roles_handler::create_user_role))
        .route("/{id}", put(handlers::user_roles_handler::update_user_role))
        .route("/{id}", delete(handlers::user_roles_handler::delete_user_role));

    // User routes
    let user_routes = Router::new()
        .route("/", get(handlers::users_handler::get_users))
        .route("/me", put(handlers::users_handler::update_own_profile))
        .route("/me/pin", post(handlers::users_handler::change_own_pin))
        .route("/substantive", get(handlers::users_handler::get_substantive_users))
        .route("/staff-list", get(handlers::users_handler::get_staff_list))
        // New Phase B endpoints - must come before /{id} to prevent route shadowing
        .route("/search", post(handlers::users_handler::search_users))
        .route("/profiles", post(handlers::users_handler::create_user_profile))
        .route("/check-email", post(handlers::users_handler::check_email_usage))
        .route("/verify-identity", post(handlers::users_handler::verify_profile_identity))
        .route("/change-profile-pin", post(handlers::users_handler::change_profile_pin))
        // Existing routes
        .route("/profiles/{id}", put(handlers::users_handler::update_user_profile))
        .route("/{id}/reset-pin", post(handlers::users_handler::reset_user_pin))
        .route("/{id}", get(handlers::users_handler::get_user));

    // Shift routes
    let shift_routes = Router::new()
        .route("/", get(handlers::shifts_handler::get_shifts_for_month))
        .route("/", post(handlers::shifts_handler::create_shift))
        .route("/by-date", get(handlers::shifts_handler::get_shifts_for_date))
        .route("/range", get(handlers::shifts_handler::get_shifts_for_range))
        .route("/{uuid}", put(handlers::shifts_handler::update_shift))
        .route("/{uuid}", delete(handlers::shifts_handler::delete_shift));

    // Template routes
    let template_routes = Router::new()
        .route("/", get(handlers::templates_handler::get_templates))
        .route("/", post(handlers::templates_handler::create_template))
        .route("/{id}", put(handlers::templates_handler::update_template))
        .route("/{id}", delete(handlers::templates_handler::delete_template));

    // Diary routes
    let diary_routes = Router::new()
        .route("/", get(handlers::diary_handler::get_diary))
        .route("/", post(handlers::diary_handler::create_diary_entry))
        .route("/{id}", delete(handlers::diary_handler::delete_diary_entry));

    // Comments routes
    let comments_routes = Router::new().route("/", get(handlers::comments_handler::get_comments));

    // Audit routes
    let audit_routes = Router::new().route("/", get(handlers::audit_handler::get_audit));

    // Job Plans routes
    let job_plans_routes = Router::new()
        .route("/", get(handlers::job_plans_handler::get_job_plans))
        .route("/", post(handlers::job_plans_handler::create_job_plan))
        .route("/{id}", put(handlers::job_plans_handler::update_job_plan))
        .route("/{id}", delete(handlers::job_plans_handler::delete_job_plan))
        .route("/{id}/terminate", post(handlers::job_plans_handler::terminate_job_plan));

    // Marketplace routes
    let marketplace_routes = Router::new()
        .route("/open", get(handlers::marketplace_handler::get_open_requests))
        .route("/my", get(handlers::marketplace_handler::get_my_requests))
        .route("/incoming", get(handlers::marketplace_handler::get_incoming_requests))
        .route("/approvals", get(handlers::marketplace_handler::get_approval_requests))
        .route("/dashboard", get(handlers::marketplace_handler::get_dashboard))
        .route("/swappable", get(handlers::marketplace_handler::get_swappable_shifts))
        .route("/requests", post(handlers::marketplace_handler::create_shift_request))
        .route("/requests/{id}/accept", post(handlers::marketplace_handler::accept_shift_request))
        .route("/requests/{id}/respond", post(handlers::marketplace_handler::respond_to_proposal))
        .route("/requests/{id}/admin-decision", post(handlers::marketplace_handler::admin_decision))
        .route("/requests/{id}", delete(handlers::marketplace_handler::cancel_shift_request));

    Router::new()
        .route("/health", get(handlers::health_check))
        .nest("/api/auth", auth_routes)
        .nest("/api/references", reference_routes)
        .nest("/api/roles", role_routes)
        .nest("/api/workplaces", workplace_routes)
        .nest("/api/user-roles", user_role_routes)
        .nest("/api/users", user_routes)
        .nest("/api/shifts", shift_routes)
        .nest("/api/templates", template_routes)
        .nest("/api/diary", diary_routes)
        .nest("/api/comments", comments_routes)
        .nest("/api/audit", audit_routes)
        .nest("/api/job-plans", job_plans_routes)
        .nest("/api/marketplace", marketplace_routes)
        .route("/api-docs/openapi.json", get(|| async { Json(ApiDoc::openapi()) }))
        .route("/swagger-ui", get(swagger_ui))
        .layer(cors)
        .with_state(state)
}

async fn swagger_ui() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>EDrota API Documentation</title>
    <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-standalone-preset.js"></script>
    <script>
        window.onload = () => {
            window.ui = SwaggerUIBundle({
                url: '/api-docs/openapi.json',
                dom_id: '#swagger-ui',
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIStandalonePreset
                ],
                layout: "StandaloneLayout"
            });
        };
    </script>
</body>
</html>
    "#)
}
