# Testing Authentication

## Step 1: Start the Server

```bash
cargo run
```

Server should start on `http://localhost:8080`

## Step 2: Get a Clerk JWT Token

### Option A: From Your Frontend App
1. Open your TanStack Start app (localhost:3000)
2. Log in with `zavori.laszlo@gmail.com` / `password`
3. Open Browser DevTools → Application/Storage → Cookies
4. Copy the `__session` cookie value (this is your JWT)

### Option B: Using Browser Console
1. Log into your frontend app
2. Open DevTools Console
3. Run: `document.cookie.split(';').find(c => c.includes('__session'))`
4. Copy the JWT token value

### Option C: From Network Tab
1. Log into frontend
2. Open DevTools → Network tab
3. Look for any API request
4. Check Request Headers for `Authorization: Bearer <token>`
5. Copy the token part

## Step 3: Test the Auth Endpoint

Replace `YOUR_JWT_TOKEN` with the actual token:

```bash
# Test health (no auth required)
curl http://localhost:8080/health

# Test auth/me (requires valid JWT)
curl -H "Authorization: Bearer YOUR_JWT_TOKEN" \
     http://localhost:8080/api/auth/me

# Expected response:
# {
#   "user_profile_id": 1,
#   "auth_id": "user_xxx",
#   "full_name": "Zavori Laszlo",
#   "short_name": "ZL",
#   "primary_email": "zavori.laszlo@gmail.com",
#   "is_super_admin": true,
#   ...
# }
```

## Step 4: Test Protected Endpoints

```bash
# Test time-off categories (no auth required)
curl http://localhost:8080/api/references/time-off-categories

# Test user roles (requires can_edit_staff permission)
# Should work for zavori.laszlo@gmail.com (super admin)
curl -H "Authorization: Bearer YOUR_JWT_TOKEN" \
     http://localhost:8080/api/user-roles

# Test without auth (should get 401)
curl http://localhost:8080/api/user-roles
```

## Step 5: Verify PIN Check

```bash
# Test PIN verification
curl -X POST http://localhost:8080/api/auth/verify-pin \
     -H "Authorization: Bearer YOUR_JWT_TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"user_profile_id": 1, "pin": "12345"}'

# Expected: {"valid": true} or {"valid": false}
```

## Troubleshooting

### Error: "Missing authorization header"
- Make sure you're passing the `Authorization: Bearer <token>` header
- Check that the token doesn't have extra quotes or spaces

### Error: "JWT validation failed"
- Token might be expired (Clerk tokens typically expire after 1 hour)
- Get a fresh token by logging in again
- Check that CLERK_SECRET_KEY and VITE_CLERK_PUBLISHABLE_KEY are correct in .env

### Error: "User profile not found"
- The user exists in Clerk but not in your database
- Check the database has a user with matching email
- The auto-linking should create the link on first auth

### Error: "Failed to resolve email"
- Check CLERK_SECRET_KEY is correct
- Check network connectivity to api.clerk.com
- Look at server logs for detailed error

## Quick Test Script

Save this as `test.sh`:

```bash
#!/bin/bash
TOKEN="$1"

if [ -z "$TOKEN" ]; then
  echo "Usage: ./test.sh YOUR_JWT_TOKEN"
  exit 1
fi

echo "Testing health endpoint..."
curl -s http://localhost:8080/health
echo -e "\n"

echo "Testing auth/me..."
curl -s -H "Authorization: Bearer $TOKEN" \
     http://localhost:8080/api/auth/me | jq .
echo -e "\n"

echo "Testing user roles..."
curl -s -H "Authorization: Bearer $TOKEN" \
     http://localhost:8080/api/user-roles | jq .
```

Run with: `chmod +x test.sh && ./test.sh YOUR_JWT_TOKEN`
