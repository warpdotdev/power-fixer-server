//! Runtime configuration helpers.

use std::env;

pub fn default_github_org() -> String {
    env::var("POWERFIXER_DEFAULT_GITHUB_ORG").unwrap_or_else(|_| "example-org".to_string())
}

pub fn default_project() -> String {
    env::var("POWERFIXER_DEFAULT_PROJECT").unwrap_or_else(|_| "example-repo".to_string())
}

pub fn default_callback_url() -> String {
    env::var("POWERFIXER_CALLBACK_URL").unwrap_or_else(|_| "http://localhost:3001".to_string())
}

/// Default Warp API base URL used when `WARP_API_BASE_URL` is unset.
///
/// `WARP_API_BASE_URL` is honored verbatim (scheme, host, port, and path), so it
/// may point at an internal Private Service Connect endpoint such as
/// `http://10.1.2.3/api/v1` over plain HTTP, with no TLS or host-derivation
/// assumptions.
pub const DEFAULT_WARP_API_BASE_URL: &str = "https://warp.dev/api/v1";

pub fn warp_api_base_url() -> String {
    resolve_warp_api_base_url(env::var("WARP_API_BASE_URL").ok())
}

/// Resolves the Warp API base URL from an optional raw env value, falling back to
/// [`DEFAULT_WARP_API_BASE_URL`] when the variable is unset.
fn resolve_warp_api_base_url(raw: Option<String>) -> String {
    raw.unwrap_or_else(|| DEFAULT_WARP_API_BASE_URL.to_string())
}

pub fn warp_api_key() -> Option<String> {
    env::var("WARP_API_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty())
}

pub fn gcp_project() -> Option<String> {
    env::var("POWERFIXER_GCP_PROJECT")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

pub fn warp_api_key_secret_name() -> String {
    env::var("POWERFIXER_WARP_API_KEY_SECRET")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "powerfixer-warp-api-key".to_string())
}

pub fn environment_id() -> Option<String> {
    env::var("POWERFIXER_ENVIRONMENT_ID")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

pub fn dedupe_environment_id() -> Option<String> {
    env::var("POWERFIXER_DEDUPE_ENVIRONMENT_ID")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(environment_id)
}

pub fn agent_profile_id() -> Option<String> {
    env::var("POWERFIXER_AGENT_PROFILE_ID")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

pub fn triage_secret_name() -> Option<String> {
    env::var("POWERFIXER_TRIAGE_SECRET_NAME")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

pub fn dedupe_secret_name() -> Option<String> {
    env::var("POWERFIXER_DEDUPE_SECRET_NAME")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

pub fn team_scoped_launch() -> bool {
    matches!(
        env::var("POWERFIXER_TEAM_SCOPED").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warp_api_base_url_falls_back_to_default_when_unset() {
        assert_eq!(resolve_warp_api_base_url(None), DEFAULT_WARP_API_BASE_URL);
    }

    #[test]
    fn warp_api_base_url_honors_public_override() {
        let public = "https://staging.warp.dev/api/v1".to_string();
        assert_eq!(resolve_warp_api_base_url(Some(public.clone())), public);
    }

    #[test]
    fn warp_api_base_url_honors_internal_psc_http_endpoint() {
        // Internal PSC endpoints use a raw IP over plain HTTP with no TLS.
        let internal = "http://10.1.2.3/api/v1".to_string();
        assert_eq!(resolve_warp_api_base_url(Some(internal.clone())), internal);
    }
}
