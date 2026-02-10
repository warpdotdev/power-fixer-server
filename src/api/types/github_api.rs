//! GitHub API integration via HTTP REST API.

use reqwest::Client;
use serde::Deserialize;
use std::env;

/// Fetches issue metadata (title and body) from GitHub using the REST API.
///
/// This works in environments where the gh CLI is not available (e.g., Cloud Run).
/// Uses GITHUB_TOKEN env var if available for higher rate limits, otherwise uses
/// unauthenticated requests (60 req/hr for public repos).
///
/// # Arguments
/// * `repo` - Repository in format "owner/repo" (e.g. "example-org/example-repo")
/// * `issue_number` - Issue number as string
///
/// # Returns
/// * `Ok((title, body))` on success
/// * `Err(String)` with error message on failure
pub async fn fetch_issue_metadata_async(
    repo: &str,
    issue_number: &str,
) -> Result<(String, String), String> {
    log::debug!(
        "[GITHUB_API] Fetching metadata via HTTP for issue #{} in {}",
        issue_number,
        repo
    );

    let url = format!(
        "https://api.github.com/repos/{}/issues/{}",
        repo, issue_number
    );

    let client = Client::new();
    let mut request = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "power-fixer-server");

    if let Ok(token) = env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("Failed to send GitHub API request: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        log::warn!(
            "[GITHUB_API] GitHub API returned {} for issue #{}: {}",
            status,
            issue_number,
            body
        );
        return Err(format!("GitHub API error: {} - {}", status, body));
    }

    #[derive(Deserialize)]
    struct GitHubIssue {
        title: String,
        body: Option<String>,
    }

    let issue: GitHubIssue = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse GitHub API response: {}", e))?;

    log::debug!(
        "[GITHUB_API] Successfully fetched issue #{} via HTTP: {} ({} chars body)",
        issue_number,
        issue.title,
        issue.body.as_ref().map(|b| b.len()).unwrap_or(0)
    );

    Ok((issue.title, issue.body.unwrap_or_default()))
}

/// Closes a GitHub issue and adds a comment indicating it's a duplicate.
///
/// This uses the GitHub REST API directly, avoiding the need for the gh CLI.
/// The GitHub token must be provided by the caller (typically from the client).
///
/// # Arguments
/// * `repo` - Repository in format "owner/repo" (e.g. "example-org/example-repo")
/// * `issue_number` - Issue number to close
/// * `canonical_issue_number` - The canonical issue this is a duplicate of
/// * `github_token` - GitHub personal access token with `repo` scope
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(String)` with error message on failure
pub async fn close_issue_async(
    repo: &str,
    issue_number: u32,
    canonical_issue_number: u32,
    github_token: &str,
) -> Result<(), String> {
    if github_token.is_empty() {
        return Err("GitHub token is empty".to_string());
    }

    let client = Client::new();

    let close_url = format!(
        "https://api.github.com/repos/{}/issues/{}",
        repo, issue_number
    );
    let close_response = client
        .patch(&close_url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "power-fixer-server")
        .header("Authorization", format!("Bearer {}", github_token))
        .json(&serde_json::json!({
            "state": "closed",
            "state_reason": "not_planned"
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to send close request: {}", e))?;

    if !close_response.status().is_success() {
        let status = close_response.status();
        let body = close_response.text().await.unwrap_or_default();
        return Err(format!("Failed to close issue: {} - {}", status, body));
    }

    let comment_url = format!(
        "https://api.github.com/repos/{}/issues/{}/comments",
        repo, issue_number
    );
    let comment_response = client
        .post(&comment_url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "power-fixer-server")
        .header("Authorization", format!("Bearer {}", github_token))
        .json(&serde_json::json!({
            "body": format!("Duplicate of #{}", canonical_issue_number)
        }))
        .send()
        .await
        .map_err(|e| format!("Issue closed but failed to add comment: {}", e))?;

    if !comment_response.status().is_success() {
        let status = comment_response.status();
        let body = comment_response.text().await.unwrap_or_default();
        log::warn!(
            "Issue #{} closed but comment failed: {} - {}",
            issue_number,
            status,
            body
        );
    }

    Ok(())
}

/// Truncates text to a maximum length, adding "..." if truncated.
pub fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        let truncated = text.chars().take(max_len).collect::<String>();
        format!("{}...", truncated.trim_end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_text_short() {
        assert_eq!(truncate_text("Short text", 100), "Short text");
    }

    #[test]
    fn test_truncate_text_exact() {
        assert_eq!(truncate_text("Exactly10!", 10), "Exactly10!");
    }

    #[test]
    fn test_truncate_text_long() {
        let result = truncate_text("This is a very long text that should be truncated", 20);
        assert_eq!(result, "This is a very long...");
        assert!(result.len() <= 23); // 20 + "..."
    }
}
