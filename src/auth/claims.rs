use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClerkClaims {
    pub sub: String,  // Clerk user ID (user_xxx)
    pub exp: i64,     // Expiration timestamp
    pub iat: i64,     // Issued at timestamp
    pub iss: String,  // Issuer
    pub azp: Option<String>, // Authorized party
}
