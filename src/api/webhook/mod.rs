//! Webhook endpoints for external integrations.
//!
//! This module handles webhooks from GitHub Actions and other external sources.
//! Endpoints are authenticated via a shared API key rather than IAP.

pub mod dedupe;

use axum::http::HeaderMap;
use std::env;

pub use dedupe::webhook_dedupe;

/// Validates the webhook API key from the X-Webhook-Api-Key header.
///
/// Returns true if the key matches the configured POWERFIXER_WEBHOOK_API_KEY.
pub fn validate_webhook_api_key(headers: &HeaderMap) -> bool {
    let expected_key = match env::var("POWERFIXER_WEBHOOK_API_KEY") {
        Ok(key) if !key.is_empty() => key,
        _ => {
            log::warn!("POWERFIXER_WEBHOOK_API_KEY not configured");
            return false;
        }
    };

    let provided_key = headers
        .get("x-webhook-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if provided_key.is_empty() {
        log::debug!("Missing X-Webhook-Api-Key header");
        return false;
    }

    let valid = provided_key == expected_key;
    if !valid {
        log::debug!("Invalid webhook API key provided");
    }
    valid
}
