# EDRota4 Axum Backend — Implementation Plan

> **For AI agents:** This document is a complete specification for building the Rust/Axum backend.
> Read this fully before writing any code. Cross-reference with the frontend docs for business logic.

## Project Overview

**What:** Rust/Axum REST API backend for an NHS hospital rota management system.
**Why:** Replace TanStack Start server functions with a dedicated backend. Runs as a sidecar (port `8080`) sharing the same Neon PostgreSQL database, allowing gradual frontend migration.
**Where:** `./Backend/edrota4-axum/`

### Related Projects

| Project | Path | Notes |
|---|---|---|
| Frontend (TanStack Start) | `./WEB/edrota4/` | Full working app with server functions |
| Frontend agent docs | `./WEB/edrota4/.agent/` | **Detailed business logic, workflow, auth architecture, schema docs** — read these for any domain logic questions |
| Domain types (API shapes) | `./WEB/edrota4/src/types/domain.ts` | Rust structs must serialize to identical JSON |
| DB schema | `./WEB/edrota4/src/db/schema.ts` | Drizzle schema — source of truth for table/column names |
| Server functions | `./WEB/edrota4/src/server/*.ts` | The exact logic being ported to Rust |
| Repositories | `./WEB/edrota4/src/repositories/*.ts` | SQL query patterns to replicate |
| Auth verification | `./WEB/edrota4/src/server/utils/verify-auth.ts` | Permission checking logic |
| Clerk auth | `./WEB/edrota4/src/server/utils/clerk-auth.ts` | Email resolution + caching |
| Mock fixtures | `./Backend/edrota4-axum/fixtures/*.json` | Test data for all entities |
| Environment | `./Backend/edrota4-axum/.env` | DATABASE_URL, CLERK_SECRET_KEY, VITE_CLERK_PUBLISHABLE_KEY |

---

## Prerequisites — Rust Toolchain

Rust is NOT installed. First step:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Select option 1 (default)
source "$HOME/.cargo/env"
rustc --version && cargo --version
```

Then initialize the project:

```bash
cd ./Backend/edrota4-axum
cargo init --name edrota4-axum
```

This creates `Cargo.toml` and `src/main.rs` alongside the existing `.env` and `fixtures/`.

---

## Cargo.toml

```toml
[package]
name = "edrota4-axum"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = { version = "0.8", features = ["macros"] }
axum-extra = { version = "0.10", features = ["typed-header"] }
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono", "json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
jsonwebtoken = "9"
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }
dotenvy = "0.15"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
uuid = { version = "1", features = ["serde", "v4"] }
chrono = { version = "0.4", features = ["serde"] }
moka = { version = "0.12", features = ["future"] }
thiserror = "2"
```

**Why SQLx:** DB is owned by Drizzle migrations on the TypeScript side. SQLx works with raw SQL against existing tables — no migration ownership, no code generation required.
**Why jsonwebtoken (not clerk-rs):** Clerk uses standard JWTs with JWKS. Direct verification is more transparent and reliable than community wrappers.
**Why moka:** Async-native cache with TTL, replacing the TypeScript LRU cache pattern.

---

## File Structure

```
src/
├── main.rs                  # Entry: load .env, build state, build router, serve on :8080
├── config.rs                # AppConfig from env (DATABASE_URL, CLERK_SECRET_KEY, CLERK_DOMAIN)
├── error.rs                 # AppError enum → IntoResponse
├── startup.rs               # build_router() — assembles routes + middleware layers
│
├── auth/
│   ├── mod.rs
│   ├── clerk_jwks.rs        # Fetch + cache Clerk JWKS public keys
│   ├── jwt.rs               # Validate Bearer token against JWKS
│   └── claims.rs            # ClerkClaims { sub, exp, iat, iss, azp }
│
├── extractors/
│   ├── mod.rs
│   ├── auth.rs              # AuthenticatedUser extractor (JWT → user_profile_id)
│   └── permissions.rs       # RequirePermission<"can_edit_rota"> guard
│
├── db/
│   ├── mod.rs
│   └── pool.rs              # PgPool from DATABASE_URL
│
├── models/                  # SQLx FromRow + Serde structs
│   ├── mod.rs
│   ├── user.rs              # User, UserRole, StaffFilterOption
│   ├── shift.rs             # Shift, ShiftTemplate
│   ├── role.rs              # Role, Workplace
│   ├── diary.rs             # DiaryEntry
│   ├── comment.rs           # COD
│   ├── audit.rs             # AuditEntry
│   ├── job_plan.rs          # JobPlan, JobPlanTemplate
│   ├── marketplace.rs       # ShiftRequest, ShiftRequestWithDetails
│   └── time_off.rs          # TimeOffCategory
│
└── handlers/                # Route handlers (one file per domain)
    ├── mod.rs
    ├── health.rs
    ├── auth_handler.rs
    ├── users_handler.rs
    ├── shifts_handler.rs
    ├── roles_handler.rs
    ├── workplaces_handler.rs
    ├── templates_handler.rs
    ├── diary_handler.rs
    ├── comments_handler.rs
    ├── audit_handler.rs
    ├── job_plans_handler.rs
    ├── marketplace_handler.rs
    └── references_handler.rs
```

---

## Application State

```rust
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwks_cache: Arc<JwksCache>,
    pub user_cache: moka::future::Cache<String, Option<i32>>,  // auth_id → profile_id
    pub config: AppConfig,
}
```

---

## Authentication Architecture

### Flow (mirrors `./WEB/edrota4/src/server/utils/clerk-auth.ts` + `verify-auth.ts`)

1. Frontend sends `Authorization: Bearer <clerk-session-jwt>`
2. Decode JWT header → extract `kid` (key ID)
3. Fetch matching RSA public key from Clerk JWKS cache
   - JWKS endpoint: `https://{clerk-domain}/.well-known/jwks.json`
   - Clerk domain: decode base64 portion of `VITE_CLERK_PUBLISHABLE_KEY` after `pk_test_` or `pk_live_`
4. Validate JWT (signature, expiration, issuer)
5. Extract `sub` claim → Clerk user ID (`"user_xxx"`)
6. Resolve email: `GET https://api.clerk.com/v1/users/{sub}` with `Authorization: Bearer {CLERK_SECRET_KEY}` — cache result in moka (60s TTL)
7. Resolve `user_profile_id`: query `"Users"` WHERE `auth_id = sub`, fallback to `primary_email` match (auto-linking for first login)

### AuthenticatedUser Extractor

```rust
pub struct AuthenticatedUser {
    pub clerk_user_id: String,
    pub email: String,
    pub profile_id: i32,
    pub is_super_admin: bool,
}
```

Implement `FromRequestParts` for Axum extraction. Returns 401 on failure.

### Permission Guard

6 permissions stored in `"UserRoles"` table: `can_work_shifts`, `can_access_diary`, `can_edit_rota`, `can_edit_templates`, `can_edit_staff`, `can_view_staff_details`.

`is_super_admin` is on the `"Users"` table and **bypasses all permission checks**.

For each protected endpoint, check that the authenticated user has at least one `UserRole` with the required permission set to `true`, OR is a super admin.

See `./WEB/edrota4/src/server/utils/verify-auth.ts` for exact logic including `verifyAnyPermission()` (OR check across multiple permissions).

---

## Database — Critical Details

### Table names are PascalCase and MUST be quoted

```sql
SELECT * FROM "Shifts" WHERE role_id = $1
SELECT * FROM "Users" WHERE auth_id = $1
```

### Column → API field aliases (where they differ)

The TypeScript repositories use `mapToDomain()` to rename some columns. The Rust SQL queries must use aliases to match.

| Table | DB Column | API Field | SQL Pattern |
|---|---|---|---|
| `"Shifts"` | `role_id` | `role` | `SELECT role_id AS role` |
| `"Shifts"` | `time_off_category_id` | `time_off` | `SELECT time_off_category_id AS time_off` |
| `"ShiftTemplates"` | `role_id` | `role` | `SELECT role_id AS role` |
| `"Roles"` | `workplace_id` | `workplace` | `SELECT workplace_id AS workplace` |
| `"JobPlans"` | `role_id` | `user_role` | `SELECT role_id AS user_role` |
| `"JobPlanTemplates"` | `workplace_id` | `workplace` | `SELECT workplace_id AS workplace` |
| `"TimeOffCategories"` | `name` | `label` | `SELECT name AS label` |
| `"ShiftAudit"` | `role_id` | `role` | `SELECT role_id AS role` |

### The `"UserRoles"` table with nested `Roles` + `Workplaces`

The API returns `UserRole` objects with nested `Roles` (which itself contains nested `Workplaces`). This requires a JOIN query with manual struct assembly:

```typescript
// domain.ts shape to match:
interface UserRole {
    id: number;
    role_id: number;
    user_profile_id: number;
    can_edit_rota: boolean;
    // ... 5 more permission booleans
    created_at: string;
    Roles: {
        id: number;
        workplace: number;
        role_name: string;
        Workplaces: { id, hospital, ward, address, code } | null;
    } | null;
}
```

Use a flat JOIN query, then assemble the nested structs in Rust before serializing.

### AuditEntry enrichment

`"ShiftAudit"` stores raw JSON in `old`/`new` columns. The repository enriches these with `created_by_name`, `old_staff_name`, `new_staff_name`, `old_time_off_category`, `new_time_off_category` by joining against `"Users"` and `"TimeOffCategories"`. See `./WEB/edrota4/src/repositories/auditRepository.ts`.

---

## API Routes — Complete Map (~55 endpoints)

### Auth `/api/auth`
| Method | Route | Source fn | Permission |
|---|---|---|---|
| GET | `/me` | `getUserServer` | Authenticated |
| POST | `/verify-pin` | `verifyPin` | Authenticated |

### Users `/api/users`
| Method | Route | Source fn | Permission |
|---|---|---|---|
| GET | `/` | `getUsers` | — |
| GET | `/:id` | `getUser` | — |
| GET | `/substantive` | `getSubstantiveUsersServer` | — |
| GET | `/staff-list` | `getStaffList` | — |
| POST | `/search` | `searchUsers` | `can_edit_staff` |
| POST | `/profiles` | `createUserProfile` | `can_edit_staff` |
| PUT | `/profiles/:id` | `updateUserProfile` | `can_edit_staff` |
| PUT | `/me` | `updateOwnProfile` | Self |
| POST | `/me/pin` | `changeOwnPin` | Self |
| POST | `/check-email` | `checkEmailUsage` | `can_edit_staff` |
| POST | `/:id/reset-pin` | `resetUserPin` | `can_edit_staff` |
| POST | `/verify-identity` | `verifyProfileIdentity` | Authenticated |
| POST | `/change-profile-pin` | `changeProfilePin` | Token-verified |

### Shifts `/api/shifts`
| Method | Route | Source fn | Permission |
|---|---|---|---|
| GET | `/` | `getShiftsForMonth` | — |
| GET | `/by-date` | `getShiftsForDate` | — |
| GET | `/range` | `getShiftsForRange` | — |
| POST | `/` | `createShiftServer` | `can_edit_rota` |
| PUT | `/:uuid` | `updateShiftServer` | `can_edit_rota` |
| DELETE | `/:uuid` | `deleteShiftServer` | `can_edit_rota` |

### Roles `/api/roles`
| Method | Route | Permission |
|---|---|---|
| GET | `/` | — |
| POST | `/` | Super Admin |
| PUT | `/:id` | Super Admin |
| DELETE | `/:id` | Super Admin |
| GET | `/:id/dependencies` | Super Admin |
| POST | `/:id/nuke` | Super Admin |

### User Roles `/api/user-roles`
| Method | Route | Permission |
|---|---|---|
| GET | `/?user_profile_id=` | `can_edit_staff` |
| POST | `/` | `can_edit_staff` |
| PUT | `/:id/permissions` | `can_edit_staff` |
| DELETE | `/:id` | `can_edit_staff` |

### Workplaces `/api/workplaces`
| Method | Route | Permission |
|---|---|---|
| GET | `/` | — |
| POST | `/` | Super Admin |
| PUT | `/:id` | Super Admin |
| DELETE | `/:id` | Super Admin |
| GET | `/:id/dependencies` | Super Admin |
| POST | `/:id/nuke` | Super Admin |

### Templates `/api/templates`
| Method | Route | Permission |
|---|---|---|
| GET | `/?roleId=` | — |
| POST | `/` | `can_edit_templates` |
| PUT | `/:id` | `can_edit_templates` |
| DELETE | `/:id` | `can_edit_templates` |

### Diary `/api/diary`
| Method | Route | Permission |
|---|---|---|
| GET | `/?roleId=&start=&end=` | — |
| POST | `/` | `can_access_diary` |
| DELETE | `/:id` | `can_access_diary` |

### Comments `/api/comments`
| Method | Route | Permission |
|---|---|---|
| GET | `/?year=&month=&roleId=` | — |

### Audit `/api/audit`
| Method | Route | Permission |
|---|---|---|
| GET | `/?roleId=&year=&month=` | `can_edit_staff` OR `can_edit_templates` OR `can_edit_rota` |

### Job Plans `/api/job-plans`
| Method | Route | Permission |
|---|---|---|
| GET | `/?user_profile_id=&role_id=` | `can_edit_staff` |
| POST | `/` | `can_edit_staff` |
| PUT | `/:id` | `can_edit_staff` |
| DELETE | `/:id` | `can_edit_staff` |
| POST | `/:id/terminate` | `can_edit_staff` |

### Marketplace `/api/marketplace`
| Method | Route | Permission |
|---|---|---|
| GET | `/open?roleId=` | — |
| GET | `/my?userId=` | — |
| GET | `/incoming?userId=` | — |
| GET | `/approvals?roleId=` | `can_edit_rota` |
| GET | `/dashboard?userId=` | — |
| GET | `/swappable?roleId=&month=&year=` | — |
| POST | `/giveaway` | `can_work_shifts` |
| POST | `/pickup` | `can_edit_rota` |
| POST | `/swap` | `can_work_shifts` |
| POST | `/:id/claim` | `can_work_shifts` |
| POST | `/:id/respond` | `can_work_shifts` |
| POST | `/:id/resolve` | `can_edit_rota` |
| POST | `/:id/cancel` | `can_work_shifts` |

### References `/api/references`
| Method | Route | Permission |
|---|---|---|
| GET | `/time-off-categories` | — |

---

## CORS

```rust
CorsLayer::new()
    .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT])
    .allow_credentials(true)
```

---

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),          // 401
    #[error("Forbidden: {0}")]
    Forbidden(String),             // 403
    #[error("Not Found: {0}")]
    NotFound(String),              // 404
    #[error("Bad Request: {0}")]
    BadRequest(String),            // 400
    #[error("Conflict: {0}")]
    Conflict(String),              // 409
    #[error("{0}")]
    Internal(String),              // 500
    #[error("{0}")]
    Database(#[from] sqlx::Error), // 500
    #[error("{0}")]
    Validation(String),            // 422
}
```

Implement `IntoResponse` → JSON `{ "error": "<message>" }` with appropriate status code.

---

## Marketplace Business Logic

The marketplace is the most complex module. Read `./WEB/edrota4/src/server/marketplace.ts` (large file) thoroughly.

Key patterns:
- **Transactions:** Claim, approve, swap all use `db.transaction()`. In SQLx: `pool.begin()` → pass `&mut tx` → `tx.commit()`.
- **Auto-approve:** Some roles have `marketplace_auto_approve = true`. When a request is claimed/accepted for such a role, it resolves immediately without admin approval.
- **Swap two-phase:** PROPOSED → peer accepts/rejects → if accepted, goes to PENDING_APPROVAL → admin resolves.
- **Shift reassignment on approval:** When a giveaway/pickup/swap is APPROVED, the actual `"Shifts"` rows must be updated (reassign `user_profile_id`).
- **Generic account handling:** Marketplace mutations accept a `confirmedRequesterId` for when a generic-login user acts on behalf of a specific staff member (shadow identity). See the frontend `.agent` docs on shadow identity.

---

## Implementation Order

### Step 1 — Foundation
Create: `main.rs`, `config.rs`, `error.rs`, `startup.rs`, `db/pool.rs`
Endpoints: `GET /health`, `GET /api/references/time-off-categories`
**Verify:** `cargo run` + `curl localhost:8080/health`

### Step 2 — Auth
Create: `auth/*`, `extractors/*`, `models/user.rs`, `models/role.rs`
Endpoints: `GET /api/auth/me`, `POST /api/auth/verify-pin`
**Verify:** Use Clerk JWT from browser DevTools → curl with Bearer token

### Step 3 — All GET endpoints
Create: remaining `models/*`, all handler GET methods
**Verify:** Compare JSON responses with TanStack output — field names and types must match exactly

### Step 4 — All mutation endpoints
Implement POST/PUT/DELETE handlers with full business logic
Include: shift audit trail writes, marketplace transactions, PIN management
**Verify:** Mutate via Axum → confirm in TanStack app (same DB), and vice versa

---

## Verification Checklist

- [ ] `cargo build` compiles clean
- [ ] `cargo run` → server on `:8080`
- [ ] `GET /health` → `{"status":"ok"}`
- [ ] `GET /api/references/time-off-categories` → matches TanStack output
- [ ] Auth: Clerk JWT → `GET /api/auth/me` → correct user profile
- [ ] Read parity: all GET endpoint JSON shapes match `domain.ts` types
- [ ] Write parity: mutations via Axum reflect in TanStack app
- [ ] Marketplace: full giveaway/pickup/swap lifecycle works with transactions
- [ ] Permissions: unauthenticated → 401, wrong permission → 403, super admin → bypasses all
