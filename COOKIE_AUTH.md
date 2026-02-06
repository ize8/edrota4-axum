# Cookie-Based Authentication - Implementation Summary

## Overview

The Axum backend now seamlessly integrates with your TanStack Start frontend by supporting **HTTP-only cookie authentication**. No frontend changes required!

## What Changed

### Modified File: `src/extractors/auth.rs`

Added a new `extract_token_from_request()` function that:

1. **First priority:** Reads the `__session` cookie (set by Clerk on the frontend)
2. **Fallback:** Reads the `Authorization: Bearer <token>` header (for testing with curl)

### How It Works

```rust
fn extract_token_from_request(parts: &Parts) -> Option<String> {
    // Try __session cookie first (for TanStack frontend)
    if let Some(cookie_header) = parts.headers.get(header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                if let Some(value) = cookie.strip_prefix("__session=") {
                    return Some(value.to_string());
                }
            }
        }
    }

    // Fallback to Authorization header (for testing)
    if let Some(auth_header) = parts.headers.get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }

    None
}
```

## Frontend Integration

Your TanStack Start frontend can now make requests without manual token management:

```typescript
// Before (manual token handling) ❌
const { getToken } = useAuth();
const token = await getToken();
const response = await fetch('http://localhost:8080/api/users', {
  headers: {
    'Authorization': `Bearer ${token}`
  }
});

// After (automatic cookie handling) ✅
const response = await fetch('http://localhost:8080/api/users', {
  credentials: 'include', // This sends the __session cookie
});
```

## Key Benefits

1. **Seamless Integration:** Works with existing Clerk TanStack Start setup
2. **No Frontend Changes:** The `__session` cookie is automatically sent with `credentials: 'include'`
3. **Backwards Compatible:** Still supports Bearer tokens for testing with curl
4. **Secure:** HTTP-only cookies prevent XSS attacks (cookie not accessible from JavaScript)

## Testing

### Option 1: Frontend (Automatic)

Just run both servers and the authentication works automatically:

```bash
# Terminal 1 - Rust Backend
cd ~/Dev/Backend/edrota4-axum
cargo run

# Terminal 2 - TanStack Frontend
cd ~/Dev/WEB/edrota4
npm run dev
```

Open `http://localhost:3000` and login. All requests to the Rust backend will automatically include the `__session` cookie.

### Option 2: curl (Manual with Bearer Token)

For API testing without the frontend:

```bash
TOKEN="your_jwt_token_here"
curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:8080/api/auth/me
```

## CORS Configuration

The backend CORS is already configured correctly:

```rust
let cors = CorsLayer::new()
    .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT])
    .allow_credentials(true); // ✅ Required for cookies!
```

The `.allow_credentials(true)` is crucial - it tells the browser to include cookies in cross-origin requests.

## Troubleshooting

### "Missing authentication" Error

**Cause:** Frontend not sending cookie or cookie not being parsed

**Check:**
1. Browser DevTools → Network → Request Headers → Look for `Cookie: __session=...`
2. Verify `credentials: 'include'` is set in fetch options
3. Ensure both servers are on localhost (not mixing localhost/127.0.0.1)

### Cookie Not Being Sent

**Cause:** CORS issue or same-site cookie restrictions

**Fix:**
- Clerk's `__session` cookie is set with `SameSite=Lax` by default, which works for same-site requests (localhost:3000 → localhost:8080)
- Ensure CORS allows credentials: `.allow_credentials(true)` ✅

### Token Expired

**Cause:** Clerk session tokens expire after a period

**Fix:**
- Refresh the page to get a new session
- Clerk automatically refreshes tokens in the background

## Implementation Details

### Token Extraction Flow

```
Request arrives at Axum
    ↓
AuthenticatedUser::from_request_parts() called
    ↓
extract_token_from_request()
    ↓
Try __session cookie → Found? Use it
    ↓ No
Try Authorization header → Found? Use it
    ↓ No
Return 401 Unauthorized
    ↓ Yes (token found)
Validate JWT with Clerk JWKS
    ↓
Resolve email from Clerk API
    ↓
Auto-link user in database
    ↓
Return AuthenticatedUser
```

### Security Considerations

1. **HTTP-Only Cookie:** The `__session` cookie is HTTP-only, preventing JavaScript access (XSS protection)
2. **HTTPS Only (Production):** In production, Clerk sets `Secure` flag, requiring HTTPS
3. **SameSite:** Clerk uses `SameSite=Lax` to prevent CSRF attacks
4. **Token Validation:** Every request validates the JWT signature with Clerk's public keys (JWKS)

## Next Steps

1. ✅ Cookie authentication implemented
2. ✅ Documentation updated
3. ⏭️ Test integration with frontend
4. ⏭️ Verify all endpoints work with cookie auth
5. ⏭️ Implement mutation endpoints (Phase 2)

---

**Status:** Ready for testing! 🚀

The backend is now fully compatible with your TanStack Start frontend's cookie-based authentication.
