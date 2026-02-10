//! GitHub-related constants and utilities.

pub const DEFAULT_GITHUB_ORG: &str = "example-org";
pub const DEFAULT_PROJECT: &str = "example-repo";
pub const DEFAULT_PROVIDER_CONFIG_ID: i32 = 1;

/// Constructs a GitHub issue URL from org, project, and issue number.
pub fn github_issue_url(org: &str, project: &str, issue_number: impl std::fmt::Display) -> String {
    format!(
        "https://github.com/{}/{}/issues/{}",
        org, project, issue_number
    )
}
