use moka::future::Cache;
use once_cell::sync::Lazy;
use std::time::Duration;

// Cache UserRoleRows per profile_id (30-second TTL)
static ROLES_CACHE: Lazy<Cache<i32, Vec<UserRoleRow>>> = Lazy::new(|| {
    Cache::builder()
        .time_to_live(Duration::from_secs(30))
        .max_capacity(1_000)
        .build()
});

/// Fetch user roles with caching
async fn get_cached_roles(db: &sqlx::PgPool, profile_id: i32) -> Result<Vec<UserRoleRow>, sqlx::Error> {
    if let Some(cached) = ROLES_CACHE.get(&profile_id).await {
        return Ok(cached);
    }

    let roles = sqlx::query_as::<_, UserRoleRow>(
        r#"SELECT * FROM "UserRoles" WHERE user_profile_id = $1"#,
    )
    .bind(profile_id)
    .fetch_all(db)
    .await?;

    ROLES_CACHE.insert(profile_id, roles.clone()).await;
    Ok(roles)
}

/// Check if user has the required permission
pub async fn has_permission(
    db: &sqlx::PgPool,
    profile_id: i32,
    is_super_admin: bool,
    permission_check: impl Fn(&UserRoleRow) -> bool,
) -> Result<bool, sqlx::Error> {
    if is_super_admin {
        return Ok(true);
    }

    let roles = get_cached_roles(db, profile_id).await?;
    Ok(roles.iter().any(permission_check))
}

/// Check if user has any of the specified permissions
pub async fn has_any_permission(
    db: &sqlx::PgPool,
    profile_id: i32,
    is_super_admin: bool,
    checks: &[fn(&UserRoleRow) -> bool],
) -> Result<bool, sqlx::Error> {
    if is_super_admin {
        return Ok(true);
    }

    let roles = get_cached_roles(db, profile_id).await?;

    for check in checks {
        if roles.iter().any(check) {
            return Ok(true);
        }
    }

    Ok(false)
}

#[derive(sqlx::FromRow, Clone)]
pub struct UserRoleRow {
    pub id: i32,
    pub role_id: i32,
    pub user_profile_id: i32,
    pub can_edit_rota: bool,
    pub can_access_diary: bool,
    pub can_work_shifts: bool,
    pub can_edit_templates: bool,
    pub can_edit_staff: bool,
    pub can_view_staff_details: bool,
}

// Permission check functions
pub fn can_edit_rota(role: &UserRoleRow) -> bool {
    role.can_edit_rota
}

pub fn can_access_diary(role: &UserRoleRow) -> bool {
    role.can_access_diary
}

pub fn can_work_shifts(role: &UserRoleRow) -> bool {
    role.can_work_shifts
}

pub fn can_edit_templates(role: &UserRoleRow) -> bool {
    role.can_edit_templates
}

pub fn can_edit_staff(role: &UserRoleRow) -> bool {
    role.can_edit_staff
}

pub fn can_view_staff_details(role: &UserRoleRow) -> bool {
    role.can_view_staff_details
}

/// Check if user has a specific permission by name (string-based for convenience in handlers)
/// Uses cached roles data instead of individual DB queries
pub async fn has_permission_by_name(
    db: &sqlx::PgPool,
    profile_id: i32,
    is_super_admin: bool,
    permission_name: &str,
) -> Result<bool, sqlx::Error> {
    if is_super_admin {
        return Ok(true);
    }

    let roles = get_cached_roles(db, profile_id).await?;

    let check: fn(&UserRoleRow) -> bool = match permission_name {
        "can_edit_rota" => can_edit_rota,
        "can_access_diary" => can_access_diary,
        "can_work_shifts" => can_work_shifts,
        "can_edit_templates" => can_edit_templates,
        "can_edit_staff" => can_edit_staff,
        "can_view_staff_details" => can_view_staff_details,
        _ => return Err(sqlx::Error::RowNotFound),
    };

    Ok(roles.iter().any(check))
}
