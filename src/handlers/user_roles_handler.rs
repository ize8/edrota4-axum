use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDateTime;
use serde::Deserialize;
use sqlx::FromRow;
use std::sync::Arc;
use utoipa::IntoParams;

use crate::{
    extractors::{permissions, AuthenticatedUser},
    models::{CreateUserRoleInput, Role, UpdateUserRoleInput, UserRole, UserRoleMutationResponse, Workplace},
    AppError, AppResult, AppState,
};

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetUserRolesQuery {
    pub user_profile_id: Option<i32>,
}

#[derive(Debug, FromRow)]
struct UserRoleQueryRow {
    id: i32,
    role_id: i32,
    user_profile_id: i32,
    can_edit_rota: bool,
    can_access_diary: bool,
    can_work_shifts: bool,
    can_edit_templates: bool,
    can_edit_staff: bool,
    can_view_staff_details: bool,
    created_at: NaiveDateTime,
    r_id: Option<i32>,
    r_workplace: Option<i32>,
    r_role_name: Option<String>,
    w_id: Option<i32>,  // INT4, not INT8
    w_hospital: Option<String>,
    w_ward: Option<String>,
    w_address: Option<String>,
    w_code: Option<String>,
}

/// GET /api/user-roles?user_profile_id=
#[utoipa::path(
    get,
    path = "/api/user-roles",
    params(GetUserRolesQuery),
    responses(
        (status = 200, description = "List of user role assignments with joined role and workplace data", body = Vec<UserRole>),
        (status = 403, description = "Missing can_edit_staff permission")
    ),
    tag = "user-roles",
    security(("cookie_auth" = []))
)]
pub async fn get_user_roles(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser,
    Query(query): Query<GetUserRolesQuery>,
) -> AppResult<Json<Vec<UserRole>>> {
    // Determine which user_profile_id to query for
    let target_user_id = query.user_profile_id.unwrap_or(auth.profile_id);

    // Permission check: users can view their own roles, but need can_edit_staff to view others
    let is_viewing_self = target_user_id == auth.profile_id;

    if !is_viewing_self {
        let has_perm = permissions::has_permission(
            &state.db,
            auth.profile_id,
            auth.is_super_admin,
            permissions::can_edit_staff,
        )
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if !has_perm {
            return Err(AppError::Forbidden(
                "Missing can_edit_staff permission to view other users' roles".to_string(),
            ));
        }
    }

    // Run super admin check and roles query IN PARALLEL (not sequential!)
    let (is_target_super_admin, actual_user_roles) = tokio::try_join!(
        async {
            sqlx::query_scalar::<_, bool>(
                r#"SELECT is_super_admin FROM "Users" WHERE user_profile_id = $1"#
            )
            .bind(target_user_id)
            .fetch_optional(&state.db)
            .await
            .map(|opt| opt.unwrap_or(false))
        },
        async {
            let query_str = r#"
                SELECT
                    ur.id::int4,
                    ur.role_id::int4,
                    ur.user_profile_id::int4,
                    ur.can_edit_rota,
                    ur.can_access_diary,
                    ur.can_work_shifts,
                    ur.can_edit_templates,
                    ur.can_edit_staff,
                    ur.can_view_staff_details,
                    ur.created_at,
                    r.id::int4 AS r_id,
                    r.workplace_id::int4 AS r_workplace,
                    r.role_name AS r_role_name,
                    w.id::int4 AS w_id,
                    w.hospital AS w_hospital,
                    w.ward AS w_ward,
                    w.address AS w_address,
                    w.code AS w_code
                FROM "UserRoles" ur
                LEFT JOIN "Roles" r ON ur.role_id = r.id
                LEFT JOIN "Workplaces" w ON r.workplace_id = w.id
                WHERE ur.user_profile_id = $1
                ORDER BY ur.id
            "#;
            sqlx::query_as::<_, UserRoleQueryRow>(query_str)
                .bind(target_user_id)
                .fetch_all(&state.db)
                .await
        }
    )?;

    let mut result: Vec<UserRole> = actual_user_roles
        .iter()
        .map(|row| UserRole {
            id: row.id,
            role_id: row.role_id,
            user_profile_id: row.user_profile_id,
            can_edit_rota: row.can_edit_rota,
            can_access_diary: row.can_access_diary,
            can_work_shifts: row.can_work_shifts,
            can_edit_templates: row.can_edit_templates,
            can_edit_staff: row.can_edit_staff,
            can_view_staff_details: row.can_view_staff_details,
            created_at: row.created_at,
            roles: row.r_id.map(|id| Role {
                id,
                workplace: row.r_workplace.unwrap_or(0),
                role_name: row.r_role_name.clone().unwrap_or_default(),
                marketplace_auto_approve: None,  // Not fetched in UserRoles query
                workplaces: row.w_id.map(|w_id| Workplace {
                    id: w_id,
                    hospital: row.w_hospital.clone(),
                    ward: row.w_ward.clone(),
                    address: row.w_address.clone(),
                    code: row.w_code.clone(),
                }),
            }),
        })
        .collect();

    // If target user is super admin, supplement with synthetic roles for missing roles
    if is_target_super_admin {
        tracing::info!(
            "User {} is super admin, supplementing with synthetic roles",
            target_user_id
        );

        // Get IDs of roles user already has
        let existing_role_ids: std::collections::HashSet<i32> = actual_user_roles
            .iter()
            .map(|row| row.role_id)
            .collect();

        // Fetch all roles that user doesn't have
        let all_roles = sqlx::query_as::<_, UserRoleQueryRow>(
            r#"
            SELECT
                r.id::int4 AS id,
                r.id::int4 AS role_id,
                $1::int4 AS user_profile_id,
                true AS can_edit_rota,
                true AS can_access_diary,
                true AS can_work_shifts,
                true AS can_edit_templates,
                true AS can_edit_staff,
                true AS can_view_staff_details,
                '1970-01-01 00:00:00'::timestamp AS created_at,
                r.id::int4 AS r_id,
                r.workplace_id::int4 AS r_workplace,
                r.role_name AS r_role_name,
                w.id::int4 AS w_id,
                w.hospital AS w_hospital,
                w.ward AS w_ward,
                w.address AS w_address,
                w.code AS w_code
            FROM "Roles" r
            LEFT JOIN "Workplaces" w ON r.workplace_id = w.id
            ORDER BY r.id
            "#,
        )
        .bind(target_user_id)
        .fetch_all(&state.db)
        .await?;

        // Create synthetic UserRole objects for roles not already assigned
        let synthetic_roles: Vec<UserRole> = all_roles
            .iter()
            .filter(|row| !existing_role_ids.contains(&row.role_id))
            .map(|row| UserRole {
                id: row.id,
                role_id: row.role_id,
                user_profile_id: row.user_profile_id,
                can_edit_rota: true,
                can_access_diary: true,
                can_work_shifts: true,
                can_edit_templates: true,
                can_edit_staff: true,
                can_view_staff_details: true,
                created_at: row.created_at,
                roles: row.r_id.map(|id| Role {
                    id,
                    workplace: row.r_workplace.unwrap_or(0),
                    role_name: row.r_role_name.clone().unwrap_or_default(),
                    marketplace_auto_approve: None,
                    workplaces: row.w_id.map(|w_id| Workplace {
                        id: w_id,
                        hospital: row.w_hospital.clone(),
                        ward: row.w_ward.clone(),
                        address: row.w_address.clone(),
                        code: row.w_code.clone(),
                    }),
                }),
            })
            .collect();

        tracing::info!(
            "Super admin {}: {} actual + {} synthetic = {} total roles",
            target_user_id,
            result.len(),
            synthetic_roles.len(),
            result.len() + synthetic_roles.len()
        );

        // Append synthetic roles (actual roles first, synthetic second)
        result.extend(synthetic_roles);
    }

    Ok(Json(result))
}

/// POST /api/user-roles - Create a new user role assignment
#[utoipa::path(
    post,
    path = "/api/user-roles",
    request_body = CreateUserRoleInput,
    responses(
        (status = 200, description = "User role created successfully", body = UserRole),
        (status = 403, description = "Missing can_edit_staff permission")
    ),
    tag = "user-roles",
    security(("cookie_auth" = []))
)]
pub async fn create_user_role(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser,
    Json(input): Json<CreateUserRoleInput>,
) -> AppResult<Json<UserRole>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_staff").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_staff permission".to_string(),
        ));
    }

    // Check for duplicate assignment
    let existing: Option<i32> = sqlx::query_scalar(
        r#"SELECT id FROM "UserRoles" WHERE user_profile_id = $1 AND role_id = $2"#
    )
    .bind(input.user_profile_id)
    .bind(input.role_id)
    .fetch_optional(&state.db)
    .await?;

    if existing.is_some() {
        return Err(AppError::BadRequest(
            "User already has this role assigned".to_string(),
        ));
    }

    // Check if user is a generic account
    let is_generic: bool = sqlx::query_scalar(
        r#"SELECT COALESCE(is_generic_login, false) FROM "Users" WHERE user_profile_id = $1"#
    )
    .bind(input.user_profile_id)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or(false);

    // Block generic accounts from having can_work_shifts permission
    if is_generic && input.can_work_shifts {
        return Err(AppError::BadRequest(
            "Generic accounts cannot have can_work_shifts permission".to_string(),
        ));
    }

    // Insert the new user role
    let user_role_id: i32 = sqlx::query_scalar(
        r#"
        INSERT INTO "UserRoles" (
            role_id, user_profile_id, can_edit_rota, can_access_diary,
            can_work_shifts, can_edit_templates, can_edit_staff, can_view_staff_details
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id
        "#,
    )
    .bind(input.role_id)
    .bind(input.user_profile_id)
    .bind(input.can_edit_rota)
    .bind(input.can_access_diary)
    .bind(input.can_work_shifts)
    .bind(input.can_edit_templates)
    .bind(input.can_edit_staff)
    .bind(input.can_view_staff_details)
    .fetch_one(&state.db)
    .await?;

    // Fetch the created user role with joined data
    let user_role = fetch_user_role_by_id(&state.db, user_role_id).await?;

    Ok(Json(user_role))
}

/// PUT /api/user-roles/{id} - Update a user role assignment
#[utoipa::path(
    put,
    path = "/api/user-roles/{id}",
    params(
        ("id" = i32, Path, description = "User role ID")
    ),
    request_body = UpdateUserRoleInput,
    responses(
        (status = 200, description = "User role updated successfully", body = UserRole),
        (status = 400, description = "No fields to update"),
        (status = 403, description = "Missing can_edit_staff permission"),
        (status = 404, description = "User role not found")
    ),
    tag = "user-roles",
    security(("cookie_auth" = []))
)]
pub async fn update_user_role(
    State(state): State<Arc<AppState>>,
    Path(user_role_id): Path<i32>,
    auth: AuthenticatedUser,
    Json(input): Json<UpdateUserRoleInput>,
) -> AppResult<Json<UserRole>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_staff").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_staff permission".to_string(),
        ));
    }

    // If trying to enable can_work_shifts, check if user is generic
    if let Some(true) = input.can_work_shifts {
        // Get user_profile_id for this user_role
        let user_profile_id: Option<i32> = sqlx::query_scalar(
            r#"SELECT user_profile_id FROM "UserRoles" WHERE id = $1"#
        )
        .bind(user_role_id)
        .fetch_optional(&state.db)
        .await?;

        if let Some(uid) = user_profile_id {
            let is_generic: bool = sqlx::query_scalar(
                r#"SELECT COALESCE(is_generic_login, false) FROM "Users" WHERE user_profile_id = $1"#
            )
            .bind(uid)
            .fetch_optional(&state.db)
            .await?
            .unwrap_or(false);

            if is_generic {
                return Err(AppError::BadRequest(
                    "Generic accounts cannot have can_work_shifts permission".to_string(),
                ));
            }
        }
    }

    // Build dynamic UPDATE query
    let mut updates = vec![];
    let mut bind_count = 1;

    if input.role_id.is_some() {
        updates.push(format!("role_id = ${}", bind_count));
        bind_count += 1;
    }
    if input.can_edit_rota.is_some() {
        updates.push(format!("can_edit_rota = ${}", bind_count));
        bind_count += 1;
    }
    if input.can_access_diary.is_some() {
        updates.push(format!("can_access_diary = ${}", bind_count));
        bind_count += 1;
    }
    if input.can_work_shifts.is_some() {
        updates.push(format!("can_work_shifts = ${}", bind_count));
        bind_count += 1;
    }
    if input.can_edit_templates.is_some() {
        updates.push(format!("can_edit_templates = ${}", bind_count));
        bind_count += 1;
    }
    if input.can_edit_staff.is_some() {
        updates.push(format!("can_edit_staff = ${}", bind_count));
        bind_count += 1;
    }
    if input.can_view_staff_details.is_some() {
        updates.push(format!("can_view_staff_details = ${}", bind_count));
        bind_count += 1;
    }

    if updates.is_empty() {
        return Err(AppError::BadRequest("No fields to update".to_string()));
    }

    let sql = format!(
        r#"UPDATE "UserRoles" SET {} WHERE id = ${}"#,
        updates.join(", "),
        bind_count
    );

    // Build query with bindings
    let mut query = sqlx::query(&sql);

    if let Some(role_id) = input.role_id {
        query = query.bind(role_id);
    }
    if let Some(can_edit_rota) = input.can_edit_rota {
        query = query.bind(can_edit_rota);
    }
    if let Some(can_access_diary) = input.can_access_diary {
        query = query.bind(can_access_diary);
    }
    if let Some(can_work_shifts) = input.can_work_shifts {
        query = query.bind(can_work_shifts);
    }
    if let Some(can_edit_templates) = input.can_edit_templates {
        query = query.bind(can_edit_templates);
    }
    if let Some(can_edit_staff) = input.can_edit_staff {
        query = query.bind(can_edit_staff);
    }
    if let Some(can_view_staff_details) = input.can_view_staff_details {
        query = query.bind(can_view_staff_details);
    }

    query = query.bind(user_role_id);

    let result = query.execute(&state.db).await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "User role {} not found",
            user_role_id
        )));
    }

    // Fetch the updated user role with joined data
    let user_role = fetch_user_role_by_id(&state.db, user_role_id).await?;

    Ok(Json(user_role))
}

/// DELETE /api/user-roles/{id} - Delete a user role assignment
#[utoipa::path(
    delete,
    path = "/api/user-roles/{id}",
    params(
        ("id" = i32, Path, description = "User role ID")
    ),
    responses(
        (status = 200, description = "User role deleted successfully", body = UserRoleMutationResponse),
        (status = 403, description = "Missing can_edit_staff permission"),
        (status = 404, description = "User role not found")
    ),
    tag = "user-roles",
    security(("cookie_auth" = []))
)]
pub async fn delete_user_role(
    State(state): State<Arc<AppState>>,
    Path(user_role_id): Path<i32>,
    auth: AuthenticatedUser,
) -> AppResult<Json<UserRoleMutationResponse>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_staff").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_staff permission".to_string(),
        ));
    }

    let result = sqlx::query(r#"DELETE FROM "UserRoles" WHERE id = $1"#)
        .bind(user_role_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "User role {} not found",
            user_role_id
        )));
    }

    Ok(Json(UserRoleMutationResponse {
        success: true,
        message: Some("User role deleted successfully".to_string()),
    }))
}

/// Helper function to check if user has a specific permission
/// Helper function to fetch a user role by ID with joined Role and Workplace data
async fn fetch_user_role_by_id(db: &sqlx::PgPool, user_role_id: i32) -> AppResult<UserRole> {
    let row = sqlx::query_as::<_, UserRoleQueryRow>(
        r#"
        SELECT
            ur.id::int4,
            ur.role_id::int4,
            ur.user_profile_id::int4,
            ur.can_edit_rota,
            ur.can_access_diary,
            ur.can_work_shifts,
            ur.can_edit_templates,
            ur.can_edit_staff,
            ur.can_view_staff_details,
            ur.created_at,
            r.id::int4 AS r_id,
            r.workplace_id::int4 AS r_workplace,
            r.role_name AS r_role_name,
            w.id::int4 AS w_id,
            w.hospital AS w_hospital,
            w.ward AS w_ward,
            w.address AS w_address,
            w.code AS w_code
        FROM "UserRoles" ur
        LEFT JOIN "Roles" r ON ur.role_id = r.id
        LEFT JOIN "Workplaces" w ON r.workplace_id = w.id
        WHERE ur.id = $1
        "#,
    )
    .bind(user_role_id)
    .fetch_one(db)
    .await?;

    Ok(UserRole {
        id: row.id,
        role_id: row.role_id,
        user_profile_id: row.user_profile_id,
        can_edit_rota: row.can_edit_rota,
        can_access_diary: row.can_access_diary,
        can_work_shifts: row.can_work_shifts,
        can_edit_templates: row.can_edit_templates,
        can_edit_staff: row.can_edit_staff,
        can_view_staff_details: row.can_view_staff_details,
        created_at: row.created_at,
        roles: row.r_id.map(|id| Role {
            id,
            workplace: row.r_workplace.unwrap_or(0),
            role_name: row.r_role_name.unwrap_or_default(),
            marketplace_auto_approve: None,
            workplaces: row.w_id.map(|w_id| Workplace {
                id: w_id,
                hospital: row.w_hospital,
                ward: row.w_ward,
                address: row.w_address,
                code: row.w_code,
            }),
        }),
    })
}
