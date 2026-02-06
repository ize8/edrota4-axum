use jsonwebtoken::{decode, Algorithm, Header, Validation};

use super::{claims::ClerkClaims, clerk_jwks::JwksCache};

pub async fn validate_jwt(
    token: &str,
    jwks_cache: &JwksCache,
    expected_issuer: &str,
) -> Result<ClerkClaims, String> {
    // Decode header to get kid
    let header = decode_header(token)?;
    let kid = header.kid.ok_or("Missing kid in JWT header")?;

    // Get decoding key from JWKS cache
    let decoding_key = jwks_cache.get_decoding_key(&kid).await?;

    // Set up validation
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[expected_issuer]);
    validation.validate_exp = true;

    // Decode and validate token
    let token_data = decode::<ClerkClaims>(token, &decoding_key, &validation)
        .map_err(|e| format!("JWT validation failed: {}", e))?;

    Ok(token_data.claims)
}

fn decode_header(token: &str) -> Result<Header, String> {
    jsonwebtoken::decode_header(token).map_err(|e| format!("Failed to decode JWT header: {}", e))
}
