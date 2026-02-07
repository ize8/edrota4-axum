use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use utoipa::IntoParams;

use crate::{
    extractors::{permissions, AuthenticatedUser},
    models::AuditEntry,
    AppError, AppResult, AppState,
};

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetAuditQuery {
    #[serde(rename = "roleId")]
    pub role_id: Option<i32>,
    pub year: Option<i32>,
    pub month: Option<i32>,
}

/// GET /api/audit?roleId=&year=&month=
#[utoipa::path(
    get,
    path = "/api/audit",
    params(GetAuditQuery),
    responses(
        (status = 200, description = "List of audit entries for shift changes", body = Vec<AuditEntry>),
        (status = 403, description = "Missing required permissions (can_edit_staff, can_edit_templates, or can_edit_rota)")
    ),
    tag = "audit",
    security(("cookie_auth" = []))
)]
pub async fn get_audit(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser,
    Query(query): Query<GetAuditQuery>,
) -> AppResult<Json<Vec<AuditEntry>>> {
    // Check permissions - requires any of: can_edit_staff, can_edit_templates, can_edit_rota
    let has_perm = permissions::has_any_permission(
        &state.db,
        auth.profile_id,
        auth.is_super_admin,
        &[
            permissions::can_edit_staff,
            permissions::can_edit_templates,
            permissions::can_edit_rota,
        ],
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    if !has_perm {
        return Err(AppError::Forbidden(
            "Missing required permissions for audit access".to_string(),
        ));
    }

    // Build query with enrichment (joins to Users and TimeOffCategories)
    let mut sql = r#"
        SELECT
            sa.uuid,
            sa.role_id,
            sa.created_by,
            COALESCE(u.short_name, 'Unknown') AS created_by_name,
            sa.old,
            sa.new,
            u_old.short_name AS old_staff_name,
            u_new.short_name AS new_staff_name,
            toc_old.short_name AS old_time_off_category,
            toc_new.short_name AS new_time_off_category,
            COALESCE(sa.date::text, '') AS date,
            sa.created_at
        FROM "ShiftAudit" sa
        LEFT JOIN "Users" u ON sa.created_by = u.user_profile_id
        LEFT JOIN "Users" u_old ON (sa.old->>'user_profile_id')::int = u_old.user_profile_id
        LEFT JOIN "Users" u_new ON (sa.new->>'user_profile_id')::int = u_new.user_profile_id
        LEFT JOIN "TimeOffCategories" toc_old ON (sa.old->>'time_off')::int = toc_old.id
        LEFT JOIN "TimeOffCategories" toc_new ON (sa.new->>'time_off')::int = toc_new.id
        WHERE 1=1
    "#
    .to_string();

    let mut bindings = vec![];

    if let Some(role_id) = query.role_id {
        sql.push_str(&format!(" AND sa.role_id = ${}", bindings.len() + 1));
        bindings.push(role_id);
    }

    if let Some(year) = query.year {
        sql.push_str(&format!(" AND EXTRACT(YEAR FROM sa.date) = ${}", bindings.len() + 1));
        bindings.push(year);
    }

    if let Some(month) = query.month {
        sql.push_str(&format!(" AND EXTRACT(MONTH FROM sa.date) = ${}", bindings.len() + 1));
        bindings.push(month);
    }

    sql.push_str(" ORDER BY sa.created_at DESC");

    let mut query_builder = sqlx::query_as::<_, AuditEntry>(&sql);
    for binding in bindings {
        query_builder = query_builder.bind(binding);
    }

    let entries = query_builder.fetch_all(&state.db).await?;

    Ok(Json(entries))
}
