use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Workplace {
    pub id: i32,
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
    #[serde(rename = "Workplaces")]
    pub workplaces: Option<Workplace>,
}
