use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::{
    extractors::AuthenticatedUser,
    models::{
        ChangeOwnPinInput, PinResponse, StaffFilterOption, UpdateOwnProfileInput,
        UpdateUserProfileInput, User,
    },
    AppError, AppResult, AppState,
};

// ToSchema is used in auth_handler for VerifyPinRequest/Response

/// GET /api/users
#[utoipa::path(
    get,
    path = "/api/users",
    responses(
        (status = 200, description = "List of all users", body = Vec<User>)
    ),
    tag = "users"
)]
pub async fn get_users(State(state): State<Arc<AppState>>) -> AppResult<Json<Vec<User>>> {
    let users = sqlx::query_as::<_, User>(
        r#"
        SELECT * FROM "Users"
        ORDER BY short_name
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(users))
}

/// GET /api/users/{id}
#[utoipa::path(
    get,
    path = "/api/users/{id}",
    params(
        ("id" = i32, Path, description = "User profile ID")
    ),
    responses(
        (status = 200, description = "User found", body = User),
        (status = 404, description = "User not found")
    ),
    tag = "users"
)]
pub async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> AppResult<Json<User>> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT * FROM "Users"
        WHERE user_profile_id = $1
        "#,
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(user))
}

/// GET /api/users/substantive
#[utoipa::path(
    get,
    path = "/api/users/substantive",
    responses(
        (status = 200, description = "List of substantive (non-generic) users", body = Vec<User>)
    ),
    tag = "users"
)]
pub async fn get_substantive_users(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Vec<User>>> {
    let users = sqlx::query_as::<_, User>(
        r#"
        SELECT * FROM "Users"
        WHERE is_generic_login = false
        ORDER BY short_name
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(users))
}

/// GET /api/users/staff-list
#[utoipa::path(
    get,
    path = "/api/users/staff-list",
    responses(
        (status = 200, description = "Staff list for filters", body = Vec<StaffFilterOption>)
    ),
    tag = "users"
)]
pub async fn get_staff_list(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Vec<StaffFilterOption>>> {
    let staff = sqlx::query_as::<_, StaffFilterOption>(
        r#"
        SELECT
            user_profile_id,
            short_name,
            full_name,
            color
        FROM "Users"
        WHERE is_generic_login = false
        ORDER BY short_name
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(staff))
}

/// PUT /api/users/me - Update own profile (self-service)
#[utoipa::path(
    put,
    path = "/api/users/me",
    request_body = UpdateOwnProfileInput,
    responses(
        (status = 200, description = "Profile updated", body = User),
        (status = 403, description = "Generic accounts cannot self-update")
    ),
    tag = "users",
    security(("cookie_auth" = []))
)]
pub async fn update_own_profile(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser,
    Json(input): Json<UpdateOwnProfileInput>,
) -> AppResult<Json<User>> {
    // Block generic accounts from self-service updates
    let user = sqlx::query_as::<_, User>(r#"SELECT * FROM "Users" WHERE user_profile_id = $1"#)
        .bind(auth.profile_id)
        .fetch_one(&state.db)
        .await?;

    if user.is_generic_login {
        return Err(AppError::Forbidden(
            "Generic accounts cannot update their profile".to_string(),
        ));
    }

    // Validate color format if provided
    if let Some(ref color) = input.color {
        if !color.starts_with('#') || color.len() != 7 {
            return Err(AppError::BadRequest(
                "Color must be a valid hex color (#RRGGBB)".to_string(),
            ));
        }
    }

    // Update allowed fields only
    let updated_user = sqlx::query_as::<_, User>(
        r#"
        UPDATE "Users"
        SET short_name = $1, tel = $2, color = $3
        WHERE user_profile_id = $4
        RETURNING *
        "#,
    )
    .bind(&input.short_name)
    .bind(&input.tel)
    .bind(&input.color)
    .bind(auth.profile_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(updated_user))
}

/// POST /api/users/me/pin - Change own PIN (self-service)
#[utoipa::path(
    post,
    path = "/api/users/me/pin",
    request_body = ChangeOwnPinInput,
    responses(
        (status = 200, description = "PIN changed successfully", body = PinResponse),
        (status = 400, description = "Invalid PIN format or PINs don't match"),
        (status = 401, description = "Current PIN incorrect")
    ),
    tag = "users",
    security(("cookie_auth" = []))
)]
pub async fn change_own_pin(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser,
    Json(input): Json<ChangeOwnPinInput>,
) -> AppResult<Json<PinResponse>> {
    // Validate PINs match
    if input.new_pin != input.confirm_new_pin {
        return Err(AppError::BadRequest("New PINs do not match".to_string()));
    }

    // Validate PIN format (5 digits)
    if input.new_pin.len() != 5 || !input.new_pin.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::BadRequest(
            "PIN must be exactly 5 digits".to_string(),
        ));
    }

    // Get user to check generic account status and current PIN
    let user = sqlx::query_as::<_, User>(r#"SELECT * FROM "Users" WHERE user_profile_id = $1"#)
        .bind(auth.profile_id)
        .fetch_one(&state.db)
        .await?;

    if user.is_generic_login {
        return Err(AppError::Forbidden(
            "Generic accounts cannot change their PIN".to_string(),
        ));
    }

    // Verify current PIN (allow NULL for first-time setup)
    if let Some(ref current_pin) = user.auth_pin {
        if current_pin != &input.current_pin {
            return Err(AppError::BadRequest(
                "Current PIN is incorrect".to_string(),
            ));
        }

        // Prevent setting same PIN
        if current_pin == &input.new_pin {
            return Err(AppError::BadRequest(
                "New PIN must be different from current PIN".to_string(),
            ));
        }
    }

    // Update PIN
    sqlx::query(r#"UPDATE "Users" SET auth_pin = $1 WHERE user_profile_id = $2"#)
        .bind(&input.new_pin)
        .bind(auth.profile_id)
        .execute(&state.db)
        .await?;

    Ok(Json(PinResponse {
        success: true,
        new_pin: None,
        message: Some("PIN changed successfully".to_string()),
    }))
}

/// PUT /api/users/profiles/{id} - Update user profile (admin)
#[utoipa::path(
    put,
    path = "/api/users/profiles/{id}",
    params(
        ("id" = i32, Path, description = "User profile ID")
    ),
    request_body = UpdateUserProfileInput,
    responses(
        (status = 200, description = "User profile updated", body = User),
        (status = 403, description = "Missing can_edit_staff permission"),
        (status = 404, description = "User not found")
    ),
    tag = "users",
    security(("cookie_auth" = []))
)]
pub async fn update_user_profile(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i32>,
    auth: AuthenticatedUser,
    Json(input): Json<UpdateUserProfileInput>,
) -> AppResult<Json<User>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_staff").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_staff permission".to_string(),
        ));
    }

    // Validate PIN format if provided
    if let Some(ref pin) = input.auth_pin {
        if pin.len() != 5 || !pin.chars().all(|c| c.is_ascii_digit()) {
            return Err(AppError::BadRequest(
                "PIN must be exactly 5 digits".to_string(),
            ));
        }
    }

    // Validate color format if provided
    if let Some(ref color) = input.color {
        if !color.starts_with('#') || color.len() != 7 {
            return Err(AppError::BadRequest(
                "Color must be a valid hex color (#RRGGBB)".to_string(),
            ));
        }
    }

    // Build dynamic UPDATE query
    let mut updates = vec![];
    let mut bind_count = 1;

    if input.full_name.is_some() {
        updates.push(format!("full_name = ${}", bind_count));
        bind_count += 1;
    }
    if input.short_name.is_some() {
        updates.push(format!("short_name = ${}", bind_count));
        bind_count += 1;
    }
    if input.gmc.is_some() {
        updates.push(format!("gmc = ${}", bind_count));
        bind_count += 1;
    }
    if input.primary_email.is_some() {
        updates.push(format!("primary_email = ${}", bind_count));
        bind_count += 1;
    }
    if input.secondary_emails.is_some() {
        updates.push(format!("secondary_emails = ${}", bind_count));
        bind_count += 1;
    }
    if input.tel.is_some() {
        updates.push(format!("tel = ${}", bind_count));
        bind_count += 1;
    }
    if input.comment.is_some() {
        updates.push(format!("comment = ${}", bind_count));
        bind_count += 1;
    }
    if input.auth_pin.is_some() {
        updates.push(format!("auth_pin = ${}", bind_count));
        bind_count += 1;
    }
    if input.color.is_some() {
        updates.push(format!("color = ${}", bind_count));
        bind_count += 1;
    }

    if updates.is_empty() {
        return Err(AppError::BadRequest("No fields to update".to_string()));
    }

    let sql = format!(
        r#"UPDATE "Users" SET {} WHERE user_profile_id = ${} RETURNING *"#,
        updates.join(", "),
        bind_count
    );

    // Build query with bindings
    let mut query = sqlx::query_as::<_, User>(&sql);

    if let Some(full_name) = &input.full_name {
        query = query.bind(full_name);
    }
    if let Some(short_name) = &input.short_name {
        query = query.bind(short_name);
    }
    if let Some(gmc) = input.gmc {
        query = query.bind(gmc);
    }
    if let Some(primary_email) = &input.primary_email {
        query = query.bind(primary_email);
    }
    if let Some(secondary_emails) = &input.secondary_emails {
        query = query.bind(secondary_emails);
    }
    if let Some(tel) = &input.tel {
        query = query.bind(tel);
    }
    if let Some(comment) = &input.comment {
        query = query.bind(comment);
    }
    if let Some(auth_pin) = &input.auth_pin {
        query = query.bind(auth_pin);
    }
    if let Some(color) = &input.color {
        query = query.bind(color);
    }

    query = query.bind(user_id);

    let updated_user = query.fetch_one(&state.db).await?;

    Ok(Json(updated_user))
}

/// POST /api/users/{id}/reset-pin - Reset user PIN (admin)
#[utoipa::path(
    post,
    path = "/api/users/{id}/reset-pin",
    params(
        ("id" = i32, Path, description = "User profile ID")
    ),
    responses(
        (status = 200, description = "PIN reset successfully, new PIN returned", body = PinResponse),
        (status = 403, description = "Missing can_edit_staff permission"),
        (status = 404, description = "User not found")
    ),
    tag = "users",
    security(("cookie_auth" = []))
)]
pub async fn reset_user_pin(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i32>,
    auth: AuthenticatedUser,
) -> AppResult<Json<PinResponse>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_staff").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_staff permission".to_string(),
        ));
    }

    // Generate new random 5-digit PIN
    use rand::{Rng, SeedableRng};
    let mut rng = rand::rngs::StdRng::from_entropy();
    let new_pin = format!("{:05}", rng.gen_range(0..100000));

    // Update PIN
    sqlx::query(r#"UPDATE "Users" SET auth_pin = $1 WHERE user_profile_id = $2"#)
        .bind(&new_pin)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(PinResponse {
        success: true,
        new_pin: Some(new_pin),
        message: Some("PIN reset successfully".to_string()),
    }))
}

// Note: has_permission is now centralized in crate::extractors::permissions::has_permission_by_name
