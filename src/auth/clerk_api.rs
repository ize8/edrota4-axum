use serde_json::Value;

use crate::AppError;

/// Check if an email exists in Clerk's user directory
/// Returns true if the email is registered with Clerk, false otherwise
pub async fn check_email_in_clerk(email: &str, clerk_secret_key: &str) -> Result<bool, AppError> {
    let client = reqwest::Client::new();

    // Clerk API endpoint to search users by email
    let url = "https://api.clerk.com/v1/users";

    tracing::debug!(email, "Checking email existence in Clerk");

    let response = client
        .get(url)
        .query(&[("email_address", email)])
        .header("Authorization", format!("Bearer {}", clerk_secret_key))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, email, "Failed to call Clerk API");
            AppError::Internal(format!("Failed to check email with Clerk: {}", e))
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        tracing::error!(status = %status, body, email, "Clerk API returned error");
        return Err(AppError::Internal(format!(
            "Clerk API error: {} - {}",
            status, body
        )));
    }

    let users: Vec<Value> = response.json().await.map_err(|e| {
        tracing::error!(error = %e, email, "Failed to parse Clerk API response");
        AppError::Internal(format!("Failed to parse Clerk response: {}", e))
    })?;

    let exists = !users.is_empty();
    tracing::debug!(email, exists, "Clerk email check result");

    Ok(exists)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a valid Clerk API key and will make real API calls
    // In production, consider mocking the HTTP client

    #[tokio::test]
    #[ignore] // Ignore by default to avoid requiring Clerk API key in CI
    async fn test_check_nonexistent_email() {
        let clerk_key = std::env::var("CLERK_SECRET_KEY").unwrap();
        let result = check_email_in_clerk("nonexistent@example.com", &clerk_key).await;

        // This test assumes the email doesn't exist
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }
}
