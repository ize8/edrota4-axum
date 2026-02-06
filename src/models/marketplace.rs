use chrono::{DateTime, NaiveDate, Utc};
use utoipa::ToSchema;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ShiftRequest {
    pub id: i32,
    pub shift_id: Uuid,
    pub requester_id: i32,
    #[serde(rename = "type")]
    #[sqlx(rename = "request_type")]
    pub request_type: String,
    pub status: String,
    pub target_user_id: Option<i32>,
    pub target_shift_id: Option<Uuid>,
    pub candidate_id: Option<i32>,
    pub resolved_by: Option<i32>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ShiftRequestWithDetails {
    #[serde(flatten)]
    pub request: ShiftRequest,
    pub shift_date: NaiveDate,
    pub shift_label: String,
    pub shift_start: Option<String>,
    pub shift_end: Option<String>,
    pub shift_role_id: i32,
    pub shift_role_name: String,
    pub shift_user_id: Option<i32>,
    pub requester_name: String,
    pub requester_short_name: String,
    pub target_user_name: Option<String>,
    pub target_user_short_name: Option<String>,
    pub target_shift_date: Option<NaiveDate>,
    pub target_shift_label: Option<String>,
    pub target_shift_start: Option<String>,
    pub target_shift_end: Option<String>,
    pub candidate_name: Option<String>,
    pub candidate_short_name: Option<String>,
    pub role_auto_approve: bool,
}
