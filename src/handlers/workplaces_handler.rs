use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::{
    extractors::AuthenticatedUser,
    models::{CreateWorkplaceInput, DependencyCount, UpdateWorkplaceInput, Workplace, WorkplaceMutationResponse},
    AppError, AppResult, AppState,
};

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
    let workplaces =
        sqlx::query_as::<_, Workplace>(r#"SELECT * FROM "Workplaces" ORDER BY id"#)
            .fetch_all(&state.db)
            .await?;

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
        RETURNING id, hospital, ward, address, code
        "#,
    )
    .bind(&input.hospital)
    .bind(&input.ward)
    .bind(&input.address)
    .bind(&input.code)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(workplace))
}

/// PUT /api/workplaces/{id} - Update a workplace
#[utoipa::path(
    put,
    path = "/api/workplaces/{id}",
    params(
        ("id" = i64, Path, description = "Workplace ID")
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
    Path(workplace_id): Path<i64>,
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
        RETURNING id, hospital, ward, address, code
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
        Some(wp) => Ok(Json(wp)),
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
        ("id" = i64, Path, description = "Workplace ID")
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
    Path(workplace_id): Path<i64>,
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
        ("id" = i64, Path, description = "Workplace ID")
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
    Path(workplace_id): Path<i64>,
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
        r#"SELECT id FROM "Roles" WHERE workplace_id = $1"#
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

    // Build IN clause for role IDs
    let role_ids_str = role_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");

    // Count dependencies
    let user_roles_count: i64 = sqlx::query_scalar(
        &format!(r#"SELECT COUNT(*)::int8 FROM "UserRoles" WHERE role_id IN ({})"#, role_ids_str)
    )
    .fetch_one(&state.db)
    .await?;

    let job_plans_count: i64 = sqlx::query_scalar(
        &format!(r#"SELECT COUNT(*)::int8 FROM "JobPlans" WHERE user_role IN ({})"#, role_ids_str)
    )
    .fetch_one(&state.db)
    .await?;

    let shifts_count: i64 = sqlx::query_scalar(
        &format!(r#"SELECT COUNT(*)::int8 FROM "Shifts" WHERE role IN ({})"#, role_ids_str)
    )
    .fetch_one(&state.db)
    .await?;

    let templates_count: i64 = sqlx::query_scalar(
        &format!(r#"SELECT COUNT(*)::int8 FROM "ShiftTemplates" WHERE role IN ({})"#, role_ids_str)
    )
    .fetch_one(&state.db)
    .await?;

    let diary_count: i64 = sqlx::query_scalar(
        &format!(r#"SELECT COUNT(*)::int8 FROM "Diary" WHERE role_id IN ({})"#, role_ids_str)
    )
    .fetch_one(&state.db)
    .await?;

    let audit_count: i64 = sqlx::query_scalar(
        &format!(r#"SELECT COUNT(*)::int8 FROM "ShiftAudit" WHERE role IN ({})"#, role_ids_str)
    )
    .fetch_one(&state.db)
    .await?;

    let cod_count: i64 = sqlx::query_scalar(
        &format!(r#"SELECT COUNT(*)::int8 FROM "COD" WHERE role_id IN ({})"#, role_ids_str)
    )
    .fetch_one(&state.db)
    .await?;

    // Get shift UUIDs for marketplace requests
    let shift_uuids: Vec<String> = sqlx::query_scalar(
        &format!(r#"SELECT uuid::text FROM "Shifts" WHERE role IN ({})"#, role_ids_str)
    )
    .fetch_all(&state.db)
    .await?;

    let shift_requests_count: i64 = if !shift_uuids.is_empty() {
        let uuids_str = shift_uuids.iter().map(|u| format!("'{}'", u)).collect::<Vec<_>>().join(",");
        sqlx::query_scalar(
            &format!(r#"SELECT COUNT(*)::int8 FROM "ShiftRequests" WHERE shift_id::text IN ({})"#, uuids_str)
        )
        .fetch_one(&state.db)
        .await?
    } else {
        0
    };

    // Get unique staff count
    let unique_staff: i64 = sqlx::query_scalar(
        &format!(r#"SELECT COUNT(DISTINCT user_profile_id)::int8 FROM "UserRoles" WHERE role_id IN ({})"#, role_ids_str)
    )
    .fetch_one(&state.db)
    .await?;

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
        ("id" = i64, Path, description = "Workplace ID")
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
    Path(workplace_id): Path<i64>,
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
        r#"SELECT id FROM "Roles" WHERE workplace_id = $1"#
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
        tracing::info!("NUKE: Workplace deleted (no roles)");
        return Ok(Json(WorkplaceMutationResponse {
            success: true,
            message: Some("Workplace deleted (no dependencies)".to_string()),
        }));
    }

    let role_ids_str = role_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
    tracing::info!("NUKE: Deleting {} roles and all related data", role_ids.len());

    // Get shift UUIDs
    let shift_uuids: Vec<String> = sqlx::query_scalar(
        &format!(r#"SELECT uuid::text FROM "Shifts" WHERE role IN ({})"#, role_ids_str)
    )
    .fetch_all(&mut *tx)
    .await?;

    // Delete in order (deepest children → parent):

    // 1. Shift requests (references shifts)
    if !shift_uuids.is_empty() {
        let uuids_str = shift_uuids.iter().map(|u| format!("'{}'", u)).collect::<Vec<_>>().join(",");
        sqlx::query(&format!(r#"DELETE FROM "ShiftRequests" WHERE shift_id::text IN ({})"#, uuids_str))
            .execute(&mut *tx)
            .await?;
        tracing::info!("NUKE: Deleted shift requests");
    }

    // 2. Job plans (references roles)
    sqlx::query(&format!(r#"DELETE FROM "JobPlans" WHERE user_role IN ({})"#, role_ids_str))
        .execute(&mut *tx)
        .await?;

    // 3. Shift audit trail
    sqlx::query(&format!(r#"DELETE FROM "ShiftAudit" WHERE role IN ({})"#, role_ids_str))
        .execute(&mut *tx)
        .await?;

    // 4. Diary entries
    sqlx::query(&format!(r#"DELETE FROM "Diary" WHERE role_id IN ({})"#, role_ids_str))
        .execute(&mut *tx)
        .await?;

    // 5. Shifts
    sqlx::query(&format!(r#"DELETE FROM "Shifts" WHERE role IN ({})"#, role_ids_str))
        .execute(&mut *tx)
        .await?;

    // 6. Shift templates
    sqlx::query(&format!(r#"DELETE FROM "ShiftTemplates" WHERE role IN ({})"#, role_ids_str))
        .execute(&mut *tx)
        .await?;

    // 7. User role assignments
    sqlx::query(&format!(r#"DELETE FROM "UserRoles" WHERE role_id IN ({})"#, role_ids_str))
        .execute(&mut *tx)
        .await?;

    // 8. COD entries
    sqlx::query(&format!(r#"DELETE FROM "COD" WHERE role_id IN ({})"#, role_ids_str))
        .execute(&mut *tx)
        .await?;

    // 9. Roles
    sqlx::query(&format!(r#"DELETE FROM "Roles" WHERE workplace_id = {}"#, workplace_id))
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
    tracing::warn!("⚠️ NUKE: Workplace {} annihilated ({} roles deleted)", workplace_id, role_ids.len());

    Ok(Json(WorkplaceMutationResponse {
        success: true,
        message: Some(format!("Workplace and {} roles with all dependencies deleted", role_ids.len())),
    }))
}