use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;


/// Input for creating a diary entry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateDiaryInput {
    pub role_id: i32,
    pub date: NaiveDate,
    pub entry: Option<String>,
    pub al: bool,
    pub sl: bool,
    pub pl: bool,
    pub user_profile_id: Option<i32>,
    pub created_by: Option<i32>, // Will be set to authenticated user
}

/// Response for diary mutations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DiaryMutationResponse {
    pub success: bool,
    pub message: Option<String>,
}
