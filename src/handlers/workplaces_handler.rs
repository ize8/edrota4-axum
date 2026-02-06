use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::{
    extractors::AuthenticatedUser,
    models::{CreateWorkplaceInput, UpdateWorkplaceInput, Workplace, WorkplaceMutationResponse},
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
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_staff").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_staff permission".to_string(),
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
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_staff").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_staff permission".to_string(),
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
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_staff").await? {
        return Err(AppError::Forbidden(
            "Missing can_edit_staff permission".to_string(),
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