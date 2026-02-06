use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{NaiveDate, NaiveDateTime};
use serde::Deserialize;
use sqlx::FromRow;
use std::sync::Arc;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::{
    extractors::{permissions, AuthenticatedUser},
    models::{AcceptRequestInput, AdminDecisionInput, CreateShiftRequestInput, MarketplaceMutationResponse, RespondToProposalInput, ShiftRequestWithDetails},
    AppError, AppResult, AppState,
};

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetMarketplaceQuery {
    #[serde(rename = "roleId")]
    pub role_id: Option<i32>,
    #[serde(rename = "userId")]
    pub user_id: Option<i32>,
    pub month: Option<i32>,
    pub year: Option<i32>,
}

#[derive(Debug, FromRow)]
struct ShiftRequestRow {
    // ShiftRequest fields
    id: i32,
    shift_id: Uuid,
    requester_id: i32,
    request_type: String,
    status: String,
    target_user_id: Option<i32>,
    target_shift_id: Option<Uuid>,
    candidate_id: Option<i32>,
    resolved_by: Option<i32>,
    resolved_at: Option<NaiveDateTime>,
    notes: Option<String>,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    // Enriched fields
    shift_date: NaiveDate,
    shift_label: String,
    shift_start: Option<String>,
    shift_end: Option<String>,
    shift_role_id: i32,
    shift_role_name: String,
    shift_user_id: Option<i32>,
    requester_name: String,
    requester_short_name: String,
    target_user_name: Option<String>,
    target_user_short_name: Option<String>,
    target_shift_date: Option<NaiveDate>,
    target_shift_label: Option<String>,
    target_shift_start: Option<String>,
    target_shift_end: Option<String>,
    candidate_name: Option<String>,
    candidate_short_name: Option<String>,
    role_auto_approve: bool,
}

const MARKETPLACE_BASE_QUERY: &str = r#"
    SELECT
        sr.id,
        sr.shift_id,
        sr.requester_id,
        sr.type AS request_type,
        sr.status,
        sr.target_user_id,
        sr.target_shift_id,
        sr.candidate_id,
        sr.resolved_by,
        sr.resolved_at,
        sr.notes,
        sr.created_at,
        sr.updated_at,
        s.date AS shift_date,
        s.label AS shift_label,
        to_char(s.start, 'HH24:MI') AS shift_start,
        to_char(s."end", 'HH24:MI') AS shift_end,
        s.role_id AS shift_role_id,
        r.role_name AS shift_role_name,
        s.user_profile_id AS shift_user_id,
        u_req.full_name AS requester_name,
        u_req.short_name AS requester_short_name,
        u_target.full_name AS target_user_name,
        u_target.short_name AS target_user_short_name,
        ts.date AS target_shift_date,
        ts.label AS target_shift_label,
        to_char(ts.start, 'HH24:MI') AS target_shift_start,
        to_char(ts."end", 'HH24:MI') AS target_shift_end,
        u_cand.full_name AS candidate_name,
        u_cand.short_name AS candidate_short_name,
        r.marketplace_auto_approve AS role_auto_approve
    FROM "ShiftRequests" sr
    INNER JOIN "Shifts" s ON sr.shift_id = s.uuid
    INNER JOIN "Roles" r ON s.role_id = r.id
    INNER JOIN "Users" u_req ON sr.requester_id = u_req.user_profile_id
    LEFT JOIN "Users" u_target ON sr.target_user_id = u_target.user_profile_id
    LEFT JOIN "Shifts" ts ON sr.target_shift_id = ts.uuid
    LEFT JOIN "Users" u_cand ON sr.candidate_id = u_cand.user_profile_id
"#;

fn row_to_shift_request_with_details(row: ShiftRequestRow) -> ShiftRequestWithDetails {
    use crate::models::ShiftRequest;

    ShiftRequestWithDetails {
        request: ShiftRequest {
            id: row.id,
            shift_id: row.shift_id,
            requester_id: row.requester_id,
            request_type: row.request_type,
            status: row.status,
            target_user_id: row.target_user_id,
            target_shift_id: row.target_shift_id,
            candidate_id: row.candidate_id,
            resolved_by: row.resolved_by,
            resolved_at: row.resolved_at.map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc)),
            notes: row.notes,
            created_at: chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(row.created_at, chrono::Utc),
            updated_at: chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(row.updated_at, chrono::Utc),
        },
        shift_date: row.shift_date,
        shift_label: row.shift_label,
        shift_start: row.shift_start,
        shift_end: row.shift_end,
        shift_role_id: row.shift_role_id,
        shift_role_name: row.shift_role_name,
        shift_user_id: row.shift_user_id,
        requester_name: row.requester_name,
        requester_short_name: row.requester_short_name,
        target_user_name: row.target_user_name,
        target_user_short_name: row.target_user_short_name,
        target_shift_date: row.target_shift_date,
        target_shift_label: row.target_shift_label,
        target_shift_start: row.target_shift_start,
        target_shift_end: row.target_shift_end,
        candidate_name: row.candidate_name,
        candidate_short_name: row.candidate_short_name,
        role_auto_approve: row.role_auto_approve,
    }
}

/// GET /api/marketplace/open?roleId=
#[utoipa::path(
    get,
    path = "/api/marketplace/open",
    params(GetMarketplaceQuery),
    responses(
        (status = 200, description = "List of open shift requests available for acceptance", body = Vec<ShiftRequestWithDetails>)
    ),
    tag = "marketplace"
)]
pub async fn get_open_requests(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetMarketplaceQuery>,
) -> AppResult<Json<Vec<ShiftRequestWithDetails>>> {
    let mut sql = format!("{} WHERE sr.status = 'OPEN'", MARKETPLACE_BASE_QUERY);

    // Build query with parameterized filters
    let rows = if let Some(role_id) = query.role_id {
        sql.push_str(" AND s.role_id = $1 ORDER BY sr.created_at DESC");
        sqlx::query_as::<sqlx::Postgres, ShiftRequestRow>(&sql)
            .bind(role_id)
            .fetch_all(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, role_id, "Failed to fetch open requests");
                e
            })?
    } else {
        sql.push_str(" ORDER BY sr.created_at DESC");
        sqlx::query_as::<sqlx::Postgres, ShiftRequestRow>(&sql)
            .fetch_all(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch open requests");
                e
            })?
    };

    tracing::debug!(count = rows.len(), "Fetched open shift requests");
    let requests = rows.into_iter().map(row_to_shift_request_with_details).collect();
    Ok(Json(requests))
}

/// GET /api/marketplace/my?userId=
#[utoipa::path(
    get,
    path = "/api/marketplace/my",
    params(GetMarketplaceQuery),
    responses(
        (status = 200, description = "List of shift requests created by the user", body = Vec<ShiftRequestWithDetails>),
        (status = 400, description = "userId required")
    ),
    tag = "marketplace"
)]
pub async fn get_my_requests(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetMarketplaceQuery>,
) -> AppResult<Json<Vec<ShiftRequestWithDetails>>> {
    let user_id = query.user_id.ok_or_else(|| {
        tracing::warn!("get_my_requests called without userId");
        AppError::BadRequest("userId required".to_string())
    })?;

    let sql = format!(
        "{} WHERE sr.requester_id = $1 ORDER BY sr.created_at DESC",
        MARKETPLACE_BASE_QUERY
    );

    let rows = sqlx::query_as::<sqlx::Postgres, ShiftRequestRow>(&sql)
        .bind(user_id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id, "Failed to fetch user's shift requests");
            e
        })?;

    tracing::debug!(user_id, count = rows.len(), "Fetched user's shift requests");
    let requests = rows.into_iter().map(row_to_shift_request_with_details).collect();
    Ok(Json(requests))
}

/// GET /api/marketplace/incoming?userId=
#[utoipa::path(
    get,
    path = "/api/marketplace/incoming",
    params(GetMarketplaceQuery),
    responses(
        (status = 200, description = "List of shift requests incoming to the user (proposed or peer accepted)", body = Vec<ShiftRequestWithDetails>),
        (status = 400, description = "userId required")
    ),
    tag = "marketplace"
)]
pub async fn get_incoming_requests(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetMarketplaceQuery>,
) -> AppResult<Json<Vec<ShiftRequestWithDetails>>> {
    let user_id = query.user_id.ok_or_else(|| {
        tracing::warn!("get_incoming_requests called without userId");
        AppError::BadRequest("userId required".to_string())
    })?;

    let sql = format!(
        "{} WHERE sr.target_user_id = $1 AND sr.status IN ('PROPOSED', 'PEER_ACCEPTED') ORDER BY sr.created_at DESC",
        MARKETPLACE_BASE_QUERY
    );

    let rows = sqlx::query_as::<sqlx::Postgres, ShiftRequestRow>(&sql)
        .bind(user_id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id, "Failed to fetch incoming requests");
            e
        })?;

    tracing::debug!(user_id, count = rows.len(), "Fetched incoming shift requests");
    let requests = rows.into_iter().map(row_to_shift_request_with_details).collect();
    Ok(Json(requests))
}

/// GET /api/marketplace/approvals?roleId=
#[utoipa::path(
    get,
    path = "/api/marketplace/approvals",
    params(GetMarketplaceQuery),
    responses(
        (status = 200, description = "List of shift requests pending admin approval", body = Vec<ShiftRequestWithDetails>),
        (status = 403, description = "Missing can_edit_rota permission")
    ),
    tag = "marketplace",
    security(("cookie_auth" = []))
)]
pub async fn get_approval_requests(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser,
    Query(query): Query<GetMarketplaceQuery>,
) -> AppResult<Json<Vec<ShiftRequestWithDetails>>> {
    // Check permission
    let has_perm = permissions::has_permission(
        &state.db,
        auth.profile_id,
        auth.is_super_admin,
        permissions::can_edit_rota,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, profile_id = auth.profile_id, "Permission check failed");
        AppError::Internal(format!("Permission check failed for user {}: {}", auth.profile_id, e))
    })?;

    if !has_perm {
        tracing::warn!(profile_id = auth.profile_id, "User attempted to access approval requests without permission");
        return Err(AppError::Forbidden("Missing can_edit_rota permission".to_string()));
    }

    let mut sql = format!("{} WHERE sr.status = 'PENDING_APPROVAL'", MARKETPLACE_BASE_QUERY);

    // Build query with parameterized filters
    let rows = if let Some(role_id) = query.role_id {
        sql.push_str(" AND s.role_id = $1 ORDER BY sr.created_at ASC");
        sqlx::query_as::<sqlx::Postgres, ShiftRequestRow>(&sql)
            .bind(role_id)
            .fetch_all(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, role_id, "Failed to fetch approval requests");
                e
            })?
    } else {
        sql.push_str(" ORDER BY sr.created_at ASC");
        sqlx::query_as::<sqlx::Postgres, ShiftRequestRow>(&sql)
            .fetch_all(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch approval requests");
                e
            })?
    };

    tracing::debug!(profile_id = auth.profile_id, count = rows.len(), "Fetched approval requests");
    let requests = rows.into_iter().map(row_to_shift_request_with_details).collect();
    Ok(Json(requests))
}

/// GET /api/marketplace/dashboard?userId=
#[utoipa::path(
    get,
    path = "/api/marketplace/dashboard",
    params(GetMarketplaceQuery),
    responses(
        (status = 200, description = "Dashboard counts for open, my, and incoming requests"),
        (status = 400, description = "userId required")
    ),
    tag = "marketplace"
)]
pub async fn get_dashboard(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetMarketplaceQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = query.user_id.ok_or_else(|| AppError::BadRequest("userId required".to_string()))?;

    // Get counts for dashboard
    let open_count: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM "ShiftRequests" sr INNER JOIN "Shifts" s ON sr.shift_id = s.uuid WHERE sr.status = 'OPEN'"#
    )
    .fetch_one(&state.db)
    .await?;

    let my_count: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM "ShiftRequests" WHERE requester_id = $1"#
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    let incoming_count: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM "ShiftRequests" WHERE target_user_id = $1 AND status IN ('PROPOSED', 'PEER_ACCEPTED')"#
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "open": open_count,
        "my": my_count,
        "incoming": incoming_count
    })))
}

/// GET /api/marketplace/swappable?roleId=&month=&year=
#[utoipa::path(
    get,
    path = "/api/marketplace/swappable",
    params(GetMarketplaceQuery),
    responses(
        (status = 200, description = "List of shifts available for swapping (assigned and published)", body = Vec<crate::models::Shift>),
        (status = 400, description = "roleId, month, and year required")
    ),
    tag = "marketplace"
)]
pub async fn get_swappable_shifts(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetMarketplaceQuery>,
) -> AppResult<Json<Vec<crate::models::Shift>>> {
    let role_id = query.role_id.ok_or_else(|| AppError::BadRequest("roleId required".to_string()))?;
    let month = query.month.ok_or_else(|| AppError::BadRequest("month required".to_string()))?;
    let year = query.year.ok_or_else(|| AppError::BadRequest("year required".to_string()))?;

    let shifts = sqlx::query_as::<sqlx::Postgres, crate::models::Shift>(
        r#"
        SELECT
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
        FROM "Shifts"
        WHERE role_id = $1
        AND EXTRACT(YEAR FROM date) = $2
        AND EXTRACT(MONTH FROM date) = $3
        AND user_profile_id IS NOT NULL
        AND published = true
        ORDER BY date, start
        "#
    )
    .bind(role_id)
    .bind(year)
    .bind(month)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(shifts))
}

/// POST /api/marketplace/requests - Create a new shift swap request
#[utoipa::path(
    post,
    path = "/api/marketplace/requests",
    request_body = CreateShiftRequestInput,
    responses(
        (status = 200, description = "Shift request created successfully", body = ShiftRequestWithDetails),
        (status = 400, description = "Invalid request_type or missing target_user_id for SWAP"),
        (status = 403, description = "You can only create requests for your own shifts"),
        (status = 404, description = "Shift not found")
    ),
    tag = "marketplace",
    security(("cookie_auth" = []))
)]
pub async fn create_shift_request(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedUser,
    Json(input): Json<CreateShiftRequestInput>,
) -> AppResult<Json<ShiftRequestWithDetails>> {
    // Verify the shift exists and belongs to the requester
    let shift: (Option<i32>,) = sqlx::query_as(
        r#"SELECT user_profile_id FROM "Shifts" WHERE uuid = $1"#
    )
    .bind(input.shift_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Shift {} not found", input.shift_id)))?;

    if shift.0 != Some(auth.profile_id) {
        return Err(AppError::Forbidden("You can only create requests for your own shifts".to_string()));
    }

    // Determine initial status based on request type
    let status = if input.request_type == "SWAP" && input.target_user_id.is_some() {
        "PROPOSED"
    } else if input.request_type == "GIVE_AWAY" {
        "OPEN"
    } else {
        return Err(AppError::BadRequest("Invalid request_type or missing target_user_id for SWAP".to_string()));
    };

    // Insert the new shift request
    let request_id: i32 = sqlx::query_scalar(
        r#"
        INSERT INTO "ShiftRequests" (
            shift_id, requester_id, type, status, target_user_id, target_shift_id, notes
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id
        "#,
    )
    .bind(input.shift_id)
    .bind(auth.profile_id)
    .bind(&input.request_type)
    .bind(status)
    .bind(input.target_user_id)
    .bind(input.target_shift_id)
    .bind(&input.notes)
    .fetch_one(&state.db)
    .await?;

    // Fetch the created request with full details
    let request = fetch_shift_request_with_details(&state.db, request_id).await?;

    Ok(Json(request))
}

/// POST /api/marketplace/requests/{id}/accept - Accept an OPEN request
#[utoipa::path(
    post,
    path = "/api/marketplace/requests/{id}/accept",
    params(
        ("id" = i32, Path, description = "Shift request ID")
    ),
    request_body = AcceptRequestInput,
    responses(
        (status = 200, description = "Request accepted, may be auto-approved or pending approval", body = ShiftRequestWithDetails),
        (status = 400, description = "Request is not OPEN or cannot accept your own request"),
        (status = 404, description = "Request not found")
    ),
    tag = "marketplace",
    security(("cookie_auth" = []))
)]
pub async fn accept_shift_request(
    State(state): State<Arc<AppState>>,
    Path(request_id): Path<i32>,
    auth: AuthenticatedUser,
    Json(input): Json<AcceptRequestInput>,
) -> AppResult<Json<ShiftRequestWithDetails>> {
    // Fetch the current request
    let (current_status, requester_id, shift_id): (String, i32, Uuid) = sqlx::query_as(
        r#"SELECT status, requester_id, shift_id FROM "ShiftRequests" WHERE id = $1"#
    )
    .bind(request_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Request {} not found", request_id)))?;

    // Validate request is OPEN
    if current_status != "OPEN" {
        return Err(AppError::BadRequest(format!("Request is not OPEN, current status: {}", current_status)));
    }

    // Requester cannot accept their own request
    if requester_id == auth.profile_id {
        return Err(AppError::BadRequest("Cannot accept your own request".to_string()));
    }

    // Check if role has auto-approve enabled
    let auto_approve: bool = sqlx::query_scalar(
        r#"
        SELECT r.marketplace_auto_approve
        FROM "Shifts" s
        INNER JOIN "Roles" r ON s.role_id = r.id
        WHERE s.uuid = $1
        "#
    )
    .bind(shift_id)
    .fetch_one(&state.db)
    .await?;

    // Determine new status
    let new_status = if auto_approve { "APPROVED" } else { "PENDING_APPROVAL" };

    // Start transaction for potential shift swap
    let mut tx = state.db.begin().await?;

    // Update request
    sqlx::query(
        r#"
        UPDATE "ShiftRequests"
        SET candidate_id = $1, target_shift_id = $2, status = $3, updated_at = NOW()
        WHERE id = $4
        "#
    )
    .bind(auth.profile_id)
    .bind(input.target_shift_id)
    .bind(new_status)
    .bind(request_id)
    .execute(&mut *tx)
    .await?;

    // If auto-approve, perform the swap immediately
    if auto_approve {
        tracing::info!(
            request_id,
            shift_id = %shift_id,
            candidate_id = auth.profile_id,
            "Auto-approving shift request and performing swap"
        );
        perform_shift_swap(&mut tx, shift_id, auth.profile_id, input.target_shift_id, requester_id).await?;

        // Mark as resolved
        sqlx::query(r#"UPDATE "ShiftRequests" SET resolved_by = $1, resolved_at = NOW() WHERE id = $2"#)
            .bind(auth.profile_id)
            .bind(request_id)
            .execute(&mut *tx)
            .await?;
    } else {
        tracing::info!(
            request_id,
            candidate_id = auth.profile_id,
            "Request accepted, pending admin approval"
        );
    }

    tx.commit().await.map_err(|e| {
        tracing::error!(
            error = %e,
            request_id,
            auto_approve,
            "Transaction rollback in accept_shift_request"
        );
        AppError::Internal(format!("Failed to commit shift request acceptance for request {}: {}", request_id, e))
    })?;

    tracing::debug!(request_id, "Shift request transaction committed successfully");

    // Fetch updated request
    let request = fetch_shift_request_with_details(&state.db, request_id).await?;

    Ok(Json(request))
}

/// POST /api/marketplace/requests/{id}/respond - Target user responds to PROPOSED swap
#[utoipa::path(
    post,
    path = "/api/marketplace/requests/{id}/respond",
    params(
        ("id" = i32, Path, description = "Shift request ID")
    ),
    request_body = RespondToProposalInput,
    responses(
        (status = 200, description = "Response processed, may be auto-approved, rejected, or pending approval", body = ShiftRequestWithDetails),
        (status = 400, description = "Request is not PROPOSED"),
        (status = 403, description = "You are not the target of this proposal"),
        (status = 404, description = "Request not found")
    ),
    tag = "marketplace",
    security(("cookie_auth" = []))
)]
pub async fn respond_to_proposal(
    State(state): State<Arc<AppState>>,
    Path(request_id): Path<i32>,
    auth: AuthenticatedUser,
    Json(input): Json<RespondToProposalInput>,
) -> AppResult<Json<ShiftRequestWithDetails>> {
    // Fetch the current request
    let (current_status, target_user_id, requester_id, shift_id, target_shift_id): (String, Option<i32>, i32, Uuid, Option<Uuid>) = sqlx::query_as(
        r#"SELECT status, target_user_id, requester_id, shift_id, target_shift_id FROM "ShiftRequests" WHERE id = $1"#
    )
    .bind(request_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Request {} not found", request_id)))?;

    // Validate request is PROPOSED
    if current_status != "PROPOSED" {
        return Err(AppError::BadRequest(format!("Request is not PROPOSED, current status: {}", current_status)));
    }

    // Validate user is the target
    if target_user_id != Some(auth.profile_id) {
        return Err(AppError::Forbidden("You are not the target of this proposal".to_string()));
    }

    if input.accept {
        // Check if role has auto-approve enabled
        let auto_approve: bool = sqlx::query_scalar(
            r#"
            SELECT r.marketplace_auto_approve
            FROM "Shifts" s
            INNER JOIN "Roles" r ON s.role_id = r.id
            WHERE s.uuid = $1
            "#
        )
        .bind(shift_id)
        .fetch_one(&state.db)
        .await?;

        let new_status = if auto_approve { "APPROVED" } else { "PENDING_APPROVAL" };

        // Start transaction
        let mut tx = state.db.begin().await?;

        // Update request status
        sqlx::query(
            r#"
            UPDATE "ShiftRequests"
            SET candidate_id = $1, status = $2, updated_at = NOW()
            WHERE id = $3
            "#
        )
        .bind(auth.profile_id)
        .bind(new_status)
        .bind(request_id)
        .execute(&mut *tx)
        .await?;

        // If auto-approve, perform the swap immediately
        if auto_approve {
            tracing::info!(
                request_id,
                shift_id = %shift_id,
                target_user_id = auth.profile_id,
                "Target user accepted proposal, auto-approving swap"
            );
            perform_shift_swap(&mut tx, shift_id, auth.profile_id, target_shift_id, requester_id).await?;

            // Mark as resolved
            sqlx::query(r#"UPDATE "ShiftRequests" SET resolved_by = $1, resolved_at = NOW() WHERE id = $2"#)
                .bind(auth.profile_id)
                .bind(request_id)
                .execute(&mut *tx)
                .await?;
        } else {
            tracing::info!(
                request_id,
                target_user_id = auth.profile_id,
                "Target user accepted proposal, pending admin approval"
            );
        }

        tx.commit().await.map_err(|e| {
            tracing::error!(
                error = %e,
                request_id,
                auto_approve,
                "Transaction rollback in respond_to_proposal (accept)"
            );
            AppError::Internal(format!("Failed to commit proposal response for request {}: {}", request_id, e))
        })?;
    } else {
        tracing::info!(
            request_id,
            target_user_id = auth.profile_id,
            "Target user rejected proposal"
        );

        // Rejected by target user
        sqlx::query(
            r#"
            UPDATE "ShiftRequests"
            SET status = 'REJECTED', resolved_by = $1, resolved_at = NOW(), updated_at = NOW()
            WHERE id = $2
            "#
        )
        .bind(auth.profile_id)
        .bind(request_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                request_id,
                target_user_id = auth.profile_id,
                "Failed to reject proposal"
            );
            e
        })?;
    }

    // Fetch updated request
    let request = fetch_shift_request_with_details(&state.db, request_id).await?;

    Ok(Json(request))
}

/// POST /api/marketplace/requests/{id}/admin-decision - Admin approves or rejects
#[utoipa::path(
    post,
    path = "/api/marketplace/requests/{id}/admin-decision",
    params(
        ("id" = i32, Path, description = "Shift request ID")
    ),
    request_body = AdminDecisionInput,
    responses(
        (status = 200, description = "Admin decision processed, shift swap performed if approved", body = ShiftRequestWithDetails),
        (status = 400, description = "Request is not PENDING_APPROVAL or has no candidate"),
        (status = 403, description = "Missing can_edit_rota permission"),
        (status = 404, description = "Request not found")
    ),
    tag = "marketplace",
    security(("cookie_auth" = []))
)]
pub async fn admin_decision(
    State(state): State<Arc<AppState>>,
    Path(request_id): Path<i32>,
    auth: AuthenticatedUser,
    Json(input): Json<AdminDecisionInput>,
) -> AppResult<Json<ShiftRequestWithDetails>> {
    // Check permission
    if !crate::extractors::permissions::has_permission_by_name(&state.db, auth.profile_id, auth.is_super_admin, "can_edit_rota").await? {
        return Err(AppError::Forbidden("Missing can_edit_rota permission".to_string()));
    }

    // Fetch the current request
    let (current_status, shift_id, candidate_id, target_shift_id, requester_id): (String, Uuid, Option<i32>, Option<Uuid>, i32) = sqlx::query_as(
        r#"SELECT status, shift_id, candidate_id, target_shift_id, requester_id FROM "ShiftRequests" WHERE id = $1"#
    )
    .bind(request_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Request {} not found", request_id)))?;

    // Validate request is PENDING_APPROVAL
    if current_status != "PENDING_APPROVAL" {
        return Err(AppError::BadRequest(format!("Request is not PENDING_APPROVAL, current status: {}", current_status)));
    }

    let candidate_id = candidate_id.ok_or_else(|| AppError::BadRequest("Request has no candidate".to_string()))?;

    if input.approve {
        tracing::info!(
            request_id,
            shift_id = %shift_id,
            candidate_id,
            admin_id = auth.profile_id,
            "Admin approving shift request"
        );

        // Start transaction
        let mut tx = state.db.begin().await?;

        // Perform the swap
        perform_shift_swap(&mut tx, shift_id, candidate_id, target_shift_id, requester_id).await?;

        // Update request status
        sqlx::query(
            r#"
            UPDATE "ShiftRequests"
            SET status = 'APPROVED', resolved_by = $1, resolved_at = NOW(), notes = $2, updated_at = NOW()
            WHERE id = $3
            "#
        )
        .bind(auth.profile_id)
        .bind(&input.notes)
        .bind(request_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await.map_err(|e| {
            tracing::error!(
                error = %e,
                request_id,
                admin_id = auth.profile_id,
                "Transaction rollback in admin_decision (approve)"
            );
            AppError::Internal(format!("Failed to commit admin approval for request {}: {}", request_id, e))
        })?;

        tracing::info!(request_id, "Admin approval transaction committed successfully");
    } else {
        tracing::info!(
            request_id,
            admin_id = auth.profile_id,
            "Admin rejecting shift request"
        );

        // Rejected by admin
        sqlx::query(
            r#"
            UPDATE "ShiftRequests"
            SET status = 'REJECTED', resolved_by = $1, resolved_at = NOW(), notes = $2, updated_at = NOW()
            WHERE id = $3
            "#
        )
        .bind(auth.profile_id)
        .bind(&input.notes)
        .bind(request_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                request_id,
                admin_id = auth.profile_id,
                "Failed to reject shift request"
            );
            e
        })?;

        tracing::info!(request_id, "Shift request rejected successfully");
    }

    // Fetch updated request
    let request = fetch_shift_request_with_details(&state.db, request_id).await?;

    Ok(Json(request))
}

/// DELETE /api/marketplace/requests/{id} - Cancel a request
#[utoipa::path(
    delete,
    path = "/api/marketplace/requests/{id}",
    params(
        ("id" = i32, Path, description = "Shift request ID")
    ),
    responses(
        (status = 200, description = "Request cancelled successfully", body = MarketplaceMutationResponse),
        (status = 400, description = "Cannot cancel request with current status"),
        (status = 403, description = "You can only cancel your own requests"),
        (status = 404, description = "Request not found")
    ),
    tag = "marketplace",
    security(("cookie_auth" = []))
)]
pub async fn cancel_shift_request(
    State(state): State<Arc<AppState>>,
    Path(request_id): Path<i32>,
    auth: AuthenticatedUser,
) -> AppResult<Json<MarketplaceMutationResponse>> {
    // Fetch the current request
    let (current_status, requester_id): (String, i32) = sqlx::query_as(
        r#"SELECT status, requester_id FROM "ShiftRequests" WHERE id = $1"#
    )
    .bind(request_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Request {} not found", request_id)))?;

    // Only requester can cancel
    if requester_id != auth.profile_id {
        return Err(AppError::Forbidden("You can only cancel your own requests".to_string()));
    }

    // Cannot cancel if already resolved
    if current_status == "APPROVED" || current_status == "REJECTED" || current_status == "CANCELLED" {
        return Err(AppError::BadRequest(format!("Cannot cancel request with status: {}", current_status)));
    }

    // Update request status to CANCELLED
    sqlx::query(
        r#"
        UPDATE "ShiftRequests"
        SET status = 'CANCELLED', resolved_by = $1, resolved_at = NOW(), updated_at = NOW()
        WHERE id = $2
        "#
    )
    .bind(auth.profile_id)
    .bind(request_id)
    .execute(&state.db)
    .await?;

    Ok(Json(MarketplaceMutationResponse {
        success: true,
        message: Some("Request cancelled successfully".to_string()),
    }))
}

/// Helper function to perform the actual shift swap in a transaction
async fn perform_shift_swap(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    shift_id: Uuid,
    new_owner_id: i32,
    target_shift_id: Option<Uuid>,
    original_owner_id: i32,
) -> AppResult<()> {
    // Assign the original shift to the new owner
    sqlx::query(r#"UPDATE "Shifts" SET user_profile_id = $1 WHERE uuid = $2"#)
        .bind(new_owner_id)
        .bind(shift_id)
        .execute(&mut **tx)
        .await?;

    // If there's a target shift (for swaps), assign it to the original owner
    if let Some(target_shift_id) = target_shift_id {
        sqlx::query(r#"UPDATE "Shifts" SET user_profile_id = $1 WHERE uuid = $2"#)
            .bind(original_owner_id)
            .bind(target_shift_id)
            .execute(&mut **tx)
            .await?;
    }

    Ok(())
}

/// Helper function to check if user has a specific permission
/// Helper function to fetch a shift request by ID with full details
async fn fetch_shift_request_with_details(
    db: &sqlx::PgPool,
    request_id: i32,
) -> AppResult<ShiftRequestWithDetails> {
    let row = sqlx::query_as::<_, ShiftRequestRow>(&format!(
        "{} WHERE sr.id = $1",
        MARKETPLACE_BASE_QUERY
    ))
    .bind(request_id)
    .fetch_one(db)
    .await?;

    Ok(row_to_shift_request_with_details(row))
}
