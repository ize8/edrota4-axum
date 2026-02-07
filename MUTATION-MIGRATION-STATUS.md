# Mutation Migration Status

**Last Updated:** 2026-02-07
**Current Phase:** Phase 1 (Backend Fixes) - PARTIALLY COMPLETE

---

## Phase 1: Fix Axum Backend Issues

### ✅ COMPLETED (Commit: 6454004)

#### 1.1 Permission Fixes
- ✅ Changed `create_workplace` to require `is_super_admin` (not `can_edit_staff`)
- ✅ Changed `update_workplace` to require `is_super_admin` (not `can_edit_staff`)
- ✅ Changed `delete_workplace` to require `is_super_admin` (not `can_edit_staff`)
- ✅ Changed `create_role` to require `is_super_admin` (not `can_edit_staff`)
- ✅ Changed `update_role` to require `is_super_admin` (not `can_edit_staff`)
- ✅ Changed `delete_role` to require `is_super_admin` (not `can_edit_staff`)

#### 1.2 Missing Input Fields
- ✅ Added `marketplace_auto_approve: Option<bool>` to `CreateRoleInput`
- ✅ Added `marketplace_auto_approve: Option<bool>` to `UpdateRoleInput`
- ✅ Updated `create_role` SQL to INSERT marketplace_auto_approve
- ✅ Updated `update_role` to conditionally UPDATE marketplace_auto_approve
- ✅ Changed `hospital` from `Option<String>` to `String` in `CreateWorkplaceInput`

#### 1.3 Missing Validation in User Roles
- ✅ Added duplicate assignment check in `create_user_role`:
  - Queries existing UserRoles for user_profile_id + role_id combination
  - Returns 400 Bad Request if already assigned
- ✅ Added generic account block in `create_user_role`:
  - Checks if user is_generic_login
  - Blocks can_work_shifts=true for generic accounts
- ✅ Added generic account block in `update_user_role`:
  - Checks if user is_generic_login when enabling can_work_shifts
  - Prevents generic accounts from being granted shift work permission

**Files Modified:**
- `/Users/ize8/Dev/Backend/edrota4-axum/src/handlers/workplaces_handler.rs`
- `/Users/ize8/Dev/Backend/edrota4-axum/src/handlers/roles_handler.rs`
- `/Users/ize8/Dev/Backend/edrota4-axum/src/handlers/user_roles_handler.rs`
- `/Users/ize8/Dev/Backend/edrota4-axum/src/models/role_input.rs`

**Compilation Status:** ✅ All changes compile successfully

---

### ⏳ REMAINING (Phase 1.4 - Missing Endpoints)

These endpoints need to be implemented before frontend migration can proceed:

#### HIGH PRIORITY
1. **getWorkplaceDependencies** - `GET /api/workplaces/{id}/dependencies`
   - Count dependent records (roles, user_roles, shifts, etc.) before delete
   - Returns `DependencyCount` object
   - Required for UI confirmation dialogs

2. **getRoleDependencies** - `GET /api/roles/{id}/dependencies`
   - Count dependent records (user_roles, shifts, shift_requests, etc.)
   - Returns `DependencyCount` object
   - Required for UI confirmation dialogs

3. **nukeWorkplace** - `DELETE /api/workplaces/{id}/nuke`
   - Hard cascade delete (removes all dependent data)
   - Super admin only
   - Dangerous operation - requires careful implementation

4. **nukeRole** - `DELETE /api/roles/{id}/nuke`
   - Hard cascade delete (removes all dependent data)
   - Super admin only
   - Dangerous operation - requires careful implementation

#### MEDIUM PRIORITY
5. **createLogin** - `POST /api/users/create-login`
   - Create Clerk account for existing user profile
   - Links user profile to Clerk auth_id
   - Requires Clerk API integration

6. **changeOwnPassword** - `POST /api/users/me/password`
   - Change password via Clerk API
   - Verifies current password before changing
   - Blocks generic accounts

#### LOW PRIORITY (Rarely Used)
7. **createUser** - `POST /api/users` (super admin only)
   - Full user creation (rarely used, usually use createUserProfile instead)

8. **updateUser** - `PUT /api/users/{id}` (super admin only)
   - Full user update (rarely used)

---

## Phase 2: Frontend API Client Wrappers

**Status:** NOT STARTED

This phase requires creating API client wrappers for 43+ mutation endpoints across 8 modules:
1. Templates (3 endpoints)
2. Workplaces & Roles (10 endpoints - includes new nuke/dependencies)
3. User Roles (3 endpoints)
4. Job Plans (4 endpoints)
5. Diary (2 endpoints)
6. Users & PIN Management (11 endpoints - includes new createLogin/changePassword)
7. Shifts (3 endpoints)
8. Marketplace (7 endpoints)

**Estimated Effort:** 2-3 days (systematic implementation following established pattern)

---

## Phase 3: Testing Each Endpoint

**Status:** NOT STARTED

Per-endpoint verification protocol:
1. Test via curl/Postman
2. Verify response format matches TanStack
3. Check database state after mutation
4. Test error cases (403, 400, 404)
5. Test in UI with feature flag

**Estimated Effort:** 3-5 days (manual testing of 43+ endpoints)

---

## Phase 4: Switch Frontend to Axum

**Status:** NOT STARTED

Once all endpoints verified:
- Update `.env`: `VITE_USE_AXUM_BACKEND=true`
- Monitor for issues
- TanStack server functions remain as fallback

**Estimated Effort:** 1 day (monitoring + fixes)

---

## Generic Account Flow - DEFERRED

Per the migration plan, generic account flow support for mutations is deferred to Phase C:
- Marketplace mutations (confirmedRequesterId flow)
- Diary mutations (confirmedUserId flow)

**Rationale:** Get individual account mutations working first, then add generic account support as a separate phase.

---

## Next Steps

**Immediate (to unblock frontend migration):**
1. Implement `getWorkplaceDependencies` endpoint
2. Implement `getRoleDependencies` endpoint
3. Implement `nukeWorkplace` endpoint
4. Implement `nukeRole` endpoint
5. Implement `createLogin` endpoint
6. Implement `changeOwnPassword` endpoint

**After endpoints complete:**
7. Begin Phase 2: Create frontend API wrappers (systematic, batch-by-batch)
8. Begin Phase 3: Test each endpoint (manual verification)
9. Phase 4: Switch frontend to Axum mutations

---

## Reference Documents

- **Migration Plan:** `/Users/ize8/Dev/WEB/edrota4/.agent/migration/MUTATION-MIGRATION-PLAN.md`
- **Learned Patterns:** `/Users/ize8/Dev/WEB/edrota4/.agent/AXUM-BACKEND-GAPS.md`
- **Read-Only Progress:** `/Users/ize8/Dev/WEB/edrota4/.agent/migration/PHASE2-PROGRESS.md`
- **This Status Doc:** `/Users/ize8/Dev/Backend/edrota4-axum/MUTATION-MIGRATION-STATUS.md`

---

## Timeline Estimate

- **Phase 1 Remaining:** 2-3 days (6 endpoints)
- **Phase 2:** 2-3 days (frontend wrappers)
- **Phase 3:** 3-5 days (testing)
- **Phase 4:** 1 day (switch + monitor)

**Total:** ~8-12 days to complete full mutation migration

---

**Last Commit:** `6454004` - Phase 1 critical fixes implemented
