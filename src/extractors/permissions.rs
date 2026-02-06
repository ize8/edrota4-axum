use sqlx;

/// Check if user has the required permission
pub async fn has_permission(
    db: &sqlx::PgPool,
    profile_id: i32,
    is_super_admin: bool,
    permission_check: impl Fn(&UserRoleRow) -> bool,
) -> Result<bool, sqlx::Error> {
    // Super admins bypass all checks
    if is_super_admin {
        return Ok(true);
    }

    // Query user roles and check permission
    let roles = sqlx::query_as::<_, UserRoleRow>(
        r#"SELECT * FROM "UserRoles" WHERE user_profile_id = $1"#,
    )
    .bind(profile_id)
    .fetch_all(db)
    .await?;

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

    let roles = sqlx::query_as::<_, UserRoleRow>(
        r#"SELECT * FROM "UserRoles" WHERE user_profile_id = $1"#,
    )
    .bind(profile_id)
    .fetch_all(db)
    .await?;

    for check in checks {
        if roles.iter().any(check) {
            return Ok(true);
        }
    }

    Ok(false)
}

#[derive(sqlx::FromRow)]
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
/// This is a safe alternative to the SQL injection-prone pattern used in individual handlers
pub async fn has_permission_by_name(
    db: &sqlx::PgPool,
    profile_id: i32,
    is_super_admin: bool,
    permission_name: &str,
) -> Result<bool, sqlx::Error> {
    // Super admins bypass all checks
    if is_super_admin {
        return Ok(true);
    }

    // Use a safe approach with CASE statement instead of string interpolation
    let has_perm: bool = match permission_name {
        "can_edit_rota" => {
            sqlx::query_scalar(
                r#"SELECT EXISTS(SELECT 1 FROM "UserRoles" WHERE user_profile_id = $1 AND can_edit_rota = true)"#
            )
            .bind(profile_id)
            .fetch_one(db)
            .await?
        }
        "can_access_diary" => {
            sqlx::query_scalar(
                r#"SELECT EXISTS(SELECT 1 FROM "UserRoles" WHERE user_profile_id = $1 AND can_access_diary = true)"#
            )
            .bind(profile_id)
            .fetch_one(db)
            .await?
        }
        "can_work_shifts" => {
            sqlx::query_scalar(
                r#"SELECT EXISTS(SELECT 1 FROM "UserRoles" WHERE user_profile_id = $1 AND can_work_shifts = true)"#
            )
            .bind(profile_id)
            .fetch_one(db)
            .await?
        }
        "can_edit_templates" => {
            sqlx::query_scalar(
                r#"SELECT EXISTS(SELECT 1 FROM "UserRoles" WHERE user_profile_id = $1 AND can_edit_templates = true)"#
            )
            .bind(profile_id)
            .fetch_one(db)
            .await?
        }
        "can_edit_staff" => {
            sqlx::query_scalar(
                r#"SELECT EXISTS(SELECT 1 FROM "UserRoles" WHERE user_profile_id = $1 AND can_edit_staff = true)"#
            )
            .bind(profile_id)
            .fetch_one(db)
            .await?
        }
        "can_view_staff_details" => {
            sqlx::query_scalar(
                r#"SELECT EXISTS(SELECT 1 FROM "UserRoles" WHERE user_profile_id = $1 AND can_view_staff_details = true)"#
            )
            .bind(profile_id)
            .fetch_one(db)
            .await?
        }
        _ => return Err(sqlx::Error::RowNotFound), // Invalid permission name
    };

    Ok(has_perm)
}
