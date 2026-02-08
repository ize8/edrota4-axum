use axum::{
    extract::{Path, State},
    Json,
};
use moka::future::Cache;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::time::Duration;

use crate::{
    extractors::AuthenticatedUser,
    models::{CreateWorkplaceInput, DependencyCount, UpdateWorkplaceInput, Workplace, WorkplaceMutationResponse},
    AppError, AppResult, AppState,
};

// Cache all workplaces with 60-second TTL
static WORKPLACES_CACHE: Lazy<Cache<&'static str, Vec<Workplace>>> = Lazy::new(|| {
    Cache::builder()
        .time_to_live(Duration::from_secs(60))
        .build()
});

async fn invalidate_workplaces_cache() {
    WORKPLACES_CACHE.invalidate(&"all").await;
}

/// GET /api/workplaces
#[utoipa::path(
    get,
    path = "/api/workplaces",
    responses(
        (status = 200, description = "List of workplaces", body = Vec<Workplace>)
    ),
    tag = "workplaces"
)]
pub async fn get_workplaces(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Vec<Workplace>>> {
    if let Some(cached) = WORKPLACES_CACHE.get(&"all").await {
        return Ok(Json(cached));
    }

    let workplaces =
        sqlx::query_as::<_, Workplace>(r#"SELECT id::int4, hospital, ward, address, code FROM "Workplaces" ORDER BY id"#)
            .fetch_all(&state.db)
            .await?;

    WORKPLACES_CACHE.insert("all", workplaces.clone()).await;
    Ok(Json(workplaces))
}

/// POST /api/workplaces - Create a new workplace
#[utoipa::path(
    post,
    path = "/api/workplaces",
    request_body = CreateWorkplaceInput,
    responses(
        (status = 200, description = "Workplace created successfully", body = Workplace),
        (status = 403, description = "Missing can_edit_staff permission")
    ),
    tag = "workplaces",
    security(("cookie_auth" = []))
)]
pub async fn create_workplace(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser,
    Json(input): Json<CreateWorkplaceInput>,
) -> AppResult<Json<Workplace>> {
    // Check permission - super admin only
    if !auth.is_super_admin {
        return Err(AppError::Forbidden(
            "Super admin permission required".to_string(),
        ));
    }

    // Insert the new workplace
    let workplace = sqlx::query_as::<_, Workplace>(
        r#"
        INSERT INTO "Workplaces" (hospital, ward, address, code)
        VALUES ($1, $2, $3, $4)
        RETURNING id::int4, hospital, ward, address, code
        "#,
    )
    .bind(&input.hospital)
    .bind(&input.ward)
    .bind(&input.address)
    .bind(&input.code)
    .fetch_one(&state.db)
    .await?;

    invalidate_workplaces_cache().await;
    Ok(Json(workplace))
}

/// PUT /api/workplaces/{id} - Update a workplace
#[utoipa::path(
    put,
    path = "/api/workplaces/{id}",
    params(
        ("id" = i32, Path, description = "Workplace ID")
    ),
    request_body = UpdateWorkplaceInput,
    responses(
        (status = 200, description = "Workplace updated successfully", body = Workplace),
        (status = 400, description = "No fields to update"),
        (status = 403, description = "Missing can_edit_staff permission"),
        (status = 404, description = "Workplace not found")
    ),
    tag = "workplaces",
    security(("cookie_auth" = []))
)]
pub async fn update_workplace(
    State(state): State<Arc<AppState>>,
    Path(workplace_id): Path<i32>,
    auth: AuthenticatedUser,
    Json(input): Json<UpdateWorkplaceInput>,
) -> AppResult<Json<Workplace>> {
    // Check permission - super admin only
    if !auth.is_super_admin {
        return Err(AppError::Forbidden(
            "Super admin permission required".to_string(),
        ));
    }

    // Build dynamic UPDATE query
    let mut updates = vec![];
    let mut bind_count = 1;

    if input.hospital.is_some() {
        updates.push(format!("hospital = ${}", bind_count));
        bind_count += 1;
    }
    if input.ward.is_some() {
        updates.push(format!("ward = ${}", bind_count));
        bind_count += 1;
    }
    if input.address.is_some() {
        updates.push(format!("address = ${}", bind_count));
        bind_count += 1;
    }
    if input.code.is_some() {
        updates.push(format!("code = ${}", bind_count));
        bind_count += 1;
    }

    if updates.is_empty() {
        return Err(AppError::BadRequest("No fields to update".to_string()));
    }

    let sql = format!(
        r#"
        UPDATE "Workplaces"
        SET {}
        WHERE id = ${}
        RETURNING id::int4, hospital, ward, address, code
        "#,
        updates.join(", "),
        bind_count
    );

    // Build query with bindings
    let mut query = sqlx::query_as::<_, Workplace>(&sql);

    if let Some(hospital) = &input.hospital {
        query = query.bind(hospital);
    }
    if let Some(ward) = &input.ward {
        query = query.bind(ward);
    }
    if let Some(address) = &input.address {
        query = query.bind(address);
    }
    if let Some(code) = &input.code {
        query = query.bind(code);
    }

    query = query.bind(workplace_id);

    let workplace = query.fetch_optional(&state.db).await?;

    match workplace {
        Some(wp) => {
            invalidate_workplaces_cache().await;
            Ok(Json(wp))
        }
        None => Err(AppError::NotFound(format!(
            "Workplace {} not found",
            workplace_id
        ))),
    }
}

/// DELETE /api/workplaces/{id} - Delete a workplace
#[utoipa::path(
    delete,
    path = "/api/workplaces/{id}",
    params(
        ("id" = i32, Path, description = "Workplace ID")
    ),
    responses(
        (status = 200, description = "Workplace deleted successfully", body = WorkplaceMutationResponse),
        (status = 403, description = "Missing can_edit_staff permission"),
        (status = 404, description = "Workplace not found")
    ),
    tag = "workplaces",
    security(("cookie_auth" = []))
)]
pub async fn delete_workplace(
    State(state): State<Arc<AppState>>,
    Path(workplace_id): Path<i32>,
    auth: AuthenticatedUser,
) -> AppResult<Json<WorkplaceMutationResponse>> {
    // Check permission - super admin only
    if !auth.is_super_admin {
        return Err(AppError::Forbidden(
            "Super admin permission required".to_string(),
        ));
    }

    let result = sqlx::query(r#"DELETE FROM "Workplaces" WHERE id = $1"#)
        .bind(workplace_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "Workplace {} not found",
            workplace_id
        )));
    }

    invalidate_workplaces_cache().await;
    Ok(Json(WorkplaceMutationResponse {
        success: true,
        message: Some("Workplace deleted successfully".to_string()),
    }))
}

/// GET /api/workplaces/{id}/dependencies - Get dependency counts before deletion
#[utoipa::path(
    get,
    path = "/api/workplaces/{id}/dependencies",
    params(
        ("id" = i32, Path, description = "Workplace ID")
    ),
    responses(
        (status = 200, description = "Dependency counts", body = DependencyCount),
        (status = 403, description = "Super admin permission required")
    ),
    tag = "workplaces",
    security(("cookie_auth" = []))
)]
pub async fn get_workplace_dependencies(
    State(state): State<Arc<AppState>>,
    Path(workplace_id): Path<i32>,
    auth: AuthenticatedUser,
) -> AppResult<Json<DependencyCount>> {
    // Check permission - super admin only
    if !auth.is_super_admin {
        return Err(AppError::Forbidden(
            "Super admin permission required".to_string(),
        ));
    }

    // Get all roles for this workplace
    let role_ids: Vec<i32> = sqlx::query_scalar(
        r#"SELECT id::int4 FROM "Roles" WHERE workplace_id = $1"#
    )
    .bind(workplace_id)
    .fetch_all(&state.db)
    .await?;

    if role_ids.is_empty() {
        return Ok(Json(DependencyCount {
            roles: 0,
            user_roles: 0,
            job_plans: 0,
            shifts: 0,
            shift_requests: 0,
            templates: 0,
            diary_entries: 0,
            audit_entries: 0,
            cod_entries: 0,
            unique_staff: 0,
        }));
    }

    // Run all 9 COUNT queries in parallel for ~9x speedup
    let db = &state.db;
    let (
        user_roles_count,
        job_plans_count,
        shifts_count,
        templates_count,
        diary_count,
        audit_count,
        cod_count,
        shift_requests_count,
        unique_staff,
    ) = tokio::try_join!(
        sqlx::query_scalar::<_, i64>(r#"SELECT COUNT(*)::int8 FROM "UserRoles" WHERE role_id = ANY($1)"#)
            .bind(&role_ids).fetch_one(db),
        sqlx::query_scalar::<_, i64>(r#"SELECT COUNT(*)::int8 FROM "JobPlans" WHERE role_id = ANY($1)"#)
            .bind(&role_ids).fetch_one(db),
        sqlx::query_scalar::<_, i64>(r#"SELECT COUNT(*)::int8 FROM "Shifts" WHERE role_id = ANY($1)"#)
            .bind(&role_ids).fetch_one(db),
        sqlx::query_scalar::<_, i64>(r#"SELECT COUNT(*)::int8 FROM "ShiftTemplates" WHERE role_id = ANY($1)"#)
            .bind(&role_ids).fetch_one(db),
        sqlx::query_scalar::<_, i64>(r#"SELECT COUNT(*)::int8 FROM "Diary" WHERE role_id = ANY($1)"#)
            .bind(&role_ids).fetch_one(db),
        sqlx::query_scalar::<_, i64>(r#"SELECT COUNT(*)::int8 FROM "ShiftAudit" WHERE role_id = ANY($1)"#)
            .bind(&role_ids).fetch_one(db),
        sqlx::query_scalar::<_, i64>(r#"SELECT COUNT(*)::int8 FROM "COD" WHERE role_id = ANY($1)"#)
            .bind(&role_ids).fetch_one(db),
        sqlx::query_scalar::<_, i64>(r#"SELECT COUNT(*)::int8 FROM "ShiftRequests" WHERE shift_id IN (SELECT uuid FROM "Shifts" WHERE role_id = ANY($1))"#)
            .bind(&role_ids).fetch_one(db),
        sqlx::query_scalar::<_, i64>(r#"SELECT COUNT(DISTINCT user_profile_id)::int8 FROM "UserRoles" WHERE role_id = ANY($1)"#)
            .bind(&role_ids).fetch_one(db),
    )?;

    Ok(Json(DependencyCount {
        roles: role_ids.len() as i32,
        user_roles: user_roles_count as i32,
        job_plans: job_plans_count as i32,
        shifts: shifts_count as i32,
        shift_requests: shift_requests_count as i32,
        templates: templates_count as i32,
        diary_entries: diary_count as i32,
        audit_entries: audit_count as i32,
        cod_entries: cod_count as i32,
        unique_staff: unique_staff as i32,
    }))
}

/// DELETE /api/workplaces/{id}/nuke - CASCADE delete workplace and ALL related data
#[utoipa::path(
    delete,
    path = "/api/workplaces/{id}/nuke",
    params(
        ("id" = i32, Path, description = "Workplace ID")
    ),
    responses(
        (status = 200, description = "Workplace and all dependencies deleted", body = WorkplaceMutationResponse),
        (status = 403, description = "Super admin permission required"),
        (status = 404, description = "Workplace not found")
    ),
    tag = "workplaces",
    security(("cookie_auth" = []))
)]
pub async fn nuke_workplace(
    State(state): State<Arc<AppState>>,
    Path(workplace_id): Path<i32>,
    auth: AuthenticatedUser,
) -> AppResult<Json<WorkplaceMutationResponse>> {
    // Check permission - super admin only
    if !auth.is_super_admin {
        return Err(AppError::Forbidden(
            "Super admin permission required".to_string(),
        ));
    }

    tracing::warn!("⚠️ NUKE: Starting cascade delete of workplace {}", workplace_id);

    // Start transaction
    let mut tx = state.db.begin().await?;

    // Get all roles for this workplace
    let role_ids: Vec<i32> = sqlx::query_scalar(
        r#"SELECT id::int4 FROM "Roles" WHERE workplace_id = $1"#
    )
    .bind(workplace_id)
    .fetch_all(&mut *tx)
    .await?;

    if role_ids.is_empty() {
        // No roles, just delete the workplace
        let result = sqlx::query(r#"DELETE FROM "Workplaces" WHERE id = $1"#)
            .bind(workplace_id)
            .execute(&mut *tx)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Workplace {} not found", workplace_id)));
        }

        tx.commit().await?;
        invalidate_workplaces_cache().await;
        tracing::info!("🗑️ NUKE: Workplace deleted (no roles)");
        return Ok(Json(WorkplaceMutationResponse {
            success: true,
            message: Some("Workplace deleted (no dependencies)".to_string()),
        }));
    }

    tracing::info!("🗑️ NUKE: Deleting {} roles and all related data", role_ids.len());

    // Delete in order (deepest children → parent), using parameterized queries:

    // 1. Shift requests (references shifts via subquery)
    sqlx::query(r#"DELETE FROM "ShiftRequests" WHERE shift_id IN (SELECT uuid FROM "Shifts" WHERE role_id = ANY($1))"#)
        .bind(&role_ids)
        .execute(&mut *tx)
        .await?;
    tracing::info!("🗑️ NUKE: Deleted shift requests");

    // 2. Job plans (references roles)
    sqlx::query(r#"DELETE FROM "JobPlans" WHERE role_id = ANY($1)"#)
        .bind(&role_ids)
        .execute(&mut *tx)
        .await?;

    // 3. Shift audit trail
    sqlx::query(r#"DELETE FROM "ShiftAudit" WHERE role_id = ANY($1)"#)
        .bind(&role_ids)
        .execute(&mut *tx)
        .await?;

    // 4. Diary entries
    sqlx::query(r#"DELETE FROM "Diary" WHERE role_id = ANY($1)"#)
        .bind(&role_ids)
        .execute(&mut *tx)
        .await?;

    // 5. Shifts
    sqlx::query(r#"DELETE FROM "Shifts" WHERE role_id = ANY($1)"#)
        .bind(&role_ids)
        .execute(&mut *tx)
        .await?;

    // 6. Shift templates
    sqlx::query(r#"DELETE FROM "ShiftTemplates" WHERE role_id = ANY($1)"#)
        .bind(&role_ids)
        .execute(&mut *tx)
        .await?;

    // 7. User role assignments
    sqlx::query(r#"DELETE FROM "UserRoles" WHERE role_id = ANY($1)"#)
        .bind(&role_ids)
        .execute(&mut *tx)
        .await?;

    // 8. COD entries
    sqlx::query(r#"DELETE FROM "COD" WHERE role_id = ANY($1)"#)
        .bind(&role_ids)
        .execute(&mut *tx)
        .await?;

    // 9. Roles
    sqlx::query(r#"DELETE FROM "Roles" WHERE workplace_id = $1"#)
        .bind(workplace_id)
        .execute(&mut *tx)
        .await?;

    // 10. Finally, the workplace
    let result = sqlx::query(r#"DELETE FROM "Workplaces" WHERE id = $1"#)
        .bind(workplace_id)
        .execute(&mut *tx)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Workplace {} not found", workplace_id)));
    }

    tx.commit().await?;
    invalidate_workplaces_cache().await;
    tracing::warn!("⚠️ NUKE: Workplace {} annihilated ({} roles deleted)", workplace_id, role_ids.len());

    Ok(Json(WorkplaceMutationResponse {
        success: true,
        message: Some(format!("Workplace and {} roles with all dependencies deleted", role_ids.len())),
    }))
}