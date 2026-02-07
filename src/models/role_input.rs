use serde::{Deserialize, Serialize};

use utoipa::ToSchema;

/// Input for creating a role
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateRoleInput {
    pub workplace_id: i32,
    pub role_name: String,
    #[serde(default)]
    pub marketplace_auto_approve: Option<bool>,
}

/// Input for updating a role
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateRoleInput {
    pub workplace_id: Option<i32>,
    pub role_name: Option<String>,
    pub marketplace_auto_approve: Option<bool>,
}

/// Response for role mutations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoleMutationResponse {
    pub success: bool,
    pub message: Option<String>,
}

/// Input for creating a workplace
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkplaceInput {
    pub hospital: String,  // Required field (not Option)
    pub ward: Option<String>,
    pub address: Option<String>,
    pub code: Option<String>,
}

/// Input for updating a workplace
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateWorkplaceInput {
    pub hospital: Option<String>,
    pub ward: Option<String>,
    pub address: Option<String>,
    pub code: Option<String>,
}

/// Response for workplace mutations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkplaceMutationResponse {
    pub success: bool,
    pub message: Option<String>,
}

/// Dependency count for workplace/role deletion
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DependencyCount {
    pub roles: i32,
    pub user_roles: i32,
    pub job_plans: i32,
    pub shifts: i32,
    pub shift_requests: i32,
    pub templates: i32,
    pub diary_entries: i32,
    pub audit_entries: i32,
    pub cod_entries: i32,
    pub unique_staff: i32,
}
