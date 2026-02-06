use jsonwebtoken::{jwk::JwkSet, DecodingKey};
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

pub struct JwksCache {
    cache: Cache<String, Arc<JwkSet>>,
    jwks_url: String,
}

impl JwksCache {
    pub fn new(clerk_domain: &str) -> Self {
        let jwks_url = format!("https://{}/.well-known/jwks.json", clerk_domain);

        let cache = Cache::builder()
            .time_to_live(Duration::from_secs(3600)) // 1 hour TTL
            .build();

        Self { cache, jwks_url }
    }

    pub async fn get_jwks(&self) -> Result<Arc<JwkSet>, String> {
        // Try to get from cache
        if let Some(jwks) = self.cache.get(&self.jwks_url).await {
            return Ok(jwks);
        }

        // Fetch from Clerk
        let response = reqwest::get(&self.jwks_url)
            .await
            .map_err(|e| format!("Failed to fetch JWKS: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("JWKS endpoint returned {}", response.status()));
        }

        let jwks: JwkSet = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse JWKS: {}", e))?;

        let jwks_arc = Arc::new(jwks);
        self.cache.insert(self.jwks_url.clone(), jwks_arc.clone()).await;

        Ok(jwks_arc)
    }

    pub async fn get_decoding_key(&self, kid: &str) -> Result<DecodingKey, String> {
        let jwks = self.get_jwks().await?;

        let jwk = jwks
            .keys
            .iter()
            .find(|k| k.common.key_id.as_deref() == Some(kid))
            .ok_or_else(|| format!("No key found with kid: {}", kid))?;

        DecodingKey::from_jwk(jwk).map_err(|e| format!("Failed to create decoding key: {}", e))
    }
}
