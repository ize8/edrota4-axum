use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClerkClaims {
    pub sub: String,  // Clerk user ID (user_xxx)
    pub exp: i64,     // Expiration timestamp
    pub iat: i64,     // Issued at timestamp
    pub iss: String,  // Issuer
    pub azp: Option<String>, // Authorized party

    // User data fields (available in Clerk JWTs)
    pub email: Option<String>,              // Primary email address
    pub email_verified: Option<bool>,       // Email verification status
    pub name: Option<String>,               // Full name
    pub given_name: Option<String>,         // First name
    pub family_name: Option<String>,        // Last name
}
