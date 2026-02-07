use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use utoipa::IntoParams;

use crate::{
    extractors::AuthenticatedUser,
    models::{CreateRoleInput, DependencyCount, Role, RoleMutationResponse, UpdateRoleInput, Workplace},
    AppError, AppResult, AppState,
};

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetRolesQuery {
    pub hospital: Option<String>,
    pub ward: Option<String>,
}

/// GET /api/roles?hospital=&ward=
#[utoipa::path(
    get,
    path = "/api/roles",
    params(GetRolesQuery),
    responses(
        (status = 200, description = "List of roles with joined workplace data (optionally filtered by workplace)", body = Vec<Role>)
    ),
    tag = "roles"
)]
pub async fn get_roles(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetRolesQuery>,
) -> AppResult<Json<Vec<Role>>> {
    // Build base query
    let mut sql = r#"
        SELECT
            r.id::int4,
            r.workplace_id::int4,
            r.role_name,
            r.marketplace_auto_approve,
            w.id::int8,
            w.hospital,
            w.ward,
            w.address,
            w.code
        FROM "Roles" r
        LEFT JOIN "Workplaces" w ON r.workplace_id = w.id
    "#.to_string();

    let mut conditions = vec![];
    let mut bind_values: Vec<String> = vec![];

    // Add filters if provided
    if let Some(hospital) = query.hospital {
        conditions.push(format!("w.hospital = ${}", bind_values.len() + 1));
        bind_values.push(hospital);
    }

    if let Some(ward) = query.ward {
        conditions.push(format!("w.ward = ${}", bind_values.len() + 1));
        bind_values.push(ward);
    }

    // Append WHERE clause if there are conditions
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }

    sql.push_str(" ORDER BY r.id");

    // Build and execute query
    let mut query_builder = sqlx::query_as::<_, (i32, i32, String, Option<bool>, Option<i64>, Option<String>, Option<String>, Option<String>, Option<String>)>(&sql);

    for value in bind_values {
        query_builder = query_builder.bind(value);
    }

    let rows = query_builder.fetch_all(&state.db).await?;

    let result = rows
        .into_iter()
        .map(|(id, workplace, role_name, marketplace_auto_approve, w_id, w_hospital, w_ward, w_address, w_code)| Role {
            id,
            workplace,
            role_name,
            marketplace_auto_approve,
            workplaces: w_id.map(|id| Workplace {
                id,
                hospital: w_hospital,
                ward: w_ward,
                address: w_address,
                code: w_code,
            }),
        })
        .collect();

    Ok(Json(result))
}

/// POST /api/roles - Create a new role
#[utoipa::path(
    post,
    path = "/api/roles",
    request_body = CreateRoleInput,
    responses(
        (status = 200, description = "Role created successfully", body = Role),
        (status = 403, description = "Missing can_edit_staff permission")
    ),
    tag = "roles",
    security(("cookie_auth" = []))
)]
pub async fn create_role(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser,
    Json(input): Json<CreateRoleInput>,
) -> AppResult<Json<Role>> {
    // Check permission - super admin only
    if !auth.is_super_admin {
        return Err(AppError::Forbidden(
            "Super admin permission required".to_string(),
        ));
    }

    // Insert the new role
    let role_id: i32 = sqlx::query_scalar(
        r#"
        INSERT INTO "Roles" (workplace_id, role_name, marketplace_auto_approve)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
    )
    .bind(input.workplace_id)
    .bind(&input.role_name)
    .bind(input.marketplace_auto_approve.unwrap_or(false))
    .fetch_one(&state.db)
    .await?;

    // Fetch the created role with joined workplace data
    let role = fetch_role_by_id(&state.db, role_id).await?;

    Ok(Json(role))
}

/// PUT /api/roles/{id} - Update a role
#[utoipa::path(
    put,
    path = "/api/roles/{id}",
    params(
        ("id" = i32, Path, description = "Role ID")
    ),
    request_body = UpdateRoleInput,
    responses(
        (status = 200, description = "Role updated successfully", body = Role),
        (status = 400, description = "No fields to update"),
        (status = 403, description = "Missing can_edit_staff permission"),
        (status = 404, description = "Role not found")
    ),
    tag = "roles",
    security(("cookie_auth" = []))
)]
pub async fn update_role(
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<i32>,
    auth: AuthenticatedUser,
    Json(input): Json<UpdateRoleInput>,
) -> AppResult<Json<Role>> {
    // Check permission - super admin only
    if !auth.is_super_admin {
        return Err(AppError::Forbidden(
            "Super admin permission required".to_string(),
        ));
    }

    // Build dynamic UPDATE query
    let mut updates = vec![];
    let mut bind_count = 1;

    if input.workplace_id.is_some() {
        updates.push(format!("workplace_id = ${}", bind_count));
        bind_count += 1;
    }
    if input.role_name.is_some() {
        updates.push(format!("role_name = ${}", bind_count));
        bind_count += 1;
    }
    if input.marketplace_auto_approve.is_some() {
        updates.push(format!("marketplace_auto_approve = ${}", bind_count));
        bind_count += 1;
    }

    if updates.is_empty() {
        return Err(AppError::BadRequest("No fields to update".to_string()));
    }

    let sql = format!(
        r#"UPDATE "Roles" SET {} WHERE id = ${}"#,
        updates.join(", "),
        bind_count
    );

    // Build query with bindings
    let mut query = sqlx::query(&sql);

    if let Some(workplace_id) = input.workplace_id {
        query = query.bind(workplace_id);
    }
    if let Some(role_name) = &input.role_name {
        query = query.bind(role_name);
    }
    if let Some(marketplace_auto_approve) = input.marketplace_auto_approve {
        query = query.bind(marketplace_auto_approve);
    }

    query = query.bind(role_id);

    let result = query.execute(&state.db).await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Role {} not found", role_id)));
    }

    // Fetch the updated role with joined workplace data
    let role = fetch_role_by_id(&state.db, role_id).await?;

    Ok(Json(role))
}

/// DELETE /api/roles/{id} - Delete a role
#[utoipa::path(
    delete,
    path = "/api/roles/{id}",
    params(
        ("id" = i32, Path, description = "Role ID")
    ),
    responses(
        (status = 200, description = "Role deleted successfully", body = RoleMutationResponse),
        (status = 403, description = "Missing can_edit_staff permission"),
        (status = 404, description = "Role not found")
    ),
    tag = "roles",
    security(("cookie_auth" = []))
)]
pub async fn delete_role(
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<i32>,
    auth: AuthenticatedUser,
) -> AppResult<Json<RoleMutationResponse>> {
    // Check permission - super admin only
    if !auth.is_super_admin {
        return Err(AppError::Forbidden(
            "Super admin permission required".to_string(),
        ));
    }

    let result = sqlx::query(r#"DELETE FROM "Roles" WHERE id = $1"#)
        .bind(role_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Role {} not found", role_id)));
    }

    Ok(Json(RoleMutationResponse {
        success: true,
        message: Some("Role deleted successfully".to_string()),
    }))
}

/// GET /api/roles/{id}/dependencies - Get dependency counts before deletion
#[utoipa::path(
    get,
    path = "/api/roles/{id}/dependencies",
    params(
        ("id" = i32, Path, description = "Role ID")
    ),
    responses(
        (status = 200, description = "Dependency counts", body = DependencyCount),
        (status = 403, description = "Super admin permission required")
    ),
    tag = "roles",
    security(("cookie_auth" = []))
)]
pub async fn get_role_dependencies(
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<i32>,
    auth: AuthenticatedUser,
) -> AppResult<Json<DependencyCount>> {
    // Check permission - super admin only
    if !auth.is_super_admin {
        return Err(AppError::Forbidden(
            "Super admin permission required".to_string(),
        ));
    }

    // Count dependencies for this role
    let user_roles_count: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::int8 FROM "UserRoles" WHERE role_id = $1"#
    )
    .bind(role_id)
    .fetch_one(&state.db)
    .await?;

    let job_plans_count: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::int8 FROM "JobPlans" WHERE user_role = $1"#
    )
    .bind(role_id)
    .fetch_one(&state.db)
    .await?;

    let shifts_count: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::int8 FROM "Shifts" WHERE role = $1"#
    )
    .bind(role_id)
    .fetch_one(&state.db)
    .await?;

    let templates_count: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::int8 FROM "ShiftTemplates" WHERE role = $1"#
    )
    .bind(role_id)
    .fetch_one(&state.db)
    .await?;

    let diary_count: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::int8 FROM "Diary" WHERE role_id = $1"#
    )
    .bind(role_id)
    .fetch_one(&state.db)
    .await?;

    let audit_count: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::int8 FROM "ShiftAudit" WHERE role = $1"#
    )
    .bind(role_id)
    .fetch_one(&state.db)
    .await?;

    let cod_count: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::int8 FROM "COD" WHERE role_id = $1"#
    )
    .bind(role_id)
    .fetch_one(&state.db)
    .await?;

    // Get shift UUIDs for marketplace requests
    let shift_uuids: Vec<String> = sqlx::query_scalar(
        r#"SELECT uuid::text FROM "Shifts" WHERE role = $1"#
    )
    .bind(role_id)
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
        r#"SELECT COUNT(DISTINCT user_profile_id)::int8 FROM "UserRoles" WHERE role_id = $1"#
    )
    .bind(role_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(DependencyCount {
        roles: 1,  // Single role
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

/// DELETE /api/roles/{id}/nuke - CASCADE delete role and ALL related data
#[utoipa::path(
    delete,
    path = "/api/roles/{id}/nuke",
    params(
        ("id" = i32, Path, description = "Role ID")
    ),
    responses(
        (status = 200, description = "Role and all dependencies deleted", body = RoleMutationResponse),
        (status = 403, description = "Super admin permission required"),
        (status = 404, description = "Role not found")
    ),
    tag = "roles",
    security(("cookie_auth" = []))
)]
pub async fn nuke_role(
    State(state): State<Arc<AppState>>,
    Path(role_id): Path<i32>,
    auth: AuthenticatedUser,
) -> AppResult<Json<RoleMutationResponse>> {
    // Check permission - super admin only
    if !auth.is_super_admin {
        return Err(AppError::Forbidden(
            "Super admin permission required".to_string(),
        ));
    }

    tracing::warn!("⚠️ NUKE: Starting cascade delete of role {}", role_id);

    // Start transaction
    let mut tx = state.db.begin().await?;

    // Get shift UUIDs for this role
    let shift_uuids: Vec<String> = sqlx::query_scalar(
        r#"SELECT uuid::text FROM "Shifts" WHERE role = $1"#
    )
    .bind(role_id)
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

    // 2. Job plans (references role)
    sqlx::query(r#"DELETE FROM "JobPlans" WHERE user_role = $1"#)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;

    // 3. Shift audit trail
    sqlx::query(r#"DELETE FROM "ShiftAudit" WHERE role = $1"#)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;

    // 4. Diary entries
    sqlx::query(r#"DELETE FROM "Diary" WHERE role_id = $1"#)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;

    // 5. Shifts
    sqlx::query(r#"DELETE FROM "Shifts" WHERE role = $1"#)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;

    // 6. Shift templates
    sqlx::query(r#"DELETE FROM "ShiftTemplates" WHERE role = $1"#)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;

    // 7. User role assignments
    sqlx::query(r#"DELETE FROM "UserRoles" WHERE role_id = $1"#)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;

    // 8. COD entries
    sqlx::query(r#"DELETE FROM "COD" WHERE role_id = $1"#)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;

    // 9. Finally, the role itself
    let result = sqlx::query(r#"DELETE FROM "Roles" WHERE id = $1"#)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Role {} not found", role_id)));
    }

    tx.commit().await?;
    tracing::warn!("⚠️ NUKE: Role {} annihilated", role_id);

    Ok(Json(RoleMutationResponse {
        success: true,
        message: Some("Role and all dependencies deleted".to_string()),
    }))
}

/// Helper function to check if user has a specific permission
/// Helper function to fetch a role by ID with joined Workplace data
async fn fetch_role_by_id(db: &sqlx::PgPool, role_id: i32) -> AppResult<Role> {
    let row = sqlx::query_as::<_, (i32, i32, String, Option<bool>, Option<i64>, Option<String>, Option<String>, Option<String>, Option<String>)>(
        r#"
        SELECT
            r.id::int4,
            r.workplace_id::int4,
            r.role_name,
            r.marketplace_auto_approve,
            w.id::int8,
            w.hospital,
            w.ward,
            w.address,
            w.code
        FROM "Roles" r
        LEFT JOIN "Workplaces" w ON r.workplace_id = w.id
        WHERE r.id = $1
        "#,
    )
    .bind(role_id)
    .fetch_one(db)
    .await?;

    Ok(Role {
        id: row.0,
        workplace: row.1,
        role_name: row.2,
        marketplace_auto_approve: row.3,
        workplaces: row.4.map(|id| Workplace {
            id,
            hospital: row.5,
            ward: row.6,
            address: row.7,
            code: row.8,
        }),
    })
}
