use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use std::sync::Arc;
use utoipa::IntoParams;

use crate::{
    extractors::{permissions, AuthenticatedUser},
    models::{CreateJobPlanInput, JobPlan, JobPlanMutationResponse, UpdateJobPlanInput},
    AppError, AppResult, AppState,
};

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetJobPlansQuery {
    pub user_profile_id: Option<i32>,
    pub role_id: Option<i32>,
}

/// GET /api/job-plans?user_profile_id=&role_id=
#[utoipa::path(
    get,
    path = "/api/job-plans",
    params(GetJobPlansQuery),
    responses(
        (status = 200, description = "List of job plans", body = Vec<JobPlan>),
        (status = 403, description = "Missing can_edit_staff permission")
    ),
    tag = "job-plans",
    security(("cookie_auth" = []))
)]
pub async fn get_job_plans(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser,
    Query(query): Query<GetJobPlansQuery>,
) -> AppResult<Json<Vec<JobPlan>>> {
    // Check permission
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
            "Missing can_edit_staff permission".to_string(),
        ));
    }

    let mut sql = r#"
        SELECT
            id,
            role_id,
            user_profile_id,
            dcc_pa,
            dcc_hour,
            spa_pa,
            spa_hour,
            al_per_year,
            sl_per_year,
            pl_per_year,
            "from",
            until,
            comment
        FROM "JobPlans"
        WHERE 1=1
    "#
    .to_string();

    let mut bindings: Vec<i32> = vec![];

    if let Some(user_profile_id) = query.user_profile_id {
        sql.push_str(&format!(" AND user_profile_id = ${}", bindings.len() + 1));
        bindings.push(user_profile_id);
    }

    if let Some(role_id) = query.role_id {
        sql.push_str(&format!(" AND role_id = ${}", bindings.len() + 1));
        bindings.push(role_id);
    }

    sql.push_str(" ORDER BY \"from\" DESC");

    let mut query_builder = sqlx::query_as::<_, JobPlan>(&sql);
    for binding in bindings {
        query_builder = query_builder.bind(binding);
    }

    let job_plans = query_builder.fetch_all(&state.db).await?;

    Ok(Json(job_plans))
}

/// POST /api/job-plans - Create a new job plan
#[utoipa::path(
    post,
    path = "/api/job-plans",
    request_body = CreateJobPlanInput,
    responses(
        (status = 200, description = "Job plan created successfully", body = JobPlan),
        (status = 403, description = "Missing can_edit_staff permission")
    ),
    tag = "job-plans",
    security(("cookie_auth" = []))
)]
pub async fn create_job_plan(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser,
    Json(input): Json<CreateJobPlanInput>,
) -> AppResult<Json<JobPlan>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_staff").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_staff permission".to_string(),
        ));
    }

    let job_plan = sqlx::query_as::<_, JobPlan>(
        r#"
        INSERT INTO "JobPlans" (
            role_id, user_profile_id, dcc_pa, dcc_hour, spa_pa, spa_hour,
            al_per_year, sl_per_year, pl_per_year, "from", until, comment
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING
            id::int4,
            role_id,
            user_profile_id,
            dcc_pa,
            dcc_hour,
            spa_pa,
            spa_hour,
            al_per_year,
            sl_per_year,
            pl_per_year,
            "from",
            until,
            comment
        "#,
    )
    .bind(input.role_id)
    .bind(input.user_profile_id)
    .bind(input.dcc_pa)
    .bind(input.dcc_hour)
    .bind(input.spa_pa)
    .bind(input.spa_hour)
    .bind(input.al_per_year)
    .bind(input.sl_per_year)
    .bind(input.pl_per_year)
    .bind(input.from)
    .bind(input.until)
    .bind(&input.comment)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(job_plan))
}

/// PUT /api/job-plans/{id} - Update a job plan
#[utoipa::path(
    put,
    path = "/api/job-plans/{id}",
    params(
        ("id" = i32, Path, description = "Job plan ID")
    ),
    request_body = UpdateJobPlanInput,
    responses(
        (status = 200, description = "Job plan updated successfully", body = JobPlan),
        (status = 400, description = "No fields to update"),
        (status = 403, description = "Missing can_edit_staff permission"),
        (status = 404, description = "Job plan not found")
    ),
    tag = "job-plans",
    security(("cookie_auth" = []))
)]
pub async fn update_job_plan(
    State(state): State<Arc<AppState>>,
    Path(job_plan_id): Path<i32>,
    auth: AuthenticatedUser,
    Json(input): Json<UpdateJobPlanInput>,
) -> AppResult<Json<JobPlan>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_staff").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_staff permission".to_string(),
        ));
    }

    // Build dynamic UPDATE query
    let mut updates = vec![];
    let mut bind_count = 1;

    if input.role_id.is_some() {
        updates.push(format!("role_id = ${}", bind_count));
        bind_count += 1;
    }
    if input.user_profile_id.is_some() {
        updates.push(format!("user_profile_id = ${}", bind_count));
        bind_count += 1;
    }
    if input.dcc_pa.is_some() {
        updates.push(format!("dcc_pa = ${}", bind_count));
        bind_count += 1;
    }
    if input.dcc_hour.is_some() {
        updates.push(format!("dcc_hour = ${}", bind_count));
        bind_count += 1;
    }
    if input.spa_pa.is_some() {
        updates.push(format!("spa_pa = ${}", bind_count));
        bind_count += 1;
    }
    if input.spa_hour.is_some() {
        updates.push(format!("spa_hour = ${}", bind_count));
        bind_count += 1;
    }
    if input.al_per_year.is_some() {
        updates.push(format!("al_per_year = ${}", bind_count));
        bind_count += 1;
    }
    if input.sl_per_year.is_some() {
        updates.push(format!("sl_per_year = ${}", bind_count));
        bind_count += 1;
    }
    if input.pl_per_year.is_some() {
        updates.push(format!("pl_per_year = ${}", bind_count));
        bind_count += 1;
    }
    if input.from.is_some() {
        updates.push(format!("\"from\" = ${}", bind_count));
        bind_count += 1;
    }
    if input.until.is_some() {
        updates.push(format!("until = ${}", bind_count));
        bind_count += 1;
    }
    if input.comment.is_some() {
        updates.push(format!("comment = ${}", bind_count));
        bind_count += 1;
    }

    if updates.is_empty() {
        return Err(AppError::BadRequest("No fields to update".to_string()));
    }

    let sql = format!(
        r#"
        UPDATE "JobPlans"
        SET {}
        WHERE id = ${}
        RETURNING
            id::int4,
            role_id,
            user_profile_id,
            dcc_pa,
            dcc_hour,
            spa_pa,
            spa_hour,
            al_per_year,
            sl_per_year,
            pl_per_year,
            "from",
            until,
            comment
        "#,
        updates.join(", "),
        bind_count
    );

    // Build query with bindings
    let mut query = sqlx::query_as::<_, JobPlan>(&sql);

    if let Some(user_role) = input.role_id {
        query = query.bind(user_role);
    }
    if let Some(user_profile_id) = input.user_profile_id {
        query = query.bind(user_profile_id);
    }
    if let Some(dcc_pa) = input.dcc_pa {
        query = query.bind(dcc_pa);
    }
    if let Some(dcc_hour) = input.dcc_hour {
        query = query.bind(dcc_hour);
    }
    if let Some(spa_pa) = input.spa_pa {
        query = query.bind(spa_pa);
    }
    if let Some(spa_hour) = input.spa_hour {
        query = query.bind(spa_hour);
    }
    if let Some(al) = input.al_per_year {
        query = query.bind(al);
    }
    if let Some(sl) = input.sl_per_year {
        query = query.bind(sl);
    }
    if let Some(pl) = input.pl_per_year {
        query = query.bind(pl);
    }
    if let Some(from) = input.from {
        query = query.bind(from);
    }
    if let Some(until) = input.until {
        query = query.bind(until);
    }
    if let Some(comment) = &input.comment {
        query = query.bind(comment);
    }

    query = query.bind(job_plan_id);

    let updated_plan = query.fetch_one(&state.db).await?;

    Ok(Json(updated_plan))
}

/// DELETE /api/job-plans/{id} - Delete a job plan
#[utoipa::path(
    delete,
    path = "/api/job-plans/{id}",
    params(
        ("id" = i32, Path, description = "Job plan ID")
    ),
    responses(
        (status = 200, description = "Job plan deleted successfully", body = JobPlanMutationResponse),
        (status = 403, description = "Missing can_edit_staff permission"),
        (status = 404, description = "Job plan not found")
    ),
    tag = "job-plans",
    security(("cookie_auth" = []))
)]
pub async fn delete_job_plan(
    State(state): State<Arc<AppState>>,
    Path(job_plan_id): Path<i32>,
    auth: AuthenticatedUser,
) -> AppResult<Json<JobPlanMutationResponse>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_staff").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_staff permission".to_string(),
        ));
    }

    let result = sqlx::query(r#"DELETE FROM "JobPlans" WHERE id = $1"#)
        .bind(job_plan_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "Job plan {} not found",
            job_plan_id
        )));
    }

    Ok(Json(JobPlanMutationResponse {
        success: true,
        message: Some("Job plan deleted successfully".to_string()),
    }))
}

/// POST /api/job-plans/{id}/terminate - Terminate a job plan by setting 'until' to today
#[utoipa::path(
    post,
    path = "/api/job-plans/{id}/terminate",
    params(
        ("id" = i32, Path, description = "Job plan ID")
    ),
    responses(
        (status = 200, description = "Job plan terminated successfully", body = JobPlan),
        (status = 403, description = "Missing can_edit_staff permission"),
        (status = 404, description = "Job plan not found")
    ),
    tag = "job-plans",
    security(("cookie_auth" = []))
)]
pub async fn terminate_job_plan(
    State(state): State<Arc<AppState>>,
    Path(job_plan_id): Path<i32>,
    auth: AuthenticatedUser,
) -> AppResult<Json<JobPlan>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_staff").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_staff permission".to_string(),
        ));
    }

    // Set 'until' to today
    let today = Utc::now().date_naive();

    let updated_plan = sqlx::query_as::<_, JobPlan>(
        r#"
        UPDATE "JobPlans"
        SET until = $1
        WHERE id = $2
        RETURNING
            id::int4,
            role_id,
            user_profile_id,
            dcc_pa,
            dcc_hour,
            spa_pa,
            spa_hour,
            al_per_year,
            sl_per_year,
            pl_per_year,
            "from",
            until,
            comment
        "#,
    )
    .bind(today)
    .bind(job_plan_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(updated_plan))
}