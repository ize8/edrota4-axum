use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

use super::role::Role;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct User {
    pub user_profile_id: i32,
    pub auth_id: String,
    pub full_name: String,
    pub short_name: String,
    pub primary_email: Option<String>,
    pub secondary_emails: Option<Vec<String>>,
    pub tel: Option<Vec<String>>,
    pub gmc: Option<i32>,
    pub auth_pin: Option<String>,
    pub is_super_admin: bool,
    pub comment: Option<String>,
    #[serde(serialize_with = "serialize_naive_as_utc")]
    pub created_at: NaiveDateTime,
    pub color: Option<String>,
    pub is_generic_login: bool,
}

fn serialize_naive_as_utc<S>(dt: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let utc_dt = DateTime::<Utc>::from_naive_utc_and_offset(*dt, Utc);
    utc_dt.to_rfc3339().serialize(serializer)
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserRole {
    pub id: i32,
    pub role_id: i32,
    pub user_profile_id: i32,
    pub can_edit_rota: bool,
    pub can_access_diary: bool,
    pub can_work_shifts: bool,
    pub can_edit_templates: bool,
    pub can_edit_staff: bool,
    pub can_view_staff_details: bool,
    #[serde(serialize_with = "serialize_naive_as_utc")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "Roles")]
    pub roles: Option<Role>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct StaffFilterOption {
    pub user_profile_id: i32,
    pub short_name: String,
    pub full_name: String,
    pub color: Option<String>,
}
