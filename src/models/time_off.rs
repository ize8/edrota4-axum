use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimeOffCategory {
    pub id: i32,
    #[serde(rename = "label")]
    pub label: String,
    pub short_name: String,
    pub font_color: String,
    pub bk_color: String,
}
