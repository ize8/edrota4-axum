use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use utoipa::IntoParams;

use crate::{
    extractors::AuthenticatedUser,
    models::{CreateTemplateInput, ShiftTemplate, TemplateMutationResponse, UpdateTemplateInput},
    AppError, AppResult, AppState,
};

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetTemplatesQuery {
    #[serde(rename = "roleId")]
    pub role_id: Option<i32>,
}

/// GET /api/templates?roleId=
#[utoipa::path(
    get,
    path = "/api/templates",
    params(GetTemplatesQuery),
    responses(
        (status = 200, description = "List of shift templates", body = Vec<ShiftTemplate>)
    ),
    tag = "templates"
)]
pub async fn get_templates(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetTemplatesQuery>,
) -> AppResult<Json<Vec<ShiftTemplate>>> {
    let mut sql = r#"
        SELECT
            id,
            role_id AS role,
            label,
            to_char(start, 'HH24:MI:SS') AS start,
            to_char("end", 'HH24:MI:SS') AS "end",
            font_color,
            bk_color,
            pa_value,
            money_per_hour,
            is_spa,
            is_dcc
        FROM "ShiftTemplates"
        WHERE 1=1
    "#
    .to_string();

    if let Some(_role_id) = query.role_id {
        sql.push_str(" AND role_id = $1");
    }

    sql.push_str(" ORDER BY label");

    let templates = if let Some(role_id) = query.role_id {
        sqlx::query_as::<_, ShiftTemplate>(&sql)
            .bind(role_id)
            .fetch_all(&state.db)
            .await?
    } else {
        sqlx::query_as::<_, ShiftTemplate>(&sql)
            .fetch_all(&state.db)
            .await?
    };

    Ok(Json(templates))
}

/// POST /api/templates - Create a new template
#[utoipa::path(
    post,
    path = "/api/templates",
    request_body = CreateTemplateInput,
    responses(
        (status = 200, description = "Template created successfully", body = ShiftTemplate),
        (status = 403, description = "Missing can_edit_templates permission")
    ),
    tag = "templates",
    security(("cookie_auth" = []))
)]
pub async fn create_template(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser,
    Json(input): Json<CreateTemplateInput>,
) -> AppResult<Json<ShiftTemplate>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_templates").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_templates permission".to_string(),
        ));
    }

    // Convert time strings to TIME format for database
    let start_time = input.start.as_ref().map(|s| format!("{}:00", s));
    let end_time = input.end.as_ref().map(|s| format!("{}:00", s));

    let template = sqlx::query_as::<_, ShiftTemplate>(
        r#"
        INSERT INTO "ShiftTemplates" (
            role_id, label, start, "end", pa_value, money_per_hour,
            font_color, bk_color, is_spa, is_dcc
        )
        VALUES ($1, $2, $3::time, $4::time, $5, $6, $7, $8, $9, $10)
        RETURNING
            id,
            role_id AS role,
            label,
            to_char(start, 'HH24:MI:SS') AS start,
            to_char("end", 'HH24:MI:SS') AS "end",
            font_color,
            bk_color,
            pa_value,
            money_per_hour,
            is_spa,
            is_dcc
        "#,
    )
    .bind(input.role)
    .bind(&input.label)
    .bind(start_time)
    .bind(end_time)
    .bind(input.pa_value)
    .bind(input.money_per_hour)
    .bind(&input.font_color)
    .bind(&input.bk_color)
    .bind(input.is_spa)
    .bind(input.is_dcc)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(template))
}

/// PUT /api/templates/{id} - Update a template
#[utoipa::path(
    put,
    path = "/api/templates/{id}",
    params(
        ("id" = i32, Path, description = "Template ID")
    ),
    request_body = UpdateTemplateInput,
    responses(
        (status = 200, description = "Template updated successfully", body = ShiftTemplate),
        (status = 400, description = "No fields to update"),
        (status = 403, description = "Missing can_edit_templates permission"),
        (status = 404, description = "Template not found")
    ),
    tag = "templates",
    security(("cookie_auth" = []))
)]
pub async fn update_template(
    State(state): State<Arc<AppState>>,
    Path(template_id): Path<i32>,
    auth: AuthenticatedUser,
    Json(input): Json<UpdateTemplateInput>,
) -> AppResult<Json<ShiftTemplate>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_templates").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_templates permission".to_string(),
        ));
    }

    // Build dynamic UPDATE query
    let mut updates = vec![];
    let mut bind_count = 1;

    if input.role.is_some() {
        updates.push(format!("role_id = ${}", bind_count));
        bind_count += 1;
    }
    if input.label.is_some() {
        updates.push(format!("label = ${}", bind_count));
        bind_count += 1;
    }
    if input.start.is_some() {
        updates.push(format!("start = ${}::time", bind_count));
        bind_count += 1;
    }
    if input.end.is_some() {
        updates.push(format!("\"end\" = ${}::time", bind_count));
        bind_count += 1;
    }
    if input.pa_value.is_some() {
        updates.push(format!("pa_value = ${}", bind_count));
        bind_count += 1;
    }
    if input.money_per_hour.is_some() {
        updates.push(format!("money_per_hour = ${}", bind_count));
        bind_count += 1;
    }
    if input.font_color.is_some() {
        updates.push(format!("font_color = ${}", bind_count));
        bind_count += 1;
    }
    if input.bk_color.is_some() {
        updates.push(format!("bk_color = ${}", bind_count));
        bind_count += 1;
    }
    if input.is_spa.is_some() {
        updates.push(format!("is_spa = ${}", bind_count));
        bind_count += 1;
    }
    if input.is_dcc.is_some() {
        updates.push(format!("is_dcc = ${}", bind_count));
        bind_count += 1;
    }

    if updates.is_empty() {
        return Err(AppError::BadRequest("No fields to update".to_string()));
    }

    let sql = format!(
        r#"
        UPDATE "ShiftTemplates"
        SET {}
        WHERE id = ${}
        RETURNING
            id,
            role_id AS role,
            label,
            to_char(start, 'HH24:MI:SS') AS start,
            to_char("end", 'HH24:MI:SS') AS "end",
            font_color,
            bk_color,
            pa_value,
            money_per_hour,
            is_spa,
            is_dcc
        "#,
        updates.join(", "),
        bind_count
    );

    // Build query with bindings
    let mut query = sqlx::query_as::<_, ShiftTemplate>(&sql);

    if let Some(role) = input.role {
        query = query.bind(role);
    }
    if let Some(label) = &input.label {
        query = query.bind(label);
    }
    if let Some(start) = &input.start {
        query = query.bind(format!("{}:00", start));
    }
    if let Some(end) = &input.end {
        query = query.bind(format!("{}:00", end));
    }
    if let Some(pa) = input.pa_value {
        query = query.bind(pa);
    }
    if let Some(money) = input.money_per_hour {
        query = query.bind(money);
    }
    if let Some(font_color) = &input.font_color {
        query = query.bind(font_color);
    }
    if let Some(bk_color) = &input.bk_color {
        query = query.bind(bk_color);
    }
    if let Some(is_spa) = input.is_spa {
        query = query.bind(is_spa);
    }
    if let Some(is_dcc) = input.is_dcc {
        query = query.bind(is_dcc);
    }

    query = query.bind(template_id);

    let updated_template = query.fetch_one(&state.db).await?;

    Ok(Json(updated_template))
}

/// DELETE /api/templates/{id} - Delete a template
#[utoipa::path(
    delete,
    path = "/api/templates/{id}",
    params(
        ("id" = i32, Path, description = "Template ID")
    ),
    responses(
        (status = 200, description = "Template deleted successfully", body = TemplateMutationResponse),
        (status = 403, description = "Missing can_edit_templates permission"),
        (status = 404, description = "Template not found")
    ),
    tag = "templates",
    security(("cookie_auth" = []))
)]
pub async fn delete_template(
    State(state): State<Arc<AppState>>,
    Path(template_id): Path<i32>,
    auth: AuthenticatedUser,
) -> AppResult<Json<TemplateMutationResponse>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_templates").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_templates permission".to_string(),
        ));
    }

    let result = sqlx::query(r#"DELETE FROM "ShiftTemplates" WHERE id = $1"#)
        .bind(template_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "Template {} not found",
            template_id
        )));
    }

    Ok(Json(TemplateMutationResponse {
        success: true,
        message: Some("Template deleted successfully".to_string()),
    }))
}

