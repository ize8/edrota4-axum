use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDate;
use serde::Deserialize;
use std::sync::Arc;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::{
    extractors::AuthenticatedUser,
    models::{CreateShiftInput, Shift, ShiftMutationResponse, UpdateShiftInput},
    AppError, AppResult, AppState,
};

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetShiftsQuery {
    pub year: Option<i32>,
    pub month: Option<i32>,
    #[serde(rename = "roleId")]
    pub role_id: Option<i32>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetShiftsByDateQuery {
    pub date: String,
    #[serde(rename = "roleId")]
    pub role_id: Option<i32>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetShiftsRangeQuery {
    pub start: String,
    pub end: String,
    #[serde(rename = "roleId")]
    pub role_id: Option<i32>,
}

/// GET /api/shifts?year=&month=&roleId=
#[utoipa::path(
    get,
    path = "/api/shifts",
    params(GetShiftsQuery),
    responses(
        (status = 200, description = "List of shifts for specified month/year and optional role filter", body = Vec<Shift>)
    ),
    tag = "shifts"
)]
pub async fn get_shifts_for_month(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetShiftsQuery>,
) -> AppResult<Json<Vec<Shift>>> {
    tracing::debug!("get_shifts_for_month called with year={:?}, month={:?}, role_id={:?}",
        query.year, query.month, query.role_id);

    let mut sql = r#"
        SELECT
            uuid,
            role_id AS role,
            label,
            to_char(start, 'HH24:MI:SS') AS start,
            to_char("end", 'HH24:MI:SS') AS "end",
            money_per_hour,
            pa_value,
            font_color,
            bk_color,
            is_locum,
            published,
            date,
            created_at,
            is_dcc,
            is_spa,
            time_off_category_id AS time_off,
            user_profile_id,
            created_by
        FROM "Shifts"
        WHERE 1=1
    "#
    .to_string();

    let mut bindings = vec![];

    if let Some(year) = query.year {
        if let Some(month) = query.month {
            sql.push_str(&format!(" AND EXTRACT(YEAR FROM date) = ${}", bindings.len() + 1));
            bindings.push(year);
            sql.push_str(&format!(" AND EXTRACT(MONTH FROM date) = ${}", bindings.len() + 1));
            bindings.push(month);
        }
    }

    if let Some(role_id) = query.role_id {
        sql.push_str(&format!(" AND role_id = ${}", bindings.len() + 1));
        bindings.push(role_id);
    }

    sql.push_str(" ORDER BY date, start");

    let mut query_builder = sqlx::query_as::<_, Shift>(&sql);
    for binding in bindings {
        query_builder = query_builder.bind(binding);
    }

    let shifts = query_builder.fetch_all(&state.db).await?;

    Ok(Json(shifts))
}

/// GET /api/shifts/by-date?date=&roleId=
#[utoipa::path(
    get,
    path = "/api/shifts/by-date",
    params(GetShiftsByDateQuery),
    responses(
        (status = 200, description = "List of shifts for a specific date", body = Vec<Shift>),
        (status = 400, description = "Invalid date format")
    ),
    tag = "shifts"
)]
pub async fn get_shifts_for_date(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetShiftsByDateQuery>,
) -> AppResult<Json<Vec<Shift>>> {
    let date = NaiveDate::parse_from_str(&query.date, "%Y-%m-%d")
        .map_err(|e| crate::AppError::BadRequest(format!("Invalid date format: {}", e)))?;

    let mut sql = r#"
        SELECT
            uuid,
            role_id AS role,
            label,
            to_char(start, 'HH24:MI:SS') AS start,
            to_char("end", 'HH24:MI:SS') AS "end",
            money_per_hour,
            pa_value,
            font_color,
            bk_color,
            is_locum,
            published,
            date,
            created_at,
            is_dcc,
            is_spa,
            time_off_category_id AS time_off,
            user_profile_id,
            created_by
        FROM "Shifts"
        WHERE date = $1
    "#
    .to_string();

    if let Some(role_id) = query.role_id {
        sql.push_str(" AND role_id = $2");
    }

    sql.push_str(" ORDER BY start, role, label");

    let mut query_builder = sqlx::query_as::<_, Shift>(&sql).bind(date);
    if let Some(role_id) = query.role_id {
        query_builder = query_builder.bind(role_id);
    }

    let shifts = query_builder.fetch_all(&state.db).await?;

    Ok(Json(shifts))
}

/// GET /api/shifts/range?start=&end=&roleId=
#[utoipa::path(
    get,
    path = "/api/shifts/range",
    params(GetShiftsRangeQuery),
    responses(
        (status = 200, description = "List of shifts within date range", body = Vec<Shift>),
        (status = 400, description = "Invalid date format")
    ),
    tag = "shifts"
)]
pub async fn get_shifts_for_range(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetShiftsRangeQuery>,
) -> AppResult<Json<Vec<Shift>>> {
    let start_date = NaiveDate::parse_from_str(&query.start, "%Y-%m-%d")
        .map_err(|e| crate::AppError::BadRequest(format!("Invalid start date: {}", e)))?;
    let end_date = NaiveDate::parse_from_str(&query.end, "%Y-%m-%d")
        .map_err(|e| crate::AppError::BadRequest(format!("Invalid end date: {}", e)))?;

    let mut sql = r#"
        SELECT
            uuid,
            role_id AS role,
            label,
            to_char(start, 'HH24:MI:SS') AS start,
            to_char("end", 'HH24:MI:SS') AS "end",
            money_per_hour,
            pa_value,
            font_color,
            bk_color,
            is_locum,
            published,
            date,
            created_at,
            is_dcc,
            is_spa,
            time_off_category_id AS time_off,
            user_profile_id,
            created_by
        FROM "Shifts"
        WHERE date >= $1 AND date <= $2
    "#
    .to_string();

    if let Some(role_id) = query.role_id {
        sql.push_str(" AND role_id = $3");
    }

    sql.push_str(" ORDER BY date, start");

    let mut query_builder = sqlx::query_as::<_, Shift>(&sql).bind(start_date).bind(end_date);
    if let Some(role_id) = query.role_id {
        query_builder = query_builder.bind(role_id);
    }

    let shifts = query_builder.fetch_all(&state.db).await?;

    Ok(Json(shifts))
}

/// POST /api/shifts - Create a new shift with audit trail
#[utoipa::path(
    post,
    path = "/api/shifts",
    request_body = CreateShiftInput,
    responses(
        (status = 200, description = "Shift created successfully", body = Shift),
        (status = 403, description = "Missing can_edit_rota permission")
    ),
    tag = "shifts",
    security(("cookie_auth" = []))
)]
pub async fn create_shift(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser,
    Json(mut input): Json<CreateShiftInput>,
) -> AppResult<Json<Shift>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_rota").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_rota permission".to_string(),
        ));
    }

    // Set created_by to authenticated user if not specified
    if input.created_by.is_none() {
        input.created_by = Some(auth.profile_id);
    }

    // Generate UUID for new shift
    let shift_uuid = Uuid::new_v4();

    // Convert time strings to TIME format for database
    let start_time = input.start.as_ref().map(|s| format!("{}:00", s));
    let end_time = input.end.as_ref().map(|s| format!("{}:00", s));

    // Insert shift
    let shift = sqlx::query_as::<_, Shift>(
        r#"
        INSERT INTO "Shifts" (
            uuid, role_id, label, start, "end", money_per_hour,
            pa_value, font_color, bk_color, is_locum, published,
            date, is_dcc, is_spa, time_off_category_id,
            user_profile_id, created_by
        )
        VALUES ($1, $2, $3, $4::time, $5::time, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
        RETURNING
            uuid,
            role_id AS role,
            label,
            to_char(start, 'HH24:MI') AS start,
            to_char("end", 'HH24:MI') AS "end",
            money_per_hour,
            pa_value,
            font_color,
            bk_color,
            is_locum,
            published,
            date,
            created_at,
            is_dcc,
            is_spa,
            time_off_category_id AS time_off,
            user_profile_id,
            created_by
        "#,
    )
    .bind(shift_uuid)
    .bind(input.role)
    .bind(&input.label)
    .bind(start_time)
    .bind(end_time)
    .bind(input.money_per_hour)
    .bind(input.pa_value)
    .bind(&input.font_color)
    .bind(&input.bk_color)
    .bind(input.is_locum)
    .bind(input.published)
    .bind(input.date)
    .bind(input.is_dcc)
    .bind(input.is_spa)
    .bind(input.time_off)
    .bind(input.user_profile_id)
    .bind(input.created_by.unwrap())
    .fetch_one(&state.db)
    .await?;

    // Audit trail is automatically created by PostgreSQL triggers
    Ok(Json(shift))
}

/// PUT /api/shifts/{uuid} - Update a shift (audit trail via DB triggers)
#[utoipa::path(
    put,
    path = "/api/shifts/{uuid}",
    params(
        ("uuid" = Uuid, Path, description = "Shift UUID")
    ),
    request_body = UpdateShiftInput,
    responses(
        (status = 200, description = "Shift updated successfully", body = Shift),
        (status = 400, description = "No fields to update"),
        (status = 403, description = "Missing can_edit_rota permission"),
        (status = 404, description = "Shift not found")
    ),
    tag = "shifts",
    security(("cookie_auth" = []))
)]
pub async fn update_shift(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser,
    Path(uuid): Path<Uuid>,
    Json(input): Json<UpdateShiftInput>,
) -> AppResult<Json<Shift>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_rota").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_rota permission".to_string(),
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
    if input.money_per_hour.is_some() {
        updates.push(format!("money_per_hour = ${}", bind_count));
        bind_count += 1;
    }
    if input.pa_value.is_some() {
        updates.push(format!("pa_value = ${}", bind_count));
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
    if input.is_locum.is_some() {
        updates.push(format!("is_locum = ${}", bind_count));
        bind_count += 1;
    }
    if input.published.is_some() {
        updates.push(format!("published = ${}", bind_count));
        bind_count += 1;
    }
    if input.date.is_some() {
        updates.push(format!("date = ${}", bind_count));
        bind_count += 1;
    }
    if input.is_dcc.is_some() {
        updates.push(format!("is_dcc = ${}", bind_count));
        bind_count += 1;
    }
    if input.is_spa.is_some() {
        updates.push(format!("is_spa = ${}", bind_count));
        bind_count += 1;
    }
    if input.time_off.is_some() {
        updates.push(format!("time_off_category_id = ${}", bind_count));
        bind_count += 1;
    }
    if input.user_profile_id.is_some() {
        updates.push(format!("user_profile_id = ${}", bind_count));
        bind_count += 1;
    }

    if updates.is_empty() {
        return Err(AppError::BadRequest("No fields to update".to_string()));
    }

    let sql = format!(
        r#"
        UPDATE "Shifts"
        SET {}
        WHERE uuid = ${}
        RETURNING
            uuid,
            role_id AS role,
            label,
            to_char(start, 'HH24:MI') AS start,
            to_char("end", 'HH24:MI') AS "end",
            money_per_hour,
            pa_value,
            font_color,
            bk_color,
            is_locum,
            published,
            date,
            created_at,
            is_dcc,
            is_spa,
            time_off_category_id AS time_off,
            user_profile_id,
            created_by
        "#,
        updates.join(", "),
        bind_count
    );

    // Build query with bindings
    let mut query = sqlx::query_as::<_, Shift>(&sql);

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
    if let Some(money) = input.money_per_hour {
        query = query.bind(money);
    }
    if let Some(pa) = input.pa_value {
        query = query.bind(pa);
    }
    if let Some(font_color) = &input.font_color {
        query = query.bind(font_color);
    }
    if let Some(bk_color) = &input.bk_color {
        query = query.bind(bk_color);
    }
    if let Some(is_locum) = input.is_locum {
        query = query.bind(is_locum);
    }
    if let Some(published) = input.published {
        query = query.bind(published);
    }
    if let Some(date) = input.date {
        query = query.bind(date);
    }
    if let Some(is_dcc) = input.is_dcc {
        query = query.bind(is_dcc);
    }
    if let Some(is_spa) = input.is_spa {
        query = query.bind(is_spa);
    }
    if let Some(time_off) = input.time_off {
        query = query.bind(time_off);
    }
    if let Some(user_id) = input.user_profile_id {
        query = query.bind(user_id);
    }

    query = query.bind(uuid);

    let updated_shift = query.fetch_one(&state.db).await?;

    // Audit trail is automatically created by PostgreSQL triggers
    Ok(Json(updated_shift))
}

/// DELETE /api/shifts/{uuid} - Delete a shift (audit trail via DB triggers)
#[utoipa::path(
    delete,
    path = "/api/shifts/{uuid}",
    params(
        ("uuid" = Uuid, Path, description = "Shift UUID")
    ),
    responses(
        (status = 200, description = "Shift deleted successfully", body = ShiftMutationResponse),
        (status = 403, description = "Missing can_edit_rota permission"),
        (status = 404, description = "Shift not found")
    ),
    tag = "shifts",
    security(("cookie_auth" = []))
)]
pub async fn delete_shift(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser,
    Path(uuid): Path<Uuid>,
) -> AppResult<Json<ShiftMutationResponse>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_rota").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_rota permission".to_string(),
        ));
    }

    // Delete the shift (audit trail is automatically created by PostgreSQL triggers)
    let result = sqlx::query(r#"DELETE FROM "Shifts" WHERE uuid = $1"#)
        .bind(uuid)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Shift {} not found", uuid)));
    }

    Ok(Json(ShiftMutationResponse {
        success: true,
        shift_uuid: Some(uuid),
        message: Some("Shift deleted successfully".to_string()),
    }))
}