use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDate;
use serde::Deserialize;
use std::sync::Arc;
use utoipa::IntoParams;

use crate::{
    extractors::AuthenticatedUser,
    models::{CreateDiaryInput, DiaryEntry, DiaryMutationResponse},
    AppError, AppResult, AppState,
};

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetDiaryQuery {
    #[serde(rename = "roleId")]
    pub role_id: Option<i32>,
    pub start: Option<String>,
    pub end: Option<String>,
}

/// GET /api/diary?roleId=&start=&end=
#[utoipa::path(
    get,
    path = "/api/diary",
    params(GetDiaryQuery),
    responses(
        (status = 200, description = "List of diary entries", body = Vec<DiaryEntry>),
        (status = 400, description = "Invalid date format")
    ),
    tag = "diary"
)]
pub async fn get_diary(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetDiaryQuery>,
) -> AppResult<Json<Vec<DiaryEntry>>> {
    // Handle different query combinations
    let entries = match (query.role_id, query.start, query.end) {
        (Some(role_id), Some(start), Some(end)) => {
            let start_date = NaiveDate::parse_from_str(&start, "%Y-%m-%d")
                .map_err(|e| crate::AppError::BadRequest(format!("Invalid start date: {}", e)))?;
            let end_date = NaiveDate::parse_from_str(&end, "%Y-%m-%d")
                .map_err(|e| crate::AppError::BadRequest(format!("Invalid end date: {}", e)))?;
            sqlx::query_as::<sqlx::Postgres, DiaryEntry>(
                r#"SELECT * FROM "Diary" WHERE deleted = false AND role_id = $1 AND date >= $2 AND date <= $3 ORDER BY date"#
            )
            .bind(role_id)
            .bind(start_date)
            .bind(end_date)
            .fetch_all(&state.db)
            .await?
        }
        (Some(role_id), None, None) => {
            sqlx::query_as::<sqlx::Postgres, DiaryEntry>(
                r#"SELECT * FROM "Diary" WHERE deleted = false AND role_id = $1 ORDER BY date"#
            )
            .bind(role_id)
            .fetch_all(&state.db)
            .await?
        }
        _ => {
            sqlx::query_as::<sqlx::Postgres, DiaryEntry>(
                r#"SELECT * FROM "Diary" WHERE deleted = false ORDER BY date"#
            )
            .fetch_all(&state.db)
            .await?
        }
    };

    Ok(Json(entries))
}

/// POST /api/diary - Create a new diary entry
#[utoipa::path(
    post,
    path = "/api/diary",
    request_body = CreateDiaryInput,
    responses(
        (status = 200, description = "Diary entry created successfully", body = DiaryEntry),
        (status = 403, description = "Missing can_access_diary permission")
    ),
    tag = "diary",
    security(("cookie_auth" = []))
)]
pub async fn create_diary_entry(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser,
    Json(mut input): Json<CreateDiaryInput>,
) -> AppResult<Json<DiaryEntry>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_access_diary").await? {
        return Err(AppError::Forbidden(
            "Missing can_access_diary permission".to_string(),
        ));
    }

    // Set created_by to authenticated user
    input.created_by = Some(auth.profile_id);

    let entry = sqlx::query_as::<_, DiaryEntry>(
        r#"
        INSERT INTO "Diary" (
            role_id, date, entry, al, sl, pl, user_profile_id, created_by, deleted
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, false)
        RETURNING *
        "#,
    )
    .bind(input.role_id)
    .bind(input.date)
    .bind(&input.entry)
    .bind(input.al)
    .bind(input.sl)
    .bind(input.pl)
    .bind(input.user_profile_id)
    .bind(input.created_by.unwrap())
    .fetch_one(&state.db)
    .await?;

    Ok(Json(entry))
}

/// DELETE /api/diary/{id} - Delete a diary entry (soft delete)
#[utoipa::path(
    delete,
    path = "/api/diary/{id}",
    params(
        ("id" = i32, Path, description = "Diary entry ID")
    ),
    responses(
        (status = 200, description = "Diary entry deleted successfully", body = DiaryMutationResponse),
        (status = 403, description = "Missing can_access_diary permission"),
        (status = 404, description = "Diary entry not found")
    ),
    tag = "diary",
    security(("cookie_auth" = []))
)]
pub async fn delete_diary_entry(
    State(state): State<Arc<AppState>>,
    Path(entry_id): Path<i32>,
    auth: AuthenticatedUser,
) -> AppResult<Json<DiaryMutationResponse>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_access_diary").await? {
        return Err(AppError::Forbidden(
            "Missing can_access_diary permission".to_string(),
        ));
    }

    // Soft delete by setting deleted = true
    let result = sqlx::query(r#"UPDATE "Diary" SET deleted = true WHERE id = $1"#)
        .bind(entry_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "Diary entry {} not found",
            entry_id
        )));
    }

    Ok(Json(DiaryMutationResponse {
        success: true,
        message: Some("Diary entry deleted successfully".to_string()),
    }))
}