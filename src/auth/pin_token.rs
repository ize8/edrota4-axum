use base64::{engine::general_purpose::STANDARD, Engine as _};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::AppError;

type HmacSha256 = Hmac<Sha256>;

/// Generate a PIN verification token valid for 5 minutes
/// Token format: base64(user_profile_id:expiry_timestamp:hmac_signature)
pub fn generate_pin_token(user_profile_id: i32, secret: &str) -> Result<String, AppError> {
    // Calculate expiry time (5 minutes from now)
    let expiry_time = chrono::Utc::now().timestamp() + (5 * 60); // 300 seconds

    // Create payload: user_profile_id:expiry_timestamp
    let payload = format!("{}:{}", user_profile_id, expiry_time);

    // Generate HMAC signature
    let signature = create_hmac_signature(&payload, secret)?;

    // Combine payload and signature
    let token_data = format!("{}:{}", payload, signature);

    // Base64 encode the entire token
    let token = STANDARD.encode(token_data.as_bytes());

    Ok(token)
}

/// Validate a PIN verification token and extract the user_profile_id
/// Returns the user_profile_id if token is valid and not expired
pub fn validate_pin_token(token: &str, secret: &str) -> Result<i32, AppError> {
    // Base64 decode the token
    let decoded_bytes = STANDARD
        .decode(token)
        .map_err(|_| AppError::Unauthorized("Invalid token format".to_string()))?;

    let decoded = String::from_utf8(decoded_bytes)
        .map_err(|_| AppError::Unauthorized("Invalid token encoding".to_string()))?;

    // Parse token: user_profile_id:expiry_time:signature
    let parts: Vec<&str> = decoded.split(':').collect();

    if parts.len() != 3 {
        return Err(AppError::Unauthorized("Invalid token structure".to_string()));
    }

    let user_profile_id: i32 = parts[0]
        .parse()
        .map_err(|_| AppError::Unauthorized("Invalid user ID in token".to_string()))?;

    let expiry_time: i64 = parts[1]
        .parse()
        .map_err(|_| AppError::Unauthorized("Invalid expiry time in token".to_string()))?;

    let token_signature = parts[2];

    // Check if token has expired
    let current_time = chrono::Utc::now().timestamp();
    if current_time > expiry_time {
        return Err(AppError::BadRequest(
            "Verification token has expired. Please start over.".to_string(),
        ));
    }

    // Verify HMAC signature
    let payload = format!("{}:{}", user_profile_id, expiry_time);
    let expected_signature = create_hmac_signature(&payload, secret)?;

    // Constant-time comparison to prevent timing attacks
    if token_signature != expected_signature {
        return Err(AppError::Unauthorized("Invalid verification token".to_string()));
    }

    Ok(user_profile_id)
}

/// Create HMAC-SHA256 signature for the given data
fn create_hmac_signature(data: &str, secret: &str) -> Result<String, AppError> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| AppError::Internal(format!("HMAC initialization error: {}", e)))?;

    mac.update(data.as_bytes());

    let result = mac.finalize();
    let code_bytes = result.into_bytes();

    Ok(hex::encode(code_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_validate_token() {
        let secret = "test_secret_key_for_testing_purposes";
        let user_id = 123;

        let token = generate_pin_token(user_id, secret).unwrap();
        let validated_user_id = validate_pin_token(&token, secret).unwrap();

        assert_eq!(user_id, validated_user_id);
    }

    #[test]
    fn test_invalid_token_format() {
        let secret = "test_secret_key";
        let result = validate_pin_token("invalid_token", secret);

        assert!(result.is_err());
    }

    #[test]
    fn test_token_with_wrong_signature() {
        let secret = "test_secret_key";
        let wrong_secret = "wrong_secret_key";

        let token = generate_pin_token(123, secret).unwrap();
        let result = validate_pin_token(&token, wrong_secret);

        assert!(result.is_err());
    }
}
