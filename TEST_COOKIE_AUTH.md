# Testing Cookie-Based Authentication

## Current Status ✅

The Rust backend is now running on `http://localhost:8080` with **cookie-based authentication** enabled!

## Test 1: Health Check (No Auth)

```bash
curl http://localhost:8080/health
# Expected: {"status":"ok"}
```

✅ **Result:** Works!

## Test 2: Public Endpoints (No Auth)

```bash
# Get all roles with nested workplaces
curl -s http://localhost:8080/api/roles | python3 -m json.tool | head -20

# Get time-off categories
curl -s http://localhost:8080/api/references/time-off-categories | python3 -m json.tool
```

✅ **Result:** Both work!

## Test 3: Protected Endpoints (Requires Auth)

### Option A: With Bearer Token (for testing)

```bash
# Replace with a fresh token from your frontend
TOKEN="your_clerk_jwt_token_here"

curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:8080/api/auth/me | python3 -m json.tool
```

### Option B: With Cookie (automatic from frontend)

Your TanStack Start frontend will automatically send the `__session` cookie when you make requests with `credentials: 'include'`.

## Test 4: Frontend Integration

### Step 1: Start Both Servers

**Terminal 1 - Rust Backend (Already Running):**
```bash
cd ~/Dev/Backend/edrota4-axum
cargo run
# Running on http://localhost:8080 ✅
```

**Terminal 2 - TanStack Frontend:**
```bash
cd ~/Dev/WEB/edrota4
npm run dev
# Running on http://localhost:3000
```

### Step 2: Create Test API Client

Create a new file in your frontend: `src/lib/rust-api.ts`

```typescript
const RUST_API_URL = 'http://localhost:8080';

async function fetchWithAuth(path: string, options: RequestInit = {}) {
  const response = await fetch(`${RUST_API_URL}${path}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options.headers,
    },
    credentials: 'include', // This sends the __session cookie
  });

  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.error || 'API request failed');
  }

  return response.json();
}

export const rustApi = {
  // Users
  getUsers: () => fetchWithAuth('/api/users'),
  getUser: (id: number) => fetchWithAuth(`/api/users/${id}`),

  // Shifts
  getShifts: (params: { year: number; month: number; roleId?: number }) => {
    const query = new URLSearchParams(
      Object.entries(params)
        .filter(([_, v]) => v !== undefined)
        .map(([k, v]) => [k, String(v)])
    ).toString();
    return fetchWithAuth(`/api/shifts?${query}`);
  },

  // Roles
  getRoles: () => fetchWithAuth('/api/roles'),

  // Add more as needed...
};
```

### Step 3: Test in Browser Console

Open `http://localhost:3000`, login, then open browser console and test:

```javascript
// Test 1: Get all roles
fetch('http://localhost:8080/api/roles', {
  credentials: 'include'
}).then(r => r.json()).then(console.log)

// Test 2: Get authenticated user (requires auth)
fetch('http://localhost:8080/api/auth/me', {
  credentials: 'include'
}).then(r => r.json()).then(console.log)

// Test 3: Get users list
fetch('http://localhost:8080/api/users', {
  credentials: 'include'
}).then(r => r.json()).then(console.log)
```

### Step 4: Verify Cookie is Sent

1. Open **Browser DevTools** → **Network** tab
2. Make a request to `http://localhost:8080/api/auth/me`
3. Click on the request
4. Check **Headers** → **Request Headers**
5. Look for `Cookie: __session=...`

If the cookie is there, authentication is working! 🎉

## Test 5: Compare with TypeScript Backend

To verify data consistency, compare responses:

```javascript
// Fetch from both backends
const tsUrl = 'http://localhost:3000/api/users'; // Your TS backend
const rustUrl = 'http://localhost:8080/api/users';

Promise.all([
  fetch(tsUrl, { credentials: 'include' }).then(r => r.json()),
  fetch(rustUrl, { credentials: 'include' }).then(r => r.json())
]).then(([tsData, rustData]) => {
  console.log('TypeScript:', tsData.length, 'users');
  console.log('Rust:', rustData.length, 'users');
  console.log('Match:', tsData.length === rustData.length);
});
```

## Common Issues

### 1. "Missing authentication" Error

**Symptoms:** 401 error with message about missing `__session` cookie

**Fixes:**
- Ensure `credentials: 'include'` is set in fetch
- Verify you're logged in to the frontend
- Check that both servers are on localhost
- Refresh the page to get a fresh session

### 2. CORS Error

**Symptoms:** Browser blocks request with CORS error

**Fixes:**
- Backend CORS is already configured for localhost:3000
- Ensure frontend is on http://localhost:3000 (not 127.0.0.1 or different port)
- Check browser console for exact error

### 3. Cookie Not Being Sent

**Symptoms:** Request goes through but no Cookie header

**Check:**
1. DevTools → Application → Cookies → http://localhost:3000
2. Look for `__session` cookie
3. If missing, you're not logged in

**Fixes:**
- Log in to the frontend first
- Clerk should automatically set the cookie
- If still missing, check Clerk configuration

### 4. Token Expired

**Symptoms:** "JWT validation failed" error

**Fix:**
- Clerk session tokens have an expiration time
- Refresh the page to get a new session
- Clerk handles token refresh automatically

## Success Criteria ✅

When everything is working, you should be able to:

1. ✅ Log in to frontend at http://localhost:3000
2. ✅ Make requests to Rust backend at http://localhost:8080
3. ✅ See `__session` cookie in request headers
4. ✅ Get authenticated user data from `/api/auth/me`
5. ✅ Access protected endpoints without manual token handling

## What's Different from Before?

### Before (Bearer Token)
```typescript
const token = await getToken();
fetch(url, {
  headers: {
    'Authorization': `Bearer ${token}`
  }
});
```

### After (Cookie-Based) ✅
```typescript
fetch(url, {
  credentials: 'include'
});
```

**No manual token management needed!** The browser handles everything automatically.

## Next Steps

Once you've verified cookie authentication works:

1. ✅ Test all GET endpoints listed in README.md
2. ✅ Verify data consistency with TypeScript backend
3. ⏭️ Gradually migrate frontend to use Rust API
4. ⏭️ Implement mutation endpoints (Phase 2)

---

**Status:** Ready for frontend integration testing! 🚀

The backend now works seamlessly with your TanStack Start frontend's cookie-based authentication.
