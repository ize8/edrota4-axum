use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
};
use moka::future::Cache;
use serde_json::json;
use std::future::Future;
use std::sync::Arc;

use crate::{auth, AppError, AppResult, AppState};

/// Extracts JWT token from either __session cookie (frontend) or Authorization header (testing)
fn extract_token_from_request(parts: &Parts) -> Option<String> {
    // Try __session cookie first (for TanStack frontend)
    if let Some(cookie_header) = parts.headers.get(header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            // Parse cookies manually (cookie = "name=value; name2=value2")
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                if let Some(value) = cookie.strip_prefix("__session=") {
                    return Some(value.to_string());
                }
            }
        }
    }

    // Fallback to Authorization header (for testing with Bearer tokens)
    if let Some(auth_header) = parts.headers.get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }

    None
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub clerk_user_id: String,
    pub email: String,
    pub profile_id: i32,
    pub is_super_admin: bool,
}

impl FromRequestParts<Arc<AppState>> for AuthenticatedUser {
    type Rejection = (StatusCode, axum::Json<serde_json::Value>);

    fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        // Try both cookie-based auth (for frontend) and Bearer token (for testing)
        let token = extract_token_from_request(parts);

        let state = state.clone();

        async move {
            // Extract token (from cookie or Authorization header)
            let token = token.ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    axum::Json(json!({"error": "Missing authentication: no __session cookie or Authorization header"})),
                )
            })?;

            // Validate JWT
            let expected_issuer = format!("https://{}", state.config.clerk_domain);
            let claims = auth::validate_jwt(&token, &state.jwks_cache, &expected_issuer)
                .await
                .map_err(|e| {
                    (
                        StatusCode::UNAUTHORIZED,
                        axum::Json(json!({"error": format!("JWT validation failed: {}", e)})),
                    )
                })?;

            let clerk_user_id = claims.sub.clone();

            // OPTIMIZATION: Try database lookup FIRST (99% of requests - fast!)
            // Only fetch email from Clerk API for auto-linking new users (1% of requests)
            let user_opt = sqlx::query_as::<_, crate::models::User>(
                r#"SELECT * FROM "Users" WHERE auth_id = $1"#,
            )
            .bind(&clerk_user_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, clerk_user_id, "Database query failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(json!({"error": "Database error"})),
                )
            })?;

            if let Some(user) = user_opt {
                // ✓ User found by auth_id - use email from database (FAST!)
                let email = user.primary_email.clone().unwrap_or_else(|| {
                    tracing::warn!(clerk_user_id, profile_id = user.user_profile_id, "User has no primary_email");
                    String::from("")
                });

                tracing::debug!(clerk_user_id, profile_id = user.user_profile_id, "User found by auth_id");
                return Ok(AuthenticatedUser {
                    clerk_user_id,
                    email,
                    profile_id: user.user_profile_id,
                    is_super_admin: user.is_super_admin,
                });
            }

            // User not found by auth_id - need email for auto-linking (rare case)
            tracing::debug!(clerk_user_id, "User not found by auth_id, attempting auto-link by email");

            let email = if let Some(email) = claims.email {
                email
            } else {
                resolve_email(&state.user_cache, &clerk_user_id, &state.config.clerk_secret_key)
                    .await
                    .map_err(|e| {
                        (
                            StatusCode::UNAUTHORIZED,
                            axum::Json(json!({"error": format!("Failed to resolve email: {}", e)})),
                        )
                    })?
            };

            // Auto-link user by email
            let user = sqlx::query_as::<_, crate::models::User>(
                r#"UPDATE "Users" SET auth_id = $1 WHERE LOWER(primary_email) = LOWER($2) RETURNING *"#,
            )
            .bind(&clerk_user_id)
            .bind(&email)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, clerk_user_id, email, "Auto-link query failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(json!({"error": "Database error"})),
                )
            })?
            .ok_or_else(|| {
                tracing::warn!(clerk_user_id, email, "User profile not found for auto-linking");
                (
                    StatusCode::UNAUTHORIZED,
                    axum::Json(json!({"error": format!("User profile not found for email: {}", email)})),
                )
            })?;

            tracing::info!(
                clerk_user_id,
                profile_id = user.user_profile_id,
                email,
                "User auto-linked by email"
            );

            let user_email = user.primary_email.clone().unwrap_or_else(|| email.clone());

            Ok(AuthenticatedUser {
                clerk_user_id,
                email: user_email,
                profile_id: user.user_profile_id,
                is_super_admin: user.is_super_admin,
            })
        }
    }
}

async fn resolve_email(
    cache: &Cache<String, String>,
    clerk_user_id: &str,
    clerk_secret_key: &str,
) -> AppResult<String> {
    // Check cache first
    if let Some(cached_email) = cache.get(clerk_user_id).await {
        tracing::debug!(clerk_user_id, "Email resolved from cache");
        return Ok(cached_email);
    }

    tracing::debug!(clerk_user_id, "Fetching email from Clerk API");

    // Make Clerk API request
    let url = format!("https://api.clerk.com/v1/users/{}", clerk_user_id);
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", clerk_secret_key))
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, clerk_user_id, "Clerk API request failed");
            AppError::Internal(format!("Clerk API request failed for user {}: {}", clerk_user_id, e))
        })?;

    if !response.status().is_success() {
        let status = response.status();
        tracing::error!(status = %status, clerk_user_id, "Clerk API returned error");
        return Err(AppError::Internal(format!(
            "Clerk API returned {} for user {}",
            status,
            clerk_user_id
        )));
    }

    let user_data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, clerk_user_id, "Failed to parse Clerk response");
            AppError::Internal(format!("Failed to parse Clerk response for user {}: {}", clerk_user_id, e))
        })?;

    let email_addresses = user_data
        .get("email_addresses")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            tracing::error!(clerk_user_id, "No email addresses in Clerk response");
            AppError::Internal(format!("No email addresses in Clerk response for user {}", clerk_user_id))
        })?;

    let primary_email = email_addresses
        .iter()
        .find(|e| e.get("id") == user_data.get("primary_email_address_id"))
        .or_else(|| email_addresses.first())
        .and_then(|e| e.get("email_address"))
        .and_then(|e| e.as_str())
        .ok_or_else(|| {
            tracing::error!(clerk_user_id, "No primary email found");
            AppError::Internal(format!("No primary email found for user {}", clerk_user_id))
        })?
        .to_string();

    // Cache the email for future requests (TTL is configured in cache creation)
    cache.insert(clerk_user_id.to_string(), primary_email.clone()).await;
    tracing::debug!(clerk_user_id, email = %primary_email, "Email cached for future requests");

    Ok(primary_email)
}

async fn resolve_user_profile(
    db: &sqlx::PgPool,
    clerk_user_id: &str,
    email: &str,
) -> AppResult<crate::models::User> {
    // Try to find by auth_id first
    let user = sqlx::query_as::<_, crate::models::User>(
        r#"SELECT * FROM "Users" WHERE auth_id = $1"#,
    )
    .bind(clerk_user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, clerk_user_id, "Database query failed for auth_id lookup");
        e
    })?;

    if let Some(user) = user {
        tracing::debug!(clerk_user_id, profile_id = user.user_profile_id, "User found by auth_id");
        return Ok(user);
    }

    tracing::debug!(clerk_user_id, email, "User not found by auth_id, attempting auto-link by email");

    // Fallback: auto-link by email match
    let user = sqlx::query_as::<_, crate::models::User>(
        r#"
        UPDATE "Users"
        SET auth_id = $1
        WHERE LOWER(primary_email) = LOWER($2)
        RETURNING *
        "#,
    )
    .bind(clerk_user_id)
    .bind(email)
    .fetch_optional(db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, clerk_user_id, email, "Database query failed for email auto-link");
        e
    })?;

    match user {
        Some(user) => {
            tracing::info!(
                clerk_user_id,
                profile_id = user.user_profile_id,
                email,
                "User auto-linked by email"
            );
            Ok(user)
        }
        None => {
            tracing::warn!(clerk_user_id, email, "User profile not found in database");
            Err(AppError::Unauthorized(format!(
                "User profile not found for email: {}",
                email
            )))
        }
    }
}
