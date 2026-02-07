use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClerkClaims {
    pub sub: String,  // Clerk user ID (user_xxx)
    pub exp: i64,     // Expiration timestamp
    pub iat: i64,     // Issued at timestamp
    pub iss: String,  // Issuer
    pub azp: Option<String>, // Authorized party

    // Custom claims (set in Clerk Dashboard session token)
    #[serde(rename = "primaryEmail")]
    pub primary_email: Option<String>,      // Primary email (custom claim)

    // Standard user data fields (available in Clerk JWTs)
    pub email: Option<String>,              // Primary email address (fallback)
    pub email_verified: Option<bool>,       // Email verification status
    pub name: Option<String>,               // Full name
    pub given_name: Option<String>,         // First name
    pub family_name: Option<String>,        // Last name
}

impl ClerkClaims {
    /// Get the user's email, preferring the custom claim over the standard field
    pub fn get_email(&self) -> Option<&str> {
        self.primary_email.as_deref().or(self.email.as_deref())
    }
}
