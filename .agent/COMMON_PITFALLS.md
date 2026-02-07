# Common Pitfalls & Shadow Mode Learnings

## Purpose
This document catalogs common mismatches found during the TanStack → Axum migration using shadow mode testing. Use this checklist when implementing new Axum endpoints.

---

## 1. Database Column Mapping Mismatches

### Problem
Rust models use `#[sqlx(rename = "...")]` to map database columns to struct fields, but the mapping doesn't match the actual database schema.

### Example
```rust
// ❌ WRONG - Database column is "type" not "request_type"
#[sqlx(rename = "request_type")]
pub request_type: String,

// ✅ CORRECT
#[sqlx(rename = "type")]
pub request_type: String,
```

### How to Prevent
1. **Always check the Drizzle schema** (`src/db/schema.ts`) for the actual SQL column names
2. Look for the second parameter in Drizzle field definitions:
   ```typescript
   type: varchar('type', { length: 20 })  // SQL column is "type"
   requesterId: integer('requester_id')   // SQL column is "requester_id"
   ```
3. **Remember**: Drizzle uses camelCase in TypeScript but maps to snake_case in SQL
4. **Timestamp types**: Check `withTimezone` parameter:
   ```typescript
   // TIMESTAMP (no timezone) → use NaiveDateTime in Rust
   createdAt: timestamp('created_at', { precision: 6 })

   // TIMESTAMPTZ (with timezone) → use DateTime<Utc> in Rust
   createdAt: timestamp('created_at', { precision: 6, withTimezone: true })
   ```
5. **Verification query**: After creating a model, test with a simple `SELECT * FROM "TableName" LIMIT 1` to ensure all fields map correctly

---

## 2. Response Structure Mismatches

### Problem
Axum endpoint returns a different structure than TanStack (different field names or data types).

### Examples Encountered

#### Dashboard: Counts vs Full Data
```rust
// ❌ WRONG - Returns counts
Json(json!({ "my": 2, "incoming": 0 }))

// ✅ CORRECT - Returns full arrays like TanStack
Json(json!({
    "myRequests": my_requests,  // Array of ShiftRequest
    "incomingSwaps": incoming_swaps  // Array of ShiftRequest
}))
```

#### Staff List: Simplified vs Full Schema
```typescript
// ❌ WRONG - TanStack returned simplified
{ id: userProfileId, name: fullName }

// ✅ CORRECT - Match expected API contract
{ user_profile_id, short_name, full_name, color }
```

### How to Prevent
1. **Read the TanStack implementation** before writing Axum handler
2. **Check the TypeScript type** in `src/types/domain.ts` for the expected return type
3. **Check the API client** (`src/api/*.ts`) to see what the frontend expects
4. **Copy TanStack's response structure exactly** - don't "improve" it
5. **Rule**: If TanStack returns arrays, Axum must return arrays (not counts)

---

## 3. Business Logic Mismatches

### Problem
Axum endpoint missing filters or sorting that TanStack has.

### Example: Missing `can_work_shifts` Filter
```rust
// ❌ WRONG - Missing filter
WHERE u.is_generic_login = false AND ur.role_id = $1

// ✅ CORRECT - Includes all TanStack filters
WHERE u.is_generic_login = false
  AND ur.role_id = $1
  AND ur.can_work_shifts = true  // Don't forget this!
```

### How to Prevent
1. **Compare SQL queries** side-by-side with TanStack's Drizzle query
2. Look for ALL `.where()` conditions in TanStack
3. Look for `.orderBy()` - sorting must match
4. **Rule**: When in doubt, TanStack's logic is correct (it's battle-tested)

---

## 4. Type Serialization Mismatches

### Problem A: String vs Number for IDs
TanStack (Drizzle) sometimes serializes IDs as strings, Axum returns numbers.

**Solution**: Already handled by `normalizeIds()` in `src/api/shadow.ts` - no action needed.

### Problem B: Request Deserialization
Frontend sends `role_id: "1"` (string) but Rust expects `i32`.

**Solution**: Use custom deserializer for flexibility:
```rust
fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<i32, D::Error>
where D: Deserializer<'de> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNumber {
        String(String),
        Number(i32),
    }
    match StringOrNumber::deserialize(deserializer)? {
        StringOrNumber::String(s) => s.parse::<i32>().map_err(D::Error::custom),
        StringOrNumber::Number(n) => Ok(n),
    }
}

// Usage
#[derive(Deserialize)]
struct MyRequest {
    #[serde(deserialize_with = "deserialize_string_or_number")]
    role_id: i32,
}
```

### How to Prevent
1. **Check frontend API client** to see what format is being sent
2. If you see `roleId: params.roleId` (camelCase param), frontend likely sends snake_case in body
3. Add custom deserializer for any ID fields that might come as strings

---

## 5. URL Path Mismatches

### Problem
Frontend API client calls different URL than what Axum routes define.

### Example
```typescript
// ❌ Frontend calling wrong path
apiClient.get('/api/marketplace/my-requests')

// ✅ Should match Axum route
apiClient.get('/api/marketplace/my')
```

### How to Prevent
1. **Check `startup.rs`** first to see what routes are registered
2. **Always verify URL paths** in 3 places:
   - Axum: `src/startup.rs` `.route("/path", get(handler))`
   - Frontend API: `src/api/*.ts` `apiClient.get('/api/path')`
   - TanStack (if comparing): `src/server/*.ts`
3. Keep a mapping table if needed:
   ```
   /api/marketplace/my        → get_my_requests
   /api/marketplace/incoming  → get_incoming_requests
   /api/marketplace/dashboard → get_dashboard
   ```

---

## 6. Permission/Auth Mismatches

### Problem
Axum endpoint has stricter or different permission requirements than TanStack.

### Example: getUserRoles Permission Too Strict
```rust
// ❌ WRONG - Requires can_edit_staff even for viewing own roles
let has_perm = permissions::has_permission(&state.db, auth.profile_id, auth.is_super_admin, permissions::can_edit_staff).await?;
if !has_perm { return Err(AppError::Forbidden(...)); }

// ✅ CORRECT - Allow viewing own roles without special permission
if target_user_id != auth.profile_id {
    // Only require permission when viewing OTHER users' roles
    let has_perm = permissions::has_permission(...).await?;
    if !has_perm { return Err(AppError::Forbidden(...)); }
}
```

### How to Prevent
1. **Check TanStack's auth check** - what permission does it require?
2. **Look for `verifyServerAuth('permission', request)`** in TanStack handler
3. Match the permission key exactly: `can_work_shifts`, `can_edit_staff`, etc.
4. Consider edge cases: self vs others, admin vs regular user

---

## Pre-Implementation Checklist

Before implementing any new Axum endpoint, check:

- [ ] **Schema**: Read Drizzle schema to confirm SQL column names
- [ ] **Column Types**: Check for TIMESTAMP (use `NaiveDateTime`) vs TIMESTAMPTZ (use `DateTime<Utc>`)
- [ ] **TanStack Handler**: Read the equivalent TanStack server function
- [ ] **Response Type**: Check TypeScript type in `src/types/domain.ts`
- [ ] **API Client**: Check what the frontend expects in `src/api/*.ts`
- [ ] **Business Logic**: Copy ALL filters, sorting, and joins from TanStack
- [ ] **Permissions**: Match TanStack's auth requirements exactly
- [ ] **URL Path**: Verify path matches in `startup.rs` and frontend API
- [ ] **Time/Timestamp Fields**: Don't worry about format differences (shadow mode normalizes)
- [ ] **Test Data**: Have sample data ready for shadow mode testing

---

## Shadow Mode Testing Best Practices

1. **Navigate to the UI** - Don't just curl endpoints; use the actual app
2. **Check console logs** - Shadow mode logs all mismatches
3. **Fix Axum to match TanStack** - Don't change TanStack unless there's a bug
4. **One endpoint at a time** - Don't enable shadow mode for everything at once
5. **Document learnings** - Add new pitfalls to this document

---

## 7. Time & Timestamp Serialization Differences

### Problem
Time and timestamp fields serialize differently between Drizzle (TanStack) and sqlx (Axum), causing shadow mode mismatches even though the data is semantically identical.

### Examples

#### TIME fields (shift_start, shift_end)
```typescript
// TanStack (Drizzle)
shift_start: "08:30:00"  // Always includes :00 seconds
shift_end: "18:30:00"

// Axum (sqlx)
shift_start: "08:30"     // Omits :00 seconds
shift_end: "18:30"
```

#### TIMESTAMP fields (created_at, updated_at)
```typescript
// TanStack (Drizzle with Date.toISOString())
created_at: "2026-02-02T15:41:50.217Z"      // Adds Z suffix (treats as UTC)
resolved_at: "2026-02-02T18:07:29.573Z"

// Axum (sqlx with NaiveDateTime)
created_at: "2026-02-02T15:41:50.217104"    // No Z, more precision (microseconds)
resolved_at: "2026-02-02T18:07:29.573"
```

### Root Cause
1. **TIME type**: PostgreSQL stores TIME, but different libraries format it differently:
   - Drizzle always includes seconds (even `:00`)
   - sqlx may omit `:00` seconds
2. **TIMESTAMP type**:
   - Drizzle's `.toISOString()` adds `Z` suffix (UTC indicator)
   - sqlx's `NaiveDateTime` has no timezone, so no `Z`
   - Microsecond precision differs (3 vs 6 decimal places)

### How to Prevent
**This is now handled automatically** by the shadow mode normalizer in `src/api/shadow.ts`:
- ✅ Strips `:00` from time fields
- ✅ Removes `Z` suffix from timestamps
- ✅ Normalizes microsecond precision to milliseconds

### Manual Fix (if needed)
If you need to match the formats exactly in the API response (not just for comparison):

**For TIME fields in Axum:**
```rust
// Option 1: Format as HH:MM:SS in the query
SELECT to_char(start, 'HH24:MI:SS') as shift_start

// Option 2: Post-process in the mapper
shift_start: row.shift_start.map(|t| format!("{}:00", t))
```

**For TIMESTAMP fields in Axum:**
```rust
// Use DateTime<Utc> instead of NaiveDateTime if you want the Z suffix
// But check schema first - most tables use TIMESTAMP not TIMESTAMPTZ!
```

**Best Practice**: Don't change Axum to match TanStack's quirks. Let shadow mode normalize the comparison.

---

## 8. Array Ordering Mismatches

### Problem
Both endpoints return the same data but in different order, causing shadow mode to report a mismatch.

### Example
```
Axum:     [14, 13, 12, 11, 10, 9, 7, 6, 5, 4]  (ORDER BY created_at DESC)
TanStack: [6, 5, 13, 12, 11, 10, 9, 4, 14, 7]  (no ORDER BY - random)
```

### Root Cause
TanStack query has no `ORDER BY` clause, so results come in database default order (non-deterministic). Axum adds explicit `ORDER BY` for consistency.

### How to Prevent
1. **Check TanStack's query** - Does it have an ORDER BY?
   - If YES: Match the exact same ordering in Axum
   - If NO: Either add ORDER BY to both, or accept that ordering doesn't matter
2. **Shadow mode handles this automatically** - Sorts arrays by `id` before comparing (as of this fix)
3. **Best practice**: Always add `ORDER BY` for consistent results, even if TanStack doesn't have it

### Solution Applied
Updated `normalizeIds()` in `src/api/shadow.ts` to automatically sort arrays by `id` before comparison. This way, ordering differences don't cause false positives in shadow mode.

---

## Common Error Messages & Solutions

| Error | Root Cause | Solution |
|-------|-----------|----------|
| `no column found for name: X` | sqlx rename doesn't match DB | Check Drizzle schema for actual column name |
| `mismatched types; Rust type DateTime<Utc> is not compatible with SQL type TIMESTAMP` | Using DateTime<Utc> for TIMESTAMP column | Use `NaiveDateTime` instead (no timezone) |
| `Response mismatch` (shadow mode) | Different response structure | Match TanStack's response exactly |
| `Response mismatch` (same items, different order) | Missing or different ORDER BY | Shadow mode auto-sorts by ID; optionally add ORDER BY to both |
| `Response mismatch` (time format differences) | TIME/TIMESTAMP serialization | Shadow mode auto-normalizes; no action needed |
| `422 Unprocessable Entity` | Request deserialization failed | Check frontend sends snake_case, add deserializer if needed |
| `403 Forbidden` | Permission check too strict | Match TanStack's permission requirements |
| Array length mismatch | Missing filter in query | Compare WHERE clauses with TanStack |

---

## When Shadow Mode Finds a Mismatch

1. **Don't panic** - This is exactly what shadow mode is for!
2. **Expand the logged objects** in console to see the full diff
3. **Identify the category** - Schema? Logic? Permissions? Response structure?
4. **Fix Axum** - TanStack is the source of truth
5. **Test again** - Refresh and verify the mismatch is gone
6. **Document** - If it's a new pattern, add it to this file

---

## 9. Grouped vs Flat Response Structures

### Problem
Axum returns a flat array of items when TanStack groups them by a related entity (e.g., shifts grouped by user).

### Example: getSwappableShifts
```typescript
// ❌ WRONG - Axum returned flat array of 300 shifts
[
  { uuid: "...", role: 1, label: "WE Early", user_profile_id: 29, ... },
  { uuid: "...", role: 1, label: "Late", user_profile_id: 29, ... },
  ...
]

// ✅ CORRECT - TanStack returns 51 users with nested shifts
[
  {
    userId: 18,
    userName: "Adam Hughes",
    shifts: [
      { uuid: "...", date: "2026-02-01", startTime: "08:00", endTime: "18:00", label: "WE Early", isTimeOff: false },
      ...
    ]
  },
  ...
]
```

### Root Cause
1. **Different business logic**: TanStack groups data on the backend for UI consumption
2. **Response type mismatch**: Axum returned `Vec<Shift>` instead of `Vec<UserWithSwappableShifts>`
3. **Missing grouping logic**: Axum didn't have code to group shifts by user

### How to Prevent
1. **Check the return type** in TypeScript domain types (`src/types/domain.ts`) and API client (`src/api/*.ts`)
2. **Look for nested structures** in TanStack response - arrays of objects with nested arrays indicate grouping
3. **Check how the frontend uses the data** - if it accesses `item.userName` or `item.shifts`, it expects grouping
4. **Read TanStack repository implementation** - grouping logic like `.reduce()` or `Map.set()` indicates grouped response
5. **Create matching Rust types** for grouped structures with proper serde rename:
   ```rust
   #[derive(Serialize, Deserialize, ToSchema)]
   pub struct UserWithSwappableShifts {
       #[serde(rename = "userId")]
       pub user_id: i32,
       #[serde(rename = "userName")]
       pub user_name: String,
       pub shifts: Vec<SwappableShift>,
   }
   ```
6. **Use HashMap or similar** in Rust to group results before returning

### Solution Applied
- Created `SwappableShift` and `UserWithSwappableShifts` types in `src/models/marketplace.rs`
- Updated Axum handler to use LEFT JOIN and HashMap to group shifts by user
- Added missing `exclude_user_id` parameter
- Added future date filter (only shifts >= today)
- Result now matches TanStack's grouped structure

---

## 10. Conditional Filters (Special Values)

### Problem
TanStack applies filters conditionally based on special values (e.g., `roleId > 0`), but Axum applies them whenever the parameter is provided, regardless of value.

### Example: getPendingApprovals with roleId=0
```typescript
// ❌ WRONG - Axum filters even when roleId=0
if let Some(role_id) = query.role_id {
    sql.push_str(" AND s.role_id = $1");  // Filters for role_id = 0 (likely no results!)
}

// ✅ CORRECT - TanStack only filters when roleId > 0
if (roleId > 0) {
    conditions.push(eq(shifts.role, roleId));  // Skip filter when roleId = 0
}
```

### Root Cause
1. **Special value semantics**: `roleId = 0` often means "all roles" in the app (no filter)
2. **Option vs value check**: Rust's `Option<i32>` checks if value exists, not what it is
3. **Different business logic**: TanStack has conditional logic, Axum applied filter unconditionally

### Impact
- TanStack with `roleId=0`: Returns all pending approvals (no role filter)
- Axum with `roleId=0`: Filters for `role_id = 0` (likely returns nothing)
- Result: Shadow mode mismatch, empty array from Axum vs populated array from TanStack

### How to Prevent
1. **Check for conditional filters** in TanStack - look for `if (value > 0)`, `if (value !== null)`, etc.
2. **Understand special values** - 0, -1, null, empty string may have special meaning (all, none, default)
3. **Match the condition** in Rust:
   ```rust
   // Instead of just checking Some()
   if let Some(role_id) = query.role_id {
       // Check the actual value too
       if role_id > 0 {
           sql.push_str(" AND s.role_id = $1");
       }
   }
   ```
4. **Document special values** in comments or API docs

### Solution Applied
Added nested `if role_id > 0` check in `get_approval_requests` to match TanStack's conditional filter logic.

---

## 11. Cascading Side Effects (Competing Requests)

### Problem
When implementing marketplace mutations (claim, approve, swap), forgetting to cancel competing requests after shift ownership changes.

### Background
Users can create multiple requests for the same shift:
- User has Shift A
- Creates SWAP offering Shift A → User B
- Also creates GIVEAWAY offering Shift A to anyone
- User C claims GIVEAWAY (approved, Shift A now belongs to User C)
- **Bug**: SWAP is still PENDING_APPROVAL. If admin approves it later, Shift A moves again → conflict!

### Example
```rust
// ❌ WRONG - Missing competing requests cancellation
pub async fn approve_request(state: State<Arc<AppState>>, ...) -> AppResult<Json<...>> {
    // Update request status
    sqlx::query("UPDATE \"ShiftRequests\" SET status = 'APPROVED' WHERE id = $1")
        .bind(request_id)
        .execute(&state.db)
        .await?;

    // Update shift ownership
    sqlx::query("UPDATE \"Shifts\" SET user_profile_id = $1 WHERE uuid = $2")
        .bind(candidate_id)
        .bind(shift_id)
        .execute(&state.db)
        .await?;

    // Missing: Cancel all other requests involving this shift!
    Ok(Json(updated_request))
}

// ✅ CORRECT - Cancel competing requests in same transaction
pub async fn approve_request(state: State<Arc<AppState>>, ...) -> AppResult<Json<...>> {
    let mut tx = state.db.begin().await?;

    // 1. Update request status
    sqlx::query("UPDATE \"ShiftRequests\" SET status = 'APPROVED', resolved_at = NOW() WHERE id = $1")
        .bind(request_id)
        .execute(&mut *tx)
        .await?;

    // 2. Update shift ownership
    sqlx::query("UPDATE \"Shifts\" SET user_profile_id = $1 WHERE uuid = $2")
        .bind(candidate_id)
        .bind(shift_id)
        .execute(&mut *tx)
        .await?;

    // 3. CRITICAL: Cancel all competing requests
    sqlx::query(
        r#"
        UPDATE "ShiftRequests"
        SET status = 'CANCELLED'
        WHERE id != $1
          AND (shift_id = $2 OR target_shift_id = $2)
          AND status IN ('OPEN', 'PROPOSED', 'PENDING_APPROVAL')
        "#
    )
    .bind(request_id)       // Exclude current request
    .bind(shift_id)         // Cancel any request involving this shift
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Json(updated_request))
}
```

### Root Cause
1. **Atomic operation missing**: Shift ownership change without side effects cleanup
2. **Business logic**: Multiple requests can target the same shift (first-come-first-served)
3. **Easy to forget**: Side effect not immediately obvious from API signature

### Impact
- Orphaned requests remain active after shift is reassigned
- Admin can approve a request for a shift that's no longer available
- Double-booking: same shift assigned to multiple users
- Data inconsistency: request shows APPROVED but shift owner doesn't match

### How to Prevent
1. **Always use transactions** for mutations that change shift ownership
2. **Check TanStack for `cancelCompetingRequests` calls** - if TanStack calls it, Axum must too
3. **Look for both sides**: Cancel requests where shift is EITHER `shift_id` OR `target_shift_id`
4. **Required in these handlers**:
   - `claimRequest` / `claim_shift_request` (when claiming GIVEAWAY/PICKUP)
   - `respondToSwap` / `respond_to_swap_proposal` (when peer accepts swap)
   - `resolveRequest` / `approve_shift_request` (when admin approves any request)

### Checklist for Marketplace Mutations

When implementing any mutation that changes shift ownership:

- [ ] Uses transaction (`BEGIN` ... `COMMIT`)
- [ ] Updates request status to final state (APPROVED/REJECTED)
- [ ] Updates shift `user_profile_id` if approved
- [ ] **Calls competing requests cancellation** (UPDATE ShiftRequests SET status = 'CANCELLED' WHERE...)
- [ ] Excludes current request from cancellation (`id != $1`)
- [ ] Checks both shift directions (`shift_id = $2 OR target_shift_id = $2`)
- [ ] Only cancels active statuses (`status IN ('OPEN', 'PROPOSED', 'PENDING_APPROVAL')`)

### TanStack Reference
See `src/server/marketplace.ts`:
- `claimRequest` lines 618-631
- `respondToSwap` lines 752-762
- `resolveRequest` lines 823-858

All three call `cancelCompetingRequestsTx()` after shift updates.

---

## Future Improvements

### Possible Solutions to Reduce Mismatches

1. **OpenAPI Schema Generation**
   - Generate TypeScript types from Axum's OpenAPI spec
   - Would catch response structure mismatches at compile time

2. **Database Schema Codegen**
   - Generate Rust models directly from PostgreSQL schema
   - Would eliminate column name mismatches

3. **Integration Test Suite**
   - Automated tests that compare TanStack vs Axum responses
   - Run before deployment

4. **Shared Type Definitions**
   - Define types once, generate for both Rust and TypeScript
   - Tools: `typeshare`, `ts-rs`

For now, **manual review + shadow mode** is our safety net.
