use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Workplace {
    pub id: i32,  // SERIAL = INT4, not INT8
    pub hospital: Option<String>,
    pub ward: Option<String>,
    pub address: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Role {
    pub id: i32,
    pub workplace: i32,
    pub role_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marketplace_auto_approve: Option<bool>,
    #[serde(rename = "Workplaces")]
    pub workplaces: Option<Workplace>,
}
