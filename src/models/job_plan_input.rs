use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;


/// Input for creating a job plan
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateJobPlanInput {
    pub role_id: i32,  // Database column is role_id, not user_role
    pub user_profile_id: i32,
    pub dcc_pa: Option<f32>,
    pub dcc_hour: Option<f32>,
    pub spa_pa: Option<f32>,
    pub spa_hour: Option<f32>,
    pub al_per_year: f32,
    pub sl_per_year: f32,
    pub pl_per_year: f32,
    pub from: NaiveDate,
    pub until: Option<NaiveDate>,
    pub comment: Option<String>,
}

/// Input for updating a job plan
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateJobPlanInput {
    pub role_id: Option<i32>,  // Database column is role_id, not user_role
    pub user_profile_id: Option<i32>,
    pub dcc_pa: Option<f32>,
    pub dcc_hour: Option<f32>,
    pub spa_pa: Option<f32>,
    pub spa_hour: Option<f32>,
    pub al_per_year: Option<f32>,
    pub sl_per_year: Option<f32>,
    pub pl_per_year: Option<f32>,
    pub from: Option<NaiveDate>,
    pub until: Option<NaiveDate>,
    pub comment: Option<String>,
}

/// Response for job plan mutations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JobPlanMutationResponse {
    pub success: bool,
    pub message: Option<String>,
}
