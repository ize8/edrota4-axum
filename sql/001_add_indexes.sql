-- Performance indexes for frequently queried columns
-- Run against the Neon database manually: psql $DATABASE_URL -f sql/001_add_indexes.sql

-- Users: auth_id lookup (every authenticated request)
CREATE INDEX IF NOT EXISTS idx_users_auth_id ON "Users" (auth_id);

-- Users: email lookup (auto-linking)
CREATE INDEX IF NOT EXISTS idx_users_primary_email_lower ON "Users" (LOWER(primary_email));

-- UserRoles: user_profile_id (permission checks, staff listings)
CREATE INDEX IF NOT EXISTS idx_user_roles_user_profile_id ON "UserRoles" (user_profile_id);

-- UserRoles: role_id (dependency counts, cascades)
CREATE INDEX IF NOT EXISTS idx_user_roles_role_id ON "UserRoles" (role_id);

-- Shifts: role_id (dependency counts, marketplace queries)
CREATE INDEX IF NOT EXISTS idx_shifts_role_id ON "Shifts" (role_id);

-- Shifts: date (date-range queries, marketplace swaps)
CREATE INDEX IF NOT EXISTS idx_shifts_date ON "Shifts" (date);

-- Shifts: user_profile_id (user shift lookups)
CREATE INDEX IF NOT EXISTS idx_shifts_user_profile_id ON "Shifts" (user_profile_id);

-- JobPlans: role_id (dependency counts)
CREATE INDEX IF NOT EXISTS idx_job_plans_role_id ON "JobPlans" (role_id);

-- ShiftTemplates: role_id (dependency counts)
CREATE INDEX IF NOT EXISTS idx_shift_templates_role_id ON "ShiftTemplates" (role_id);

-- Diary: role_id + date (diary queries are always filtered by both)
CREATE INDEX IF NOT EXISTS idx_diary_role_id_date ON "Diary" (role_id, date);

-- ShiftAudit: role_id (dependency counts)
CREATE INDEX IF NOT EXISTS idx_shift_audit_role_id ON "ShiftAudit" (role_id);

-- COD: role_id + date (comment-of-day lookups)
CREATE INDEX IF NOT EXISTS idx_cod_role_id_date ON "COD" (role_id, date);

-- ShiftRequests: shift_id (marketplace join queries)
CREATE INDEX IF NOT EXISTS idx_shift_requests_shift_id ON "ShiftRequests" (shift_id);

-- Roles: workplace_id (workplace dependency cascades)
CREATE INDEX IF NOT EXISTS idx_roles_workplace_id ON "Roles" (workplace_id);
