use serde::{Deserialize, Serialize};

use utoipa::ToSchema;

/// Input for creating a shift template
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTemplateInput {
    pub role: i32,
    pub label: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub pa_value: Option<f32>,
    pub money_per_hour: Option<f32>,
    pub font_color: String,
    pub bk_color: String,
    pub is_spa: bool,
    pub is_dcc: bool,
}

/// Input for updating a shift template
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateTemplateInput {
    pub role: Option<i32>,
    pub label: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub pa_value: Option<f32>,
    pub money_per_hour: Option<f32>,
    pub font_color: Option<String>,
    pub bk_color: Option<String>,
    pub is_spa: Option<bool>,
    pub is_dcc: Option<bool>,
}

/// Response for template mutations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TemplateMutationResponse {
    pub success: bool,
    pub message: Option<String>,
}
