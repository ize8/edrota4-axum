use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::{
    extractors::AuthenticatedUser,
    models::{CreateRoleInput, Role, RoleMutationResponse, UpdateRoleInput, Workplace},
    AppError, AppResult, AppState,
};

/// GET /api/roles
#[utoipa::path(
    get,
    path = "/api/roles",
    responses(
        (status = 200, description = "List of roles with joined workplace data", body = Vec<Role>)
    ),
    tag = "roles"
)]
pub async fn get_roles(State(state): State<Arc<AppState>>) -> AppResult<Json<Vec<Role>>> {
    let rows = sqlx::query_as::<_, (i32, i32, String, Option<i32>, Option<String>, Option<String>, Option<String>, Option<String>)>(
        r#"
        SELECT
            r.id::int4,
            r.workplace_id::int4,
            r.role_name,
            w.id::int4,
            w.hospital,
            w.ward,
            w.address,
            w.code
        FROM "Roles" r
        LEFT JOIN "Workplaces" w ON r.workplace_id = w.id
        ORDER BY r.id
        "#
    )
    .fetch_all(&state.db)
    .await?;

    let result = rows
        .into_iter()
        .map(|(id, workplace, role_name, w_id, w_hospital, w_ward, w_address, w_code)| Role {
            id,
            workplace,
            role_name,
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
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_staff").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_staff permission".to_string(),
        ));
    }

    // Insert the new role
    let role_id: i32 = sqlx::query_scalar(
        r#"
        INSERT INTO "Roles" (workplace_id, role_name)
        VALUES ($1, $2)
        RETURNING id
        "#,
    )
    .bind(input.workplace_id)
    .bind(&input.role_name)
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
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_staff").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_staff permission".to_string(),
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
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_staff").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_staff permission".to_string(),
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

/// Helper function to check if user has a specific permission
/// Helper function to fetch a role by ID with joined Workplace data
async fn fetch_role_by_id(db: &sqlx::PgPool, role_id: i32) -> AppResult<Role> {
    let row = sqlx::query_as::<_, (i32, i32, String, Option<i32>, Option<String>, Option<String>, Option<String>, Option<String>)>(
        r#"
        SELECT
            r.id::int4,
            r.workplace_id::int4,
            r.role_name,
            w.id::int4,
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
        workplaces: row.3.map(|id| Workplace {
            id,
            hospital: row.4,
            ward: row.5,
            address: row.6,
            code: row.7,
        }),
    })
}
