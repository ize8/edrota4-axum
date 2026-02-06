use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use uuid::Uuid;

/// Input DTO for creating a new shift
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateShiftInput {
    pub role: i32,
    pub label: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub money_per_hour: Option<f32>,
    pub pa_value: f32,
    pub font_color: String,
    pub bk_color: String,
    pub is_locum: bool,
    pub published: bool,
    pub date: NaiveDate,
    pub is_dcc: bool,
    pub is_spa: bool,
    pub time_off: Option<i32>,
    pub user_profile_id: Option<i32>,
    pub created_by: Option<i32>, // Optional - will default to authenticated user
}

/// Input DTO for updating an existing shift
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateShiftInput {
    pub role: Option<i32>,
    pub label: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub money_per_hour: Option<f32>,
    pub pa_value: Option<f32>,
    pub font_color: Option<String>,
    pub bk_color: Option<String>,
    pub is_locum: Option<bool>,
    pub published: Option<bool>,
    pub date: Option<NaiveDate>,
    pub is_dcc: Option<bool>,
    pub is_spa: Option<bool>,
    pub time_off: Option<i32>,
    pub user_profile_id: Option<i32>,
}

/// Response after successful mutation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ShiftMutationResponse {
    pub success: bool,
    pub shift_uuid: Option<Uuid>,
    pub message: Option<String>,
}
