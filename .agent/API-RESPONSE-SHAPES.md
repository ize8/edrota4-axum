# API Response Shapes

> Source: `./WEB/edrota4/src/types/domain.ts`
> Every Rust struct must serialize to JSON matching these TypeScript interfaces exactly.
> All fields are `snake_case`. Nullable fields serialize as `null` (not omitted).

---

## User
```json
{
  "user_profile_id": 1,
  "auth_id": "user_xxx",
  "full_name": "John Smith",
  "short_name": "JS",
  "primary_email": "john@nhs.net",
  "secondary_emails": ["old@nhs.net"],
  "tel": ["+447000000000"],
  "gmc": 1234567,
  "auth_pin": "12345",
  "is_super_admin": false,
  "comment": null,
  "created_at": "2025-01-01T00:00:00.000Z",
  "color": "#FF0000",
  "is_generic_login": false
}
```

## Workplace
```json
{
  "id": 1,
  "hospital": "Salisbury District Hospital",
  "ward": "Emergency Department",
  "address": "Odstock Road",
  "code": "SDH-ED"
}
```

## Role
```json
{
  "id": 1,
  "workplace": 1,
  "role_name": "Consultant",
  "Workplaces": { "id": 1, "hospital": "...", "ward": "...", "address": "...", "code": "..." }
}
```

Note: `Workplaces` is PascalCase (legacy naming from Drizzle relation).

## UserRole
```json
{
  "id": 1,
  "role_id": 1,
  "user_profile_id": 1,
  "can_edit_rota": true,
  "can_access_diary": false,
  "can_work_shifts": true,
  "can_edit_templates": false,
  "can_edit_staff": false,
  "can_view_staff_details": false,
  "created_at": "2025-01-01T00:00:00.000Z",
  "Roles": {
    "id": 1,
    "workplace": 1,
    "role_name": "Consultant",
    "Workplaces": { "id": 1, "hospital": "...", "ward": "...", "address": "...", "code": "..." }
  }
}
```

Note: `Roles` is PascalCase (legacy naming).

## Shift
```json
{
  "uuid": "550e8400-e29b-41d4-a716-446655440000",
  "role": 1,
  "label": "ED1",
  "start": "08:00",
  "end": "16:30",
  "money_per_hour": null,
  "pa_value": 2.125,
  "font_color": "black",
  "bk_color": "#90EE90",
  "is_locum": false,
  "published": true,
  "date": "2026-02-01",
  "created_at": "2025-01-01T00:00:00.000Z",
  "is_dcc": true,
  "is_spa": false,
  "time_off": null,
  "user_profile_id": 1,
  "created_by": 1
}
```

Note: `start`/`end` are `HH:MM` format (DB stores `HH:MM:SS`, normalize on output).

## ShiftTemplate
```json
{
  "id": 1,
  "role": 1,
  "label": "ED1",
  "start": "08:00",
  "end": "16:30",
  "font_color": "black",
  "bk_color": "#90EE90",
  "pa_value": 2.125,
  "money_per_hour": null,
  "is_spa": false,
  "is_dcc": true
}
```

## TimeOffCategory
```json
{
  "id": 1,
  "label": "Annual Leave",
  "short_name": "AL",
  "font_color": "black",
  "bk_color": "salmon"
}
```

Note: DB column is `name`, API field is `label`.

## COD (Comment on Date)
```json
{
  "id": 1,
  "role_id": 1,
  "date": "2026-02-01",
  "created_by": 1,
  "comment": "Bank holiday staffing",
  "created_at": "2025-01-01T00:00:00.000Z"
}
```

## DiaryEntry
```json
{
  "id": 1,
  "role_id": 1,
  "date": "2026-02-01",
  "entry": "NC - Training day",
  "al": false,
  "sl": true,
  "pl": false,
  "created_at": "2025-01-01T00:00:00.000Z",
  "user_profile_id": 1,
  "created_by": 1,
  "deleted": false
}
```

## AuditEntry
```json
{
  "uuid": "...",
  "role_id": 1,
  "created_by": 1,
  "created_by_name": "JS",
  "old": { "uuid": "...", "label": "ED1", "..." : "..." },
  "new": { "uuid": "...", "label": "ED2", "..." : "..." },
  "old_staff_name": "John Smith",
  "new_staff_name": "Jane Doe",
  "old_time_off_category": null,
  "new_time_off_category": "AL",
  "date": "2026-02-01",
  "created_at": "2025-01-01T00:00:00.000Z"
}
```

`old` and `new` are nullable JSON objects (null = created/deleted).

## JobPlan
```json
{
  "id": 1,
  "user_role": 1,
  "user_profile_id": 1,
  "dcc_pa": 7.5,
  "dcc_hour": 30.0,
  "spa_pa": 2.5,
  "spa_hour": 10.0,
  "al_per_year": 30,
  "sl_per_year": 10,
  "pl_per_year": 0,
  "from": "2025-04-01",
  "until": "2026-03-31",
  "comment": null
}
```

Note: DB column `role_id` → API field `user_role`.

## ShiftRequest
```json
{
  "id": 1,
  "shift_id": "uuid...",
  "requester_id": 1,
  "type": "GIVEAWAY",
  "status": "OPEN",
  "target_user_id": null,
  "target_shift_id": null,
  "candidate_id": null,
  "resolved_by": null,
  "resolved_at": null,
  "notes": null,
  "created_at": "2025-01-01T00:00:00.000Z",
  "updated_at": "2025-01-01T00:00:00.000Z"
}
```

## ShiftRequestWithDetails (enriched marketplace view)
```json
{
  "...all ShiftRequest fields...": "...",
  "shift_date": "2026-02-01",
  "shift_label": "ED1",
  "shift_start": "08:00",
  "shift_end": "16:30",
  "shift_role_id": 1,
  "shift_role_name": "Consultant",
  "shift_user_id": 1,
  "requester_name": "John Smith",
  "requester_short_name": "JS",
  "target_user_name": null,
  "target_user_short_name": null,
  "target_shift_date": null,
  "target_shift_label": null,
  "target_shift_start": null,
  "target_shift_end": null,
  "candidate_name": null,
  "candidate_short_name": null,
  "role_auto_approve": false
}
```

## StaffFilterOption
```json
{
  "user_profile_id": 1,
  "short_name": "JS",
  "full_name": "John Smith",
  "color": "#FF0000"
}
```
