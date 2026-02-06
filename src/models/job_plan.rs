use chrono::NaiveDate;
use utoipa::ToSchema;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct JobPlan {
    pub id: i32,
    #[serde(rename = "user_role")]
    #[sqlx(rename = "user_role")]
    pub user_role: i32,
    pub user_profile_id: i32,
    pub dcc_pa: Option<f32>,
    pub dcc_hour: Option<f32>,
    pub spa_pa: Option<f32>,
    pub spa_hour: Option<f32>,
    pub al_per_year: f32,
    pub sl_per_year: f32,
    pub pl_per_year: f32,
    #[sqlx(rename = "from")]
    pub from: NaiveDate,
    pub until: Option<NaiveDate>,
    pub comment: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JobPlanTemplate {
    #[serde(rename = "workplace")]
    pub workplace: i32,
    pub label: String,
    pub al_per_year: Option<f32>,
    pub sl_per_year: Option<f32>,
    pub pl_per_year: Option<f32>,
    pub dcc_pa: Option<f32>,
    pub dcc_hour: Option<f32>,
    pub spa_pa: Option<f32>,
    pub spa_hour: Option<f32>,
}
