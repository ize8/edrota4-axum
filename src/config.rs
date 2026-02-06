use std::env;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub database_url: String,
    pub clerk_secret_key: String,
    pub clerk_publishable_key: String,
    pub clerk_domain: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL must be set".to_string())?;

        let clerk_secret_key = env::var("CLERK_SECRET_KEY")
            .map_err(|_| "CLERK_SECRET_KEY must be set".to_string())?;

        let clerk_publishable_key = env::var("VITE_CLERK_PUBLISHABLE_KEY")
            .map_err(|_| "VITE_CLERK_PUBLISHABLE_KEY must be set".to_string())?;

        // Extract Clerk domain from publishable key
        // Format: pk_test_xxx or pk_live_xxx
        let clerk_domain = extract_clerk_domain(&clerk_publishable_key)?;

        Ok(Self {
            database_url,
            clerk_secret_key,
            clerk_publishable_key,
            clerk_domain,
        })
    }
}

fn extract_clerk_domain(publishable_key: &str) -> Result<String, String> {
    // Remove pk_test_ or pk_live_ prefix
    let encoded = publishable_key
        .strip_prefix("pk_test_")
        .or_else(|| publishable_key.strip_prefix("pk_live_"))
        .ok_or("Invalid Clerk publishable key format")?;

    // The domain is base64-encoded in the key
    // For simplicity, we'll decode it
    use std::str;
    let decoded = base64_decode(encoded)
        .map_err(|_| "Failed to decode Clerk domain")?;

    let domain = str::from_utf8(&decoded)
        .map_err(|_| "Invalid UTF-8 in Clerk domain")?
        .trim_end_matches('$')
        .to_string();

    Ok(domain)
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    STANDARD.decode(input).map_err(|e| format!("Base64 decode error: {}", e))
}
