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

#[derive(Debug, Deserialize, IntoParams)]
pub struct DeleteDiaryQuery {
    #[serde(rename = "confirmedUserId")]
    pub confirmed_user_id: Option<i32>,
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
    auth: AuthenticatedUser,
    Query(query): Query<GetDiaryQuery>,
) -> AppResult<Json<Vec<DiaryEntry>>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(
        &state.db, auth.profile_id, auth.is_super_admin, "can_access_diary"
    ).await? {
        return Err(AppError::Forbidden("Missing can_access_diary permission".to_string()));
    }

    // Handle different query combinations
    let entries = match (query.role_id, query.start, query.end) {
        (Some(role_id), Some(start), Some(end)) => {
            let start_date = NaiveDate::parse_from_str(&start, "%Y-%m-%d")
                .map_err(|e| crate::AppError::BadRequest(format!("Invalid start date: {}", e)))?;
            let end_date = NaiveDate::parse_from_str(&end, "%Y-%m-%d")
                .map_err(|e| crate::AppError::BadRequest(format!("Invalid end date: {}", e)))?;
            sqlx::query_as::<sqlx::Postgres, DiaryEntry>(
                r#"
                SELECT d.*, u.short_name
                FROM "Diary" d
                LEFT JOIN "Users" u ON d.user_profile_id = u.user_profile_id
                WHERE d.role_id = $1 AND d.date >= $2 AND d.date <= $3
                ORDER BY d.created_at DESC
                "#
            )
            .bind(role_id)
            .bind(start_date)
            .bind(end_date)
            .fetch_all(&state.db)
            .await?
        }
        (Some(role_id), None, None) => {
            sqlx::query_as::<sqlx::Postgres, DiaryEntry>(
                r#"
                SELECT d.*, u.short_name
                FROM "Diary" d
                LEFT JOIN "Users" u ON d.user_profile_id = u.user_profile_id
                WHERE d.role_id = $1
                ORDER BY d.created_at DESC
                "#
            )
            .bind(role_id)
            .fetch_all(&state.db)
            .await?
        }
        _ => {
            sqlx::query_as::<sqlx::Postgres, DiaryEntry>(
                r#"
                SELECT d.*, u.short_name
                FROM "Diary" d
                LEFT JOIN "Users" u ON d.user_profile_id = u.user_profile_id
                ORDER BY d.created_at DESC
                "#
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
    // Use confirmed user ID if provided (generic account flow), otherwise use authenticated user
    let acting_user_id = input.confirmed_user_id.unwrap_or(auth.profile_id);

    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, acting_user_id, auth.is_super_admin, "can_access_diary").await? {
        return Err(AppError::Forbidden(
            "Missing can_access_diary permission".to_string(),
        ));
    }

    // Set created_by to acting user
    input.created_by = Some(acting_user_id);

    let entry = sqlx::query_as::<_, DiaryEntry>(
        r#"
        INSERT INTO "Diary" (
            role_id, date, entry, al, sl, pl, user_profile_id, created_by, deleted
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, false)
        RETURNING id::int4, role_id, date, entry, al, sl, pl, created_at, user_profile_id, created_by, deleted
        "#,
    )
    .bind(input.role_id)
    .bind(input.date)
    .bind(&input.entry)
    .bind(input.al)
    .bind(input.sl)
    .bind(input.pl)
    .bind(input.user_profile_id)
    .bind(input.created_by.unwrap_or(acting_user_id))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(entry))
}

/// DELETE /api/diary/{id} - Delete a diary entry (hard or soft based on creation time)
/// Logic:
/// - Announcements (no user_profile_id): Always hard delete
/// - Created < 60 minutes ago: Hard delete
/// - Created ≥ 60 minutes ago: Soft delete (set deleted=true)
#[utoipa::path(
    delete,
    path = "/api/diary/{id}",
    params(
        ("id" = i32, Path, description = "Diary entry ID"),
        ("confirmedUserId" = Option<i32>, Query, description = "For generic accounts - PIN-verified user ID")
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
    Query(params): Query<DeleteDiaryQuery>,
    auth: AuthenticatedUser,
) -> AppResult<Json<DiaryMutationResponse>> {
    // Use confirmed user ID if provided (generic account flow), otherwise use authenticated user
    let acting_user_id = params.confirmed_user_id.unwrap_or(auth.profile_id);

    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, acting_user_id, auth.is_super_admin, "can_access_diary").await? {
        return Err(AppError::Forbidden(
            "Missing can_access_diary permission".to_string(),
        ));
    }

    // Fetch entry to check creation time and user_profile_id
    #[derive(sqlx::FromRow)]
    struct DiaryCheck {
        user_profile_id: Option<i32>,
        created_at: chrono::NaiveDateTime,
    }

    let entry = sqlx::query_as::<_, DiaryCheck>(
        r#"SELECT user_profile_id, created_at FROM "Diary" WHERE id = $1"#
    )
    .bind(entry_id)
    .fetch_optional(&state.db)
    .await?;

    let entry = entry.ok_or_else(|| AppError::NotFound(format!(
        "Diary entry {} not found",
        entry_id
    )))?;

    // Decide: hard delete or soft delete
    let should_hard_delete = if entry.user_profile_id.is_none() {
        // Announcements (no user profile) are always hard deleted
        true
    } else {
        // Check if created within last 60 minutes
        use chrono::Utc;
        let now = Utc::now().naive_utc();
        let created = entry.created_at;
        let duration = now.signed_duration_since(created);
        let diff_minutes = duration.num_minutes();
        diff_minutes < 60
    };

    if should_hard_delete {
        // Hard delete
        sqlx::query(r#"DELETE FROM "Diary" WHERE id = $1"#)
            .bind(entry_id)
            .execute(&state.db)
            .await?;
    } else {
        // Soft delete
        sqlx::query(r#"UPDATE "Diary" SET deleted = true WHERE id = $1"#)
            .bind(entry_id)
            .execute(&state.db)
            .await?;
    }

    Ok(Json(DiaryMutationResponse {
        success: true,
        message: Some("Diary entry deleted successfully".to_string()),
    }))
}