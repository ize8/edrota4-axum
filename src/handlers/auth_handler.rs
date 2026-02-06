use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::{extractors::AuthenticatedUser, models::User, AppResult, AppState};

#[derive(Debug, Serialize)]
pub struct UserResponse {
    #[serde(flatten)]
    user: User,
}

/// GET /api/auth/me
#[utoipa::path(
    get,
    path = "/api/auth/me",
    responses(
        (status = 200, description = "Current authenticated user", body = User),
        (status = 401, description = "Unauthorized")
    ),
    tag = "auth",
    security(
        ("cookie_auth" = [])
    )
)]
pub async fn get_me(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser,
) -> AppResult<Json<User>> {
    let user = sqlx::query_as::<_, User>(r#"SELECT * FROM "Users" WHERE user_profile_id = $1"#)
        .bind(auth.profile_id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(user))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyPinRequest {
    pub user_profile_id: i32,
    pub pin: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VerifyPinResponse {
    pub valid: bool,
}

/// POST /api/auth/verify-pin
#[utoipa::path(
    post,
    path = "/api/auth/verify-pin",
    request_body = VerifyPinRequest,
    responses(
        (status = 200, description = "PIN verification result", body = VerifyPinResponse),
        (status = 401, description = "Unauthorized")
    ),
    tag = "auth"
)]
pub async fn verify_pin(
    State(state): State<Arc<AppState>>,
    _auth: AuthenticatedUser,
    Json(payload): Json<VerifyPinRequest>,
) -> AppResult<Json<VerifyPinResponse>> {
    let user = sqlx::query_as::<_, User>(
        r#"SELECT * FROM "Users" WHERE user_profile_id = $1"#,
    )
    .bind(payload.user_profile_id)
    .fetch_optional(&state.db)
    .await?;

    let valid = match user {
        Some(user) => user.auth_pin.as_deref() == Some(&payload.pin),
        None => false,
    };

    Ok(Json(VerifyPinResponse { valid }))
}
