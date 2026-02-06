use serde::{Deserialize, Serialize};

use utoipa::ToSchema;

/// Input for updating own profile (self-service)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateOwnProfileInput {
    pub short_name: String,
    pub tel: Option<Vec<String>>,
    pub color: Option<String>,
}

/// Input for changing own PIN (self-service)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChangeOwnPinInput {
    pub current_pin: String,
    pub new_pin: String,
    pub confirm_new_pin: String,
}

/// Input for admin updating user profile
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateUserProfileInput {
    pub full_name: Option<String>,
    pub short_name: Option<String>,
    pub gmc: Option<i32>,
    pub primary_email: Option<String>,
    pub secondary_emails: Option<Vec<String>>,
    pub tel: Option<Vec<String>>,
    pub comment: Option<String>,
    pub auth_pin: Option<String>,
    pub color: Option<String>,
}

/// Response for PIN operations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PinResponse {
    pub success: bool,
    pub new_pin: Option<String>, // Only for admin reset
    pub message: Option<String>,
}
