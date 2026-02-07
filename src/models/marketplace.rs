use chrono::{NaiveDate, NaiveDateTime};
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
    #[sqlx(rename = "type")]
    pub request_type: String,
    pub status: String,
    pub target_user_id: Option<i32>,
    pub target_shift_id: Option<Uuid>,
    pub candidate_id: Option<i32>,
    pub resolved_by: Option<i32>,
    pub resolved_at: Option<NaiveDateTime>,
    pub notes: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
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

/// Swappable shift (simplified shift info for marketplace)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SwappableShift {
    pub uuid: String,
    pub date: String,
    #[serde(rename = "startTime")]
    pub start_time: String,
    #[serde(rename = "endTime")]
    pub end_time: String,
    pub label: String,
    #[serde(rename = "isTimeOff")]
    pub is_time_off: bool,
}

/// User with their swappable shifts
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserWithSwappableShifts {
    #[serde(rename = "userId")]
    pub user_id: i32,
    #[serde(rename = "userName")]
    pub user_name: String,
    pub shifts: Vec<SwappableShift>,
}
