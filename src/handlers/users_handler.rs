use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::{
    auth::{check_email_in_clerk, generate_pin_token, validate_pin_token},
    extractors::AuthenticatedUser,
    models::{
        ChangeOwnPinInput, ChangeProfilePinRequest, CheckEmailRequest, CheckEmailResponse,
        CreateUserProfileRequest, PinResponse, SearchUsersRequest, StaffFilterOption,
        SuccessResponse, UpdateOwnProfileInput, UpdateUserProfileInput, User,
        VerifyIdentityRequest, VerifyIdentityResponse,
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

// ============================================================================
// New Endpoints - Phase B
// ============================================================================

/// POST /api/users/search - Search users by name or email
#[utoipa::path(
    post,
    path = "/api/users/search",
    request_body = SearchUsersRequest,
    responses(
        (status = 200, description = "List of matching users", body = Vec<User>),
        (status = 400, description = "Invalid search query")
    ),
    tag = "users",
    security(("cookie_auth" = []))
)]
pub async fn search_users(
    State(state): State<Arc<AppState>>,
    _auth: AuthenticatedUser, // Require authentication
    Json(req): Json<SearchUsersRequest>,
) -> AppResult<Json<Vec<User>>> {
    // Validate query is not empty
    if req.query.trim().is_empty() {
        return Err(AppError::BadRequest("Search query cannot be empty".to_string()));
    }

    let search_pattern = format!("%{}%", req.query);

    let users = if let Some(role_id) = req.role_id {
        // Search with role filter
        sqlx::query_as::<_, User>(
            r#"
            SELECT DISTINCT u.* FROM "Users" u
            INNER JOIN "UserRoles" ur ON u.user_profile_id = ur.user_profile_id
            WHERE ur.role_id = $2
              AND (u.full_name ILIKE $1
                   OR u.short_name ILIKE $1
                   OR u.primary_email ILIKE $1
                   OR EXISTS (SELECT 1 FROM unnest(u.secondary_emails) e WHERE e ILIKE $1))
            ORDER BY u.full_name
            LIMIT 50
            "#,
        )
        .bind(&search_pattern)
        .bind(role_id)
        .fetch_all(&state.db)
        .await?
    } else {
        // Search without role filter
        sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM "Users"
            WHERE full_name ILIKE $1
               OR short_name ILIKE $1
               OR primary_email ILIKE $1
               OR EXISTS (SELECT 1 FROM unnest(secondary_emails) e WHERE e ILIKE $1)
            ORDER BY full_name
            LIMIT 50
            "#,
        )
        .bind(&search_pattern)
        .fetch_all(&state.db)
        .await?
    };

    tracing::info!(
        query = %req.query,
        role_id = ?req.role_id,
        results_count = users.len(),
        "User search completed"
    );

    Ok(Json(users))
}

/// POST /api/users/profiles - Create user profile without Clerk account
#[utoipa::path(
    post,
    path = "/api/users/profiles",
    request_body = CreateUserProfileRequest,
    responses(
        (status = 200, description = "User profile created successfully", body = User),
        (status = 400, description = "Invalid input data"),
        (status = 403, description = "Missing can_edit_staff permission")
    ),
    tag = "users",
    security(("cookie_auth" = []))
)]
pub async fn create_user_profile(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser,
    Json(req): Json<CreateUserProfileRequest>,
) -> AppResult<Json<User>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(
        &state.db,
        auth.profile_id,
        auth.is_super_admin,
        "can_edit_staff",
    )
    .await?
    {
        return Err(AppError::Forbidden(
            "Missing can_edit_staff permission".to_string(),
        ));
    }

    // Validate PIN format if provided
    if let Some(ref pin) = req.auth_pin {
        if pin.len() != 5 || !pin.chars().all(|c| c.is_ascii_digit()) {
            return Err(AppError::BadRequest(
                "PIN must be exactly 5 digits".to_string(),
            ));
        }
    }

    // Validate color format if provided
    if let Some(ref color) = req.color {
        if !color.starts_with('#') || color.len() != 7 {
            return Err(AppError::BadRequest(
                "Color must be a valid hex color (#RRGGBB)".to_string(),
            ));
        }
    }

    // Generate temporary auth_id using UUID
    let temp_auth_id = format!("temp_{}", uuid::Uuid::new_v4());

    // Insert user profile
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO "Users" (
            auth_id, full_name, short_name, gmc, primary_email,
            secondary_emails, tel, comment, auth_pin, color, is_generic_login
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, false)
        RETURNING *
        "#,
    )
    .bind(&temp_auth_id)
    .bind(&req.full_name)
    .bind(&req.short_name)
    .bind(req.gmc)
    .bind(&req.primary_email)
    .bind(&req.secondary_emails)
    .bind(&req.tel)
    .bind(&req.comment)
    .bind(&req.auth_pin)
    .bind(&req.color)
    .fetch_one(&state.db)
    .await?;

    tracing::info!(
        user_profile_id = user.user_profile_id,
        full_name = %req.full_name,
        created_by = auth.profile_id,
        "User profile created without Clerk account"
    );

    Ok(Json(user))
}

/// POST /api/users/check-email - Check if email exists in Clerk or database
#[utoipa::path(
    post,
    path = "/api/users/check-email",
    request_body = CheckEmailRequest,
    responses(
        (status = 200, description = "Email availability check result", body = CheckEmailResponse),
        (status = 403, description = "Missing can_edit_staff permission")
    ),
    tag = "users",
    security(("cookie_auth" = []))
)]
pub async fn check_email_usage(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser,
    Json(req): Json<CheckEmailRequest>,
) -> AppResult<Json<CheckEmailResponse>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(
        &state.db,
        auth.profile_id,
        auth.is_super_admin,
        "can_edit_staff",
    )
    .await?
    {
        return Err(AppError::Forbidden(
            "Missing can_edit_staff permission".to_string(),
        ));
    }

    // Check database for email
    let db_result = sqlx::query_scalar::<_, Option<i32>>(
        r#"
        SELECT user_profile_id
        FROM "Users"
        WHERE LOWER(primary_email) = LOWER($1)
           OR $1 = ANY(secondary_emails)
        LIMIT 1
        "#,
    )
    .bind(&req.email)
    .fetch_optional(&state.db)
    .await?;

    let used_by_profile = db_result.is_some();
    let user_id = db_result.flatten();

    // Check Clerk for email
    let used_for_login = check_email_in_clerk(&req.email, &state.config.clerk_secret_key).await?;

    tracing::info!(
        email = %req.email,
        used_for_login,
        used_by_profile,
        "Email availability check completed"
    );

    Ok(Json(CheckEmailResponse {
        used_for_login,
        used_by_profile,
        user_id,
    }))
}

/// POST /api/users/verify-identity - Verify PIN and issue token (Step 1 of PIN change)
#[utoipa::path(
    post,
    path = "/api/users/verify-identity",
    request_body = VerifyIdentityRequest,
    responses(
        (status = 200, description = "Identity verified, token issued", body = VerifyIdentityResponse),
        (status = 400, description = "Invalid PIN format or no PIN set"),
        (status = 401, description = "Incorrect PIN"),
        (status = 403, description = "Only generic accounts can use this endpoint"),
        (status = 404, description = "User not found")
    ),
    tag = "users",
    security(("cookie_auth" = []))
)]
pub async fn verify_profile_identity(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser,
    Json(req): Json<VerifyIdentityRequest>,
) -> AppResult<Json<VerifyIdentityResponse>> {
    // Verify this is a generic account
    let current_user = sqlx::query_as::<_, User>(
        r#"SELECT * FROM "Users" WHERE user_profile_id = $1"#,
    )
    .bind(auth.profile_id)
    .fetch_one(&state.db)
    .await?;

    if !current_user.is_generic_login {
        return Err(AppError::Forbidden(
            "This function is only for generic account users".to_string(),
        ));
    }

    // Validate PIN format
    if req.pin.len() != 5 || !req.pin.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::BadRequest("PIN must be 5 digits".to_string()));
    }

    // Fetch target user and their PIN
    let target_user = sqlx::query_as::<_, User>(
        r#"SELECT * FROM "Users" WHERE user_profile_id = $1"#,
    )
    .bind(req.user_profile_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("User profile not found".to_string()))?;

    // Check if user has a PIN set
    let stored_pin = target_user
        .auth_pin
        .ok_or_else(|| AppError::BadRequest("No PIN set for this user. Contact administrator.".to_string()))?;

    // Verify PIN matches (plain text comparison)
    if req.pin != stored_pin {
        tracing::warn!(
            user_profile_id = req.user_profile_id,
            attempted_by = auth.profile_id,
            "Incorrect PIN attempt"
        );
        return Err(AppError::Unauthorized(
            "Incorrect PIN for selected user".to_string(),
        ));
    }

    // Generate verification token (valid for 5 minutes)
    let token = generate_pin_token(req.user_profile_id, &state.config.pin_token_secret)?;

    tracing::info!(
        user_profile_id = req.user_profile_id,
        verified_by = auth.profile_id,
        "Identity verified, token issued"
    );

    Ok(Json(VerifyIdentityResponse {
        success: true,
        token: Some(token),
    }))
}

/// POST /api/users/change-profile-pin - Change PIN using verification token (Step 2)
#[utoipa::path(
    post,
    path = "/api/users/change-profile-pin",
    request_body = ChangeProfilePinRequest,
    responses(
        (status = 200, description = "PIN changed successfully", body = SuccessResponse),
        (status = 400, description = "Invalid input or token expired"),
        (status = 401, description = "Invalid verification token")
    ),
    tag = "users"
)]
pub async fn change_profile_pin(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChangeProfilePinRequest>,
) -> AppResult<Json<SuccessResponse>> {
    // Validate new PIN matches confirmation
    if req.new_pin != req.confirm_pin {
        return Err(AppError::BadRequest("New PINs do not match".to_string()));
    }

    // Validate PIN format
    if req.new_pin.len() != 5 || !req.new_pin.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::BadRequest(
            "PIN must be exactly 5 digits".to_string(),
        ));
    }

    // Validate and decode token
    let user_profile_id = validate_pin_token(&req.verification_token, &state.config.pin_token_secret)?;

    // Get current PIN
    let current_pin: Option<String> = sqlx::query_scalar(
        r#"SELECT auth_pin FROM "Users" WHERE user_profile_id = $1"#,
    )
    .bind(user_profile_id)
    .fetch_one(&state.db)
    .await?;

    // Verify new PIN is different from current PIN
    if let Some(ref current) = current_pin {
        if &req.new_pin == current {
            return Err(AppError::BadRequest(
                "New PIN must be different from current PIN".to_string(),
            ));
        }
    }

    // Update PIN
    sqlx::query(r#"UPDATE "Users" SET auth_pin = $1 WHERE user_profile_id = $2"#)
        .bind(&req.new_pin)
        .bind(user_profile_id)
        .execute(&state.db)
        .await?;

    tracing::info!(
        user_profile_id,
        "Profile PIN changed successfully via token"
    );

    Ok(Json(SuccessResponse { success: true }))
}

// Note: has_permission is now centralized in crate::extractors::permissions::has_permission_by_name
