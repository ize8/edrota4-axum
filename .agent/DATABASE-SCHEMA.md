# Database Schema Reference

> Source of truth: `./WEB/edrota4/src/db/schema.ts` (Drizzle ORM)
> Migrations: `./WEB/edrota4/drizzle/` (0000–0011)
> **The Rust side does NOT own migrations.** Read-only schema knowledge.

## Important: All table names are PascalCase and must be quoted in SQL

```sql
-- CORRECT:
SELECT * FROM "Shifts" WHERE role_id = $1
-- WRONG (will fail):
SELECT * FROM Shifts WHERE role_id = $1
```

---

## Tables

### "Workplaces"
| Column | Type | Nullable | Notes |
|---|---|---|---|
| `id` | serial PK | no | |
| `hospital` | varchar(255) | yes | |
| `ward` | varchar(255) | yes | |
| `address` | varchar(255) | yes | |
| `code` | varchar(50) | yes | e.g. 'SDH-ED' |

### "Roles"
| Column | Type | Nullable | Notes |
|---|---|---|---|
| `id` | serial PK | no | |
| `workplace_id` | int FK→Workplaces | no | API field: `workplace` |
| `role_name` | varchar | no | |
| `marketplace_auto_approve` | boolean | no | default false |

### "Users"
| Column | Type | Nullable | Notes |
|---|---|---|---|
| `user_profile_id` | serial PK | no | |
| `auth_id` | varchar(255) UNIQUE | no | Clerk user ID |
| `full_name` | varchar(255) | no | |
| `short_name` | varchar(255) | no | |
| `primary_email` | text | yes | Unique case-insensitive |
| `secondary_emails` | text[] | yes | default '{}' |
| `tel` | varchar(255)[] | yes | |
| `gmc` | int | yes | GMC registration number |
| `auth_pin` | varchar(5) | yes | 5-digit PIN, NULL for generic accounts |
| `is_super_admin` | boolean | no | default false |
| `comment` | varchar | yes | |
| `created_at` | timestamp(6) | no | |
| `color` | varchar(7) | yes | Hex color |
| `is_generic_login` | boolean | no | default false |

**Constraints:**
- `generic_accounts_no_pin`: generic accounts must have NULL PIN
- `primary_not_in_secondary`: primary email not in secondary array
- Case-insensitive unique index on `primary_email`

### "Shifts"
| Column | Type | Nullable | Notes |
|---|---|---|---|
| `uuid` | uuid PK | no | default random |
| `role_id` | int FK→Roles | no | API field: `role` |
| `label` | varchar(255) | no | default '--' |
| `start` | time | yes | |
| `end` | time | yes | |
| `money_per_hour` | real | yes | |
| `pa_value` | real | no | default 0 |
| `font_color` | varchar | no | default 'black' |
| `bk_color` | varchar | no | default 'white' |
| `is_locum` | boolean | no | default false |
| `published` | boolean | no | default false |
| `date` | date | no | |
| `created_at` | timestamp(6) | no | |
| `is_spa` | boolean | no | default false |
| `is_dcc` | boolean | no | default true |
| `time_off_category_id` | int FK→TimeOffCategories | yes | API field: `time_off` |
| `user_profile_id` | int FK→Users | yes | Assigned staff |
| `created_by` | int FK→Users | no | |

**Indexes:** `(role_id, date)`, `(user_profile_id)`

### "ShiftRequests"
| Column | Type | Nullable | Notes |
|---|---|---|---|
| `id` | serial PK | no | |
| `shift_id` | uuid FK→Shifts | no | |
| `requester_id` | int FK→Users | no | |
| `type` | varchar(20) | no | GIVEAWAY, PICKUP, SWAP |
| `status` | varchar(20) | no | OPEN, PENDING_APPROVAL, APPROVED, REJECTED, CANCELLED, PROPOSED, PEER_ACCEPTED, PEER_REJECTED |
| `target_user_id` | int FK→Users | yes | Swap target |
| `target_shift_id` | uuid FK→Shifts | yes | Swap target shift |
| `candidate_id` | int FK→Users | yes | Claimer/acceptor |
| `resolved_by` | int FK→Users | yes | |
| `resolved_at` | timestamp(6) | yes | |
| `notes` | varchar | yes | |
| `created_at` | timestamp(6) | no | |
| `updated_at` | timestamp(6) | no | |

### "COD" (Comments on Date)
| Column | Type | Nullable | Notes |
|---|---|---|---|
| `id` | serial PK | no | |
| `role_id` | int FK→Roles | no | |
| `date` | date | no | |
| `created_by` | int FK→Users | no | |
| `comment` | varchar | yes | |
| `created_at` | timestamp(6, tz) | yes | |

### "ShiftTemplates"
| Column | Type | Nullable | Notes |
|---|---|---|---|
| `id` | serial PK | no | |
| `role_id` | int FK→Roles | no | API field: `role` |
| `label` | varchar | no | |
| `start` | time | yes | |
| `end` | time | yes | |
| `font_color` | varchar | yes | |
| `bk_color` | varchar | yes | |
| `pa_value` | real | yes | |
| `money_per_hour` | real | yes | |
| `is_spa` | boolean | no | default false |
| `is_dcc` | boolean | no | default false |

### "UserRoles"
| Column | Type | Nullable | Notes |
|---|---|---|---|
| `id` | serial PK | no | |
| `role_id` | int FK→Roles | no | |
| `user_profile_id` | int FK→Users | no | |
| `can_edit_rota` | boolean | no | default false |
| `can_access_diary` | boolean | no | default false |
| `can_work_shifts` | boolean | no | default true |
| `can_edit_templates` | boolean | no | default false |
| `can_edit_staff` | boolean | no | default false |
| `can_view_staff_details` | boolean | no | default false |
| `created_at` | timestamp(6) | no | |

### "JobPlans"
| Column | Type | Nullable | Notes |
|---|---|---|---|
| `id` | serial PK | no | |
| `role_id` | int FK→Roles | no | API field: `user_role` |
| `user_profile_id` | int FK→Users | no | |
| `dcc_pa` | real | yes | |
| `dcc_hour` | real | yes | |
| `spa_pa` | real | yes | |
| `spa_hour` | real | yes | |
| `al_per_year` | real | no | default 0 |
| `sl_per_year` | real | no | default 0 |
| `pl_per_year` | real | no | default 0 |
| `from` | date | no | |
| `until` | date | yes | |
| `comment` | varchar | yes | |

### "Diary"
| Column | Type | Nullable | Notes |
|---|---|---|---|
| `id` | serial PK | no | |
| `role_id` | int FK→Roles | no | |
| `date` | date | no | |
| `entry` | varchar | yes | |
| `al` | boolean | no | default false — Annual Leave |
| `sl` | boolean | no | default false — Study Leave |
| `pl` | boolean | no | default false — Professional Leave |
| `created_at` | timestamp(6) | no | |
| `user_profile_id` | int FK→Users | yes | |
| `created_by` | int FK→Users | no | |
| `deleted` | boolean | no | default false — soft delete |

### "ShiftAudit"
| Column | Type | Nullable | Notes |
|---|---|---|---|
| `uuid` | uuid PK | no | |
| `role_id` | int FK→Roles | no | |
| `created_at` | timestamp(6) | no | |
| `created_by` | int FK→Users | no | |
| `old` | json | yes | Previous shift state |
| `new` | json | yes | New shift state |
| `date` | date | yes | |

### "TimeOffCategories"
| Column | Type | Nullable | Notes |
|---|---|---|---|
| `id` | serial PK | no | |
| `name` | varchar | no | API field: `label` |
| `short_name` | varchar | no | |
| `comment` | varchar | yes | |
| `font_color` | varchar | no | default 'black' |
| `bk_color` | varchar | no | default 'salmon' |

### "JobPlanTemplates"
| Column | Type | Nullable | Notes |
|---|---|---|---|
| `id` | serial PK | no | |
| `workplace_id` | int FK→Workplaces | no | API field: `workplace` |
| `label` | varchar(255) | no | |
| `dcc_pa` | real | yes | |
| `dcc_hour` | real | yes | |
| `spa_pa` | real | yes | |
| `spa_hour` | real | yes | |
| `al_per_year` | real | yes | |
