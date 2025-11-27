use std::env;
use anyhow::{Context, Result};
use bytes::Bytes;
use http_body_util::Full;
use serde::Deserialize;
use wasi_identity::credentials::get_identity;
use wit_bindgen::block_on;

use crate::ENV;
use crate::provider::Provider;

#[derive(Debug, serde::Deserialize)]
struct CachedValue {
    value: Vec<u8>,
    #[allow(dead_code)]
    expires_at: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
struct AzureTokenResponse {
    access_token: String,
    #[allow(dead_code)]
    token_type: String,
    #[allow(dead_code)]
    expires_in: u64,
}

const TOKEN_CACHE_KEY: &str = "azure_ad_access_token";

pub async fn access_token(provider: &Provider) -> Result<String> {
    // Priority 1: OAuth credentials (production and local dev with real credentials)
    if let (Ok(client_id), Ok(client_secret), Ok(tenant)) = (
        env::var("AZURE_CLIENT_ID"),
        env::var("AZURE_CLIENT_SECRET"),
        env::var("AZURE_TENANT_ID"),
    ) && !client_secret.is_empty() {
        return get_cached_azure_token(&client_id, &client_secret, &tenant, provider).await;
    }

    // Priority 2: Mock token (for testing/development without credentials)
    if let Ok(mock_token) = env::var("DEV_MOCK_AUTH_TOKEN") {
        tracing::warn!(
            env = %ENV.as_str(),
            "using mock authentication token - NOT FOR PRODUCTION"
        );
        return Ok(mock_token);
    }

    // Priority 3: WASI identity (fallback for managed identity scenarios)
    let identity = env::var("AZURE_IDENTITY")?;
    let identity = block_on(get_identity(identity))?;
    let access_token = block_on(async move { identity.get_token(vec![]).await })?;
    Ok(access_token.token)
}

async fn get_cached_azure_token(
    client_id: &str,
    client_secret: &str,
    tenant: &str,
    provider: &Provider,
) -> Result<String> {
    // Check Redis cache first (handles JSON wrapper from WASI keyvalue bug)
    match realtime::StateStore::get(provider, TOKEN_CACHE_KEY).await {
        Ok(Some(cached_data)) => {
            tracing::debug!(raw_data_len = cached_data.len(), "found cached data in Redis");

            // Try to parse as JSON wrapper first (WASI keyvalue format with bug)
            if let Ok(wrapper) = serde_json::from_slice::<CachedValue>(&cached_data) {
                if let Ok(token_str) = String::from_utf8(wrapper.value) && !token_str.is_empty() {
                    tracing::debug!(token_len = token_str.len(), "using cached Azure AD token (unwrapped JSON)");
                    return Ok(token_str);
                }
                tracing::warn!("cached token in JSON wrapper was invalid");
            } else if let Ok(token_str) = String::from_utf8(cached_data) && !token_str.is_empty() {
                // Fallback: try as plain string (for when bug is fixed)
                tracing::debug!(token_len = token_str.len(), "using cached Azure AD token (plain)");
                return Ok(token_str);
            } else {
                tracing::error!("cached data is neither JSON wrapper nor valid UTF-8 string");
            }
        }
        Ok(None) => {
            tracing::debug!("no cached token found in Redis (expired or never set)");
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to read from Redis cache");
        }
    }

    tracing::info!("fetching new Azure AD token");
    let (token, expires_in) = fetch_azure_token(client_id, client_secret, tenant).await?;

    // Cache in Redis with safety margin
    let safety_margin = 600_u64; // 10 minutes
    let cache_ttl = expires_in.saturating_sub(safety_margin);

    match realtime::StateStore::set(provider, TOKEN_CACHE_KEY, token.as_bytes(), Some(cache_ttl)).await {
        Ok(_) => {
            tracing::info!(
                expires_in_seconds = expires_in,
                cache_ttl_seconds = cache_ttl,
                safety_margin_seconds = safety_margin,
                "cached Azure AD token in Redis"
            );
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to cache token in Redis - will fetch new token on every request");
        }
    }

    Ok(token)
}

async fn fetch_azure_token(
    client_id: &str,
    client_secret: &str,
    tenant: &str,
) -> Result<(String, u64)> {
    tracing::debug!(client_id, tenant, "fetching Azure AD token via OAuth 2.0");

    // Allow custom token endpoint or use Microsoft default
    let token_url = env::var("AZURE_TOKEN_URL")
        .unwrap_or_else(|_| format!("https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token"));

    // Allow custom scope or use Azure Management default
    let scope = env::var("AZURE_TOKEN_SCOPE")
        .unwrap_or_else(|_| "https://management.azure.com/.default".to_string());

    tracing::debug!(url = %token_url, scope = %scope, "OAuth 2.0 request parameters");

    // Build form-encoded request body for client credentials flow
    let body = format!(
        "grant_type=client_credentials&client_id={}&client_secret={}&scope={}",
        urlencoding::encode(client_id),
        urlencoding::encode(client_secret),
        urlencoding::encode(&scope)
    );

    let request = http::Request::builder()
        .method(http::Method::POST)
        .uri(&token_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(Full::new(Bytes::from(body)))
        .context("building Azure token request")?;
    
    let response = wasi_http::handle(request)
        .await
        .context("Azure token request failed")?;
    let status = response.status();

    if !status.is_success() {
        let body_bytes = response.into_body();
        let body_str = String::from_utf8_lossy(&body_bytes);
        anyhow::bail!("Azure token request failed with status {status}: {body_str}");
    }

    let body = response.into_body();
    let token_response: AzureTokenResponse = serde_json::from_slice(&body)
        .context("deserializing Azure token response")?;

    tracing::info!("successfully obtained new Azure AD token (expires in {} seconds)", token_response.expires_in);
    Ok((token_response.access_token, token_response.expires_in))
}
