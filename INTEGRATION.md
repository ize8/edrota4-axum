# Frontend Integration Guide

## Quick Start

### 1. Run Both Servers

**Terminal 1 - Rust Backend:**
```bash
cd ~/Dev/Backend/edrota4-axum
cargo run
# Running on http://localhost:8080
```

**Terminal 2 - TanStack Start Frontend:**
```bash
cd ~/Dev/WEB/edrota4
npm run dev
# Running on http://localhost:3000
```

### 2. Test Integration

Open your browser to `http://localhost:3000` and login with:
- `zavori.laszlo@gmail.com` / `password` (super admin)

The frontend can now make requests to both backends:
- **Rust (read):** `http://localhost:8080/api/*`
- **TypeScript (write):** Existing server functions

---

## API Client Example

Create a utility to call the Rust backend:

```typescript
// src/lib/rust-api.ts

const RUST_API_URL = 'http://localhost:8080';

async function fetchWithAuth(path: string, options: RequestInit = {}) {
  // No need to manually get token - Clerk's __session cookie is automatically sent!
  const response = await fetch(`${RUST_API_URL}${path}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options.headers,
    },
    credentials: 'include', // IMPORTANT: This sends the __session cookie
  });

  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.error || 'API request failed');
  }

  return response.json();
}

// Usage examples
export const rustApi = {
  // Users
  getUsers: () => fetchWithAuth('/api/users'),
  getUser: (id: number) => fetchWithAuth(`/api/users/${id}`),
  getStaffList: () => fetchWithAuth('/api/users/staff-list'),

  // Shifts
  getShifts: (params: { year: number; month: number; roleId?: number }) => {
    const query = new URLSearchParams(params as any).toString();
    return fetchWithAuth(`/api/shifts?${query}`);
  },

  // Marketplace
  getOpenRequests: (roleId?: number) => {
    const query = roleId ? `?roleId=${roleId}` : '';
    return fetchWithAuth(`/api/marketplace/open${query}`);
  },

  // Add more as needed...
};
```

---

## Gradual Migration Strategy

### Phase 1: Test Read Operations (Current)

Replace TypeScript server functions with Rust API calls for read operations:

```typescript
// Before (TypeScript)
import { getShiftsForMonth } from '@/server/shifts';

// After (Rust)
import { rustApi } from '@/lib/rust-api';
const shifts = await rustApi.getShifts({ year: 2026, month: 2, roleId: 1 });
```

### Phase 2: Add Mutations (Future)

Once mutation endpoints are implemented in Rust:

```typescript
// Create shift
await rustApi.createShift({
  role: 1,
  label: 'ED1',
  date: '2026-02-15',
  start: '08:00',
  end: '16:00',
  // ... other fields
});
```

### Phase 3: Complete Migration

Eventually replace all TypeScript server functions with Rust API calls.

---

## Testing Checklist

Use this checklist to verify each endpoint works with your frontend:

### ✅ Reference Data
- [ ] Time-off categories load correctly
- [ ] Roles with workplaces display properly
- [ ] User roles show nested structure

### ✅ Users
- [ ] User list loads
- [ ] Staff filter dropdown populates
- [ ] User profile view works

### ✅ Shifts
- [ ] Monthly shift view loads
- [ ] Date picker shows correct shifts
- [ ] Role filtering works

### ✅ Diary & Comments
- [ ] Diary entries display
- [ ] Comments on dates show up

### ✅ Marketplace
- [ ] Open requests list loads
- [ ] Dashboard counters are correct
- [ ] Swappable shifts appear

### ✅ Authentication
- [ ] Login flow works
- [ ] JWT token is sent correctly
- [ ] Permission checks work (403 for unauthorized)
- [ ] Super admin bypasses permissions

---

## Common Issues & Solutions

### CORS Errors
**Problem:** Browser blocks requests from localhost:3000 to localhost:8080

**Solution:** Already configured! The Rust backend has CORS enabled for localhost:3000. If you see issues, check the browser console.

### 401 Unauthorized
**Problem:** "Missing authentication: no __session cookie or Authorization header" or "JWT validation failed"

**Solutions:**
1. Check that `credentials: 'include'` is set in fetch options
2. Verify the `__session` cookie is being sent: Look in Network tab → Cookies
3. Token might be expired - refresh the page to get a new session
4. For local testing, ensure both servers are on localhost (not mixing localhost and 127.0.0.1)

### 403 Forbidden
**Problem:** "Missing can_edit_staff permission"

**Solutions:**
1. Verify the user has the required permission in the database
2. Super admin users should bypass all checks
3. Check the endpoint's permission requirements in the implementation

### Type Mismatches
**Problem:** Date/timestamp fields have wrong format

**Solution:** Rust serializes dates as ISO 8601 strings (RFC3339), which TypeScript should handle automatically. Check your date parsing code.

### Empty Results
**Problem:** API returns empty array when data exists

**Solutions:**
1. Check query parameters are correct (roleId vs role_id, etc.)
2. Verify data exists in database with correct values
3. Check the SQL query in the handler

---

## Performance Notes

### Caching
- JWKS keys are cached for 1 hour
- User email resolution is cached for 60 seconds
- Consider adding frontend caching with React Query or SWR

### Query Optimization
- Use specific filters to reduce data transfer
- For large datasets, consider pagination (not yet implemented)
- Date range queries are more efficient than fetching entire months

### Database Connection Pool
- Current: 5 max connections
- Increase in production based on load
- Monitor connection usage

---

## Debugging Tips

### 1. Check Rust Server Logs
The server outputs detailed logs:
```bash
cargo run
# Look for:
# - "Database pool created successfully"
# - "Server listening on 0.0.0.0:8080"
# - Request logs (if tracing enabled)
```

### 2. Use curl for Direct Testing
```bash
# Test without frontend
curl http://localhost:8080/api/users

# Test with token
curl -H "Authorization: Bearer YOUR_TOKEN" \
     http://localhost:8080/api/auth/me
```

### 3. Compare with TypeScript Backend
Query both backends and compare JSON responses:
```typescript
const tsResult = await getShiftsForMonth(2026, 2, 1);
const rustResult = await rustApi.getShifts({ year: 2026, month: 2, roleId: 1 });
console.log('TypeScript:', tsResult);
console.log('Rust:', rustResult);
```

### 4. Check Database Directly
```sql
-- Verify data exists
SELECT * FROM "Shifts" WHERE role_id = 1 LIMIT 10;
SELECT * FROM "Users" WHERE user_profile_id = 2;
```

---

## Migration Roadmap

### Week 1: Read Operations
- [x] Set up Rust backend
- [x] Implement all GET endpoints
- [ ] Test each endpoint with frontend
- [ ] Verify data consistency

### Week 2: Critical Mutations
- [ ] Implement Shift CRUD
- [ ] Implement User profile updates
- [ ] Add audit trail writes
- [ ] Test write operations

### Week 3: Marketplace
- [ ] Implement marketplace mutations
- [ ] Add transaction support
- [ ] Test swap/giveaway workflows

### Week 4: Production Prep
- [ ] Add monitoring/logging
- [ ] Security hardening
- [ ] Performance optimization
- [ ] Deploy to staging

---

## Support

If you encounter issues:
1. Check server logs for errors
2. Verify database state
3. Test with curl to isolate frontend vs backend issues
4. Check the implementation files in `src/handlers/`

Reference the main documentation:
- `README.md` - Overview and API listing
- `.agent/IMPLEMENTATION-PLAN.md` - Original specification
- `TEST_AUTH.md` - Authentication testing guide

---

Happy integrating! 🚀
