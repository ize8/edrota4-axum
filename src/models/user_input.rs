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

// ============================================================================
// New Endpoints - Phase B
// ============================================================================

/// Request for searching users by name or email
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchUsersRequest {
    pub query: String,
    #[serde(rename = "roleId")]
    pub role_id: Option<i32>,
}

/// Request for creating a user profile without Clerk account
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateUserProfileRequest {
    pub full_name: String,
    pub short_name: String,
    pub gmc: Option<i32>,
    pub primary_email: Option<String>,
    pub secondary_emails: Option<Vec<String>>,
    pub tel: Option<Vec<String>>,
    pub comment: Option<String>,
    pub auth_pin: Option<String>,
    pub color: Option<String>,
}

/// Request for checking email availability
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CheckEmailRequest {
    pub email: String,
}

/// Response for email availability check
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CheckEmailResponse {
    pub used_for_login: bool,
    pub used_by_profile: bool,
    pub user_id: Option<i32>,
}

/// Request for verifying identity via PIN (Step 1 of PIN change)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VerifyIdentityRequest {
    pub user_profile_id: i32,
    pub pin: String,
}

/// Response for identity verification (contains token)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VerifyIdentityResponse {
    pub success: bool,
    pub token: Option<String>,
}

/// Request for changing profile PIN with verification token (Step 2)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChangeProfilePinRequest {
    pub verification_token: String,
    pub new_pin: String,
    pub confirm_pin: String,
}

/// Generic success response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SuccessResponse {
    pub success: bool,
}

/// Input for creating a Clerk login for a user profile
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateLoginInput {
    pub email: String,
    pub temp_password: String,
    pub user_profile_id: i32,
    #[serde(default)]
    pub is_generic_login: bool,
    pub pin: Option<String>,
}

/// Response for creating a login
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateLoginResponse {
    pub auth_id: String,
    pub user_id: i32,
    pub is_generic_login: bool,
}

/// Input for changing own password
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChangePasswordInput {
    pub current_password: String,
    pub new_password: String,
    pub confirm_new_password: String,
}
