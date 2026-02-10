//! Agent prompt template loading and generation.
//!
//! This module handles loading prompt templates from the `prompts/` directory
//! and substituting variables to create the final prompts sent to agents.
//!
//! # Prompt Types
//!
//! - **fix**: Instructs agents to investigate and fix GitHub issues
//! - **dedupe**: Instructs agents to find duplicate issues
//! - **triage**: Instructs agents to evaluate issues for agent candidacy
//! - **status_reporting**: Appended to all prompts to enable callback reporting
//!
//! Templates use `{variable}` placeholders that are replaced at runtime.

use std::path::PathBuf;

fn get_prompts_dir() -> PathBuf {
    let exe_path = std::env::current_exe().ok();
    let project_root = exe_path
        .as_ref()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let prompts_dir = project_root.join("prompts");
    if prompts_dir.exists() {
        return prompts_dir;
    }

    let cwd_prompts = PathBuf::from("prompts");
    if cwd_prompts.exists() {
        return cwd_prompts;
    }

    prompts_dir
}

fn load_prompt_file(filename: &str) -> Option<String> {
    let prompts_dir = get_prompts_dir();
    let path = prompts_dir.join(filename);
    std::fs::read_to_string(&path).ok()
}

/// Loads the status reporting template and substitutes callback credentials.
fn get_status_reporting_block(callback_token: &str, callback_url: &str) -> String {
    if let Some(template) = load_prompt_file("status_reporting.txt") {
        template
            .replace("{callback_token}", callback_token)
            .replace("{callback_url}", callback_url)
    } else {
        format!(
            "**STATUS REPORTING (REQUIRED):**\n\
            You MUST report your progress using the PowerFixer callback API.\n\n\
            Set up these environment variables at the start:\n\
            ```bash\n\
            export POWERFIXER_CALLBACK_TOKEN=\"{}\"\n\
            export POWERFIXER_CALLBACK_URL=\"{}\"\n\
            ```\n\n\
            Report status via: python3 /workspace/power-fixer-status-update/powerfixer_status.py '{{\"state\": \"IN_PROGRESS\"}}'\n\
            When done: python3 /workspace/power-fixer-status-update/powerfixer_status.py '{{\"state\": \"SUCCEEDED\", \"summary\": \"...\"}}'\n\
            On failure: python3 /workspace/power-fixer-status-update/powerfixer_status.py '{{\"state\": \"FAILED\", \"summary\": \"...\"}}'\n\n\
            **IMPORTANT:** You MUST include a summary field when complete.",
            callback_token, callback_url
        )
    }
}

/// Builds the complete prompt for remote cloud agents by appending status reporting instructions.
///
/// This combines the base prompt (e.g., fix, dedupe, or triage instructions) with the
/// status reporting block that tells the agent how to report progress via callbacks.
///
/// We intentionally use a tiny Python (stdlib-only) HTTP client for status updates instead of `curl`.
/// Some remote environments running cloud agents restrict `curl`, and the ambient runner currently
/// does not fully support the `--profile` flag that would otherwise allow `curl` on those machines.
pub fn get_remote_agent_prompt(
    base_prompt: &str,
    callback_token: &str,
    callback_url: &str,
) -> String {
    let status_block = get_status_reporting_block(callback_token, callback_url);
    format!("{}\n\n{}", base_prompt, status_block)
}

/// Builds the base prompt for fix agents.
///
/// Loads the `fix.txt` template and substitutes the issue URL and any
/// additional instructions provided by the user.
pub fn get_fix_base_prompt(issue_url: &str, additional_prompt: &str) -> String {
    if let Some(template) = load_prompt_file("fix.txt") {
        let prompt = template
            .replace("{issue_url}", issue_url)
            .replace("{additional_prompt}", additional_prompt);
        prompt.trim().to_string()
    } else if additional_prompt.is_empty() {
        format!(
            "Investigate the GitHub issue at {}. Read the issue carefully and understand the problem.",
            issue_url
        )
    } else {
        format!(
            "Investigate the GitHub issue at {}. Read the issue carefully and understand the problem. {}",
            issue_url, additional_prompt
        )
    }
}

/// Builds the base prompt for dedupe agents.
///
/// Loads the `dedupe.txt` template and substitutes repo and issue number.
/// The agent fetches issue content live via `gh issue view` to prevent prompt injection.
pub fn get_dedupe_base_prompt(repo: &str, external_id: &str, additional_prompt: &str) -> String {
    let base = if let Some(template) = load_prompt_file("dedupe.txt") {
        template
            .replace("{repo}", repo)
            .replace("{issue_number}", external_id)
    } else {
        format!(
            r#"You are a GitHub issue duplicate finder. Your task is to find issues that are duplicates of issue {external_id} in the {repo} repository.

First, set up your GitHub token (read-only access to public repos):
```bash
export GH_TOKEN=$POWERFIXER_GH_TOKEN_READONLY
```

Fetch the issue you need to find duplicates for:
```bash
gh issue view {external_id} --repo {repo}
```

Task:
1. Use gh to search for related issues (multiple strategies).
2. Inspect candidates with `gh issue view <number> --repo {repo}`.
3. Decide the canonical issue to keep open (often the oldest or most complete).

When done, report results via callback (do NOT post visible comments)."#,
            external_id = external_id,
            repo = repo,
        )
    };

    if additional_prompt.is_empty() {
        base
    } else {
        log::debug!(
            "Appending additional dedupe prompt ({} chars): '{}'",
            additional_prompt.len(),
            additional_prompt
        );
        format!(
            "{}\n\nAdditional context from user:\n{}",
            base, additional_prompt
        )
    }
}

/// Builds the complete triage prompt including status reporting instructions.
///
/// Unlike fix/dedupe which use `get_remote_agent_prompt` to append status reporting,
/// triage prompts include callback credentials directly in the template.
pub fn get_triage_prompt(
    repo: &str,
    issue_nums_str: &str,
    callback_token: &str,
    callback_url: &str,
) -> String {
    if let Some(template) = load_prompt_file("triage.txt") {
        template
            .replace("{repo}", repo)
            .replace("{issue_nums_str}", issue_nums_str)
            .replace("{callback_token}", callback_token)
            .replace("{callback_url}", callback_url)
    } else {
        format!(
            r#"You are a GitHub issue triage agent for the Warp terminal application.

**Repository:** {repo}
**Issues to analyze:** {issue_nums_str}

Set up environment:
```bash
export GH_TOKEN=$POWERFIXER_GH_TOKEN
export POWERFIXER_CALLBACK_TOKEN="{callback_token}"
export POWERFIXER_CALLBACK_URL="{callback_url}"
```

Analyze each issue and determine which are good agent candidates.
Report results via callback API with candidates and rejected arrays."#,
            repo = repo,
            issue_nums_str = issue_nums_str,
            callback_token = callback_token,
            callback_url = callback_url
        )
    }
}
