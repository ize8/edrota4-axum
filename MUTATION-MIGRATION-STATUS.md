# Mutation Migration Status

**Last Updated:** 2026-02-07
**Current Phase:** Phase 1 (Backend Fixes) - ✅ COMPLETE

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

### ✅ COMPLETED (Phase 1.4 - Missing Endpoints - Commit: TBD)

All 6 missing endpoints have been implemented:

#### HIGH PRIORITY (All Complete)
1. **✅ getWorkplaceDependencies** - `GET /api/workplaces/{id}/dependencies`
   - Counts all dependent records (roles, user_roles, shifts, templates, diary, audit, COD, shift_requests)
   - Returns `DependencyCount` with unique staff count
   - Transaction-safe queries

2. **✅ getRoleDependencies** - `GET /api/roles/{id}/dependencies`
   - Counts all dependent records for single role
   - Returns `DependencyCount` with unique staff count
   - Matches workplace pattern

3. **✅ nukeWorkplace** - `DELETE /api/workplaces/{id}/nuke`
   - Transaction-based cascade delete
   - Deletes in correct dependency order (deepest children → parent)
   - Comprehensive logging with warnings
   - Returns deleted role count in message

4. **✅ nukeRole** - `DELETE /api/roles/{id}/nuke`
   - Transaction-based cascade delete
   - Same deletion order as workplace
   - Comprehensive logging with warnings
   - Returns success message

#### MEDIUM PRIORITY (All Complete)
5. **✅ createLogin** - `POST /api/users/create-login`
   - Creates Clerk user via API (POST /v1/users)
   - Updates Users table with real auth_id (replaces temp_*)
   - Sets PIN for generic accounts
   - Returns `CreateLoginResponse` with auth_id and user_id
   - Super admin permission required

6. **✅ changeOwnPassword** - `POST /api/users/me/password`
   - Verifies current password via Clerk (POST /v1/users/{id}/verify_password)
   - Updates password via Clerk (PATCH /v1/users/{id})
   - Blocks generic accounts
   - Returns success response

#### LOW PRIORITY (Deferred - Rarely Used)
7. **createUser** - `POST /api/users` (super admin only)
   - Full user creation (rarely used, usually use createUserProfile instead)
   - **Status:** Deferred (not needed for migration)

8. **updateUser** - `PUT /api/users/{id}` (super admin only)
   - Full user update (rarely used)
   - **Status:** Deferred (not needed for migration)

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

**Phase 1 Complete! ✅**

All backend fixes and missing endpoints are implemented and compilation verified.

**Next Phase (Phase 2 - Frontend API Wrappers):**
1. Create API client wrapper functions for all mutation endpoints (43+ endpoints across 8 modules)
   - Templates (3 endpoints)
   - Workplaces & Roles (10 endpoints)
   - User Roles (3 endpoints)
   - Job Plans (4 endpoints)
   - Diary (2 endpoints)
   - Users & PIN Management (11 endpoints)
   - Shifts (3 endpoints)
   - Marketplace (7 endpoints)

2. Add feature flag wrappers in server function files (same pattern as reads)

**Phase 3: Test Each Endpoint**
- Manual testing protocol per endpoint
- Verify response format matches TanStack
- Test error cases (403, 400, 404)
- Test in UI with feature flag

**Phase 4: Switch Frontend**
- Set `VITE_USE_AXUM_BACKEND=true` (already true for reads)
- Monitor for issues

---

## Reference Documents

- **Migration Plan:** `/Users/ize8/Dev/WEB/edrota4/.agent/migration/MUTATION-MIGRATION-PLAN.md`
- **Learned Patterns:** `/Users/ize8/Dev/WEB/edrota4/.agent/AXUM-BACKEND-GAPS.md`
- **Read-Only Progress:** `/Users/ize8/Dev/WEB/edrota4/.agent/migration/PHASE2-PROGRESS.md`
- **This Status Doc:** `/Users/ize8/Dev/Backend/edrota4-axum/MUTATION-MIGRATION-STATUS.md`

---

## Timeline Estimate

- **Phase 1:** ✅ COMPLETE (2 commits)
- **Phase 2:** 2-3 days (frontend wrappers)
- **Phase 3:** 3-5 days (testing)
- **Phase 4:** 1 day (switch + monitor)

**Total Remaining:** ~6-9 days to complete full mutation migration

---

## Commits

**Commit 1:** `6454004` - Phase 1.1-1.3 (permissions, fields, validation)
**Commit 2:** `26b5991` - Phase 1.4 (6 missing endpoints + routes)
