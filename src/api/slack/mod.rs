//! Slack integration for broadcasting agent events.
//!
//! This module provides a client for posting messages to Slack channels
//! when agents are started or change state.

use reqwest::Client;
use serde::Serialize;
use std::env;

use crate::api::openai::{summarize_or_truncate, OpenAIClient};
use crate::db::models::{AgentTaskState, AgentType};

const SLACK_CHANNEL_ID: &str = "C0A8R9176F8";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SlackMode {
    Disabled,
    Local,
    Production,
}

impl SlackMode {
    pub fn from_env() -> Self {
        match env::var("SLACK_MODE").as_deref() {
            Ok("local") => SlackMode::Local,
            Ok("production") => SlackMode::Production,
            _ => SlackMode::Disabled,
        }
    }
}

#[derive(Debug)]
pub struct SlackClient {
    client: Client,
    token: String,
    mode: SlackMode,
}

#[derive(Debug, Serialize)]
struct SlackPostMessage {
    channel: String,
    text: String,
}

impl SlackClient {
    pub fn new() -> Result<Self, String> {
        let mode = SlackMode::from_env();
        if mode == SlackMode::Disabled {
            return Err("SLACK_MODE is disabled".to_string());
        }

        let token = env::var("SLACK_BOT_TOKEN")
            .map_err(|_| "SLACK_BOT_TOKEN environment variable not set".to_string())?;

        if token.is_empty() {
            return Err("SLACK_BOT_TOKEN is empty".to_string());
        }

        Ok(Self {
            client: Client::new(),
            token,
            mode,
        })
    }

    pub async fn post_message(&self, text: &str) -> Result<(), String> {
        let final_text = if self.mode == SlackMode::Local {
            format!("[LOCAL DEBUGGING]\n{}", text)
        } else {
            text.to_string()
        };

        let message = SlackPostMessage {
            channel: SLACK_CHANNEL_ID.to_string(),
            text: final_text,
        };

        let response = self
            .client
            .post("https://slack.com/api/chat.postMessage")
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(&message)
            .send()
            .await
            .map_err(|e| format!("Failed to send Slack message: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Slack API returned error status: {}",
                response.status()
            ));
        }

        Ok(())
    }
}

pub struct AgentStateChangeInfo {
    pub agent_type: AgentType,
    pub task_state: AgentTaskState,
    pub issue_url: Option<String>,
    pub issue_number: Option<String>,
    pub session_url: Option<String>,
    pub branch_name: Option<String>,
    pub pr_url: Option<String>,
}

pub struct DedupeDuplicateInfo {
    pub issue_url: String,
    pub issue_number: Option<String>,
    pub issue_title: Option<String>,
    pub issue_body: Option<String>,
    pub confidence: f32,
    #[allow(dead_code)]
    pub reason: String,
}

pub struct AgentDedupeCompletedInfo {
    pub task_state: AgentTaskState,
    pub session_url: Option<String>,
    pub trigger_issue_url: Option<String>,
    pub trigger_issue_number: Option<String>,
    pub trigger_issue_title: Option<String>,
    pub trigger_issue_body: Option<String>,
    pub canonical_issue_url: Option<String>,
    pub canonical_issue_number: Option<String>,
    pub canonical_issue_title: Option<String>,
    pub canonical_issue_body: Option<String>,
    pub duplicates: Vec<DedupeDuplicateInfo>,
}

pub fn format_agent_state_change_message(info: &AgentStateChangeInfo) -> String {
    let emoji = match info.task_state {
        AgentTaskState::Queued => "🚀",
        AgentTaskState::InProgress => "⏳",
        AgentTaskState::Succeeded => "✅",
        AgentTaskState::Failed => "❌",
    };

    let agent_type_str = match info.agent_type {
        AgentType::Fix => "Fix",
        AgentType::Dedupe => "Dedupe",
        AgentType::Triage => "Triage",
    };

    let state_str = info.task_state.display_name();

    let mut lines = vec![format!(
        "{} *{}* agent {}",
        emoji, agent_type_str, state_str
    )];

    if let Some(ref url) = info.issue_url {
        let display = info
            .issue_number
            .as_ref()
            .map(|n| format!("#{}", n))
            .unwrap_or_else(|| url.clone());
        lines.push(format!("• Issue: <{}|{}>", url, display));
    }

    if let Some(ref session) = info.session_url {
        lines.push(format!("• Session: <{}|View>", session));
    }

    if let Some(ref branch) = info.branch_name {
        lines.push(format!("• Branch: `{}`", branch));
    }

    if let Some(ref pr) = info.pr_url {
        lines.push(format!("• PR: <{}|View>", pr));
    }

    lines.join("\n")
}

pub async fn format_dedupe_completed_message(
    info: &AgentDedupeCompletedInfo,
    openai_client: Option<&OpenAIClient>,
) -> String {
    let emoji = match info.task_state {
        AgentTaskState::Succeeded => "✅",
        AgentTaskState::Failed => "❌",
        _ => "⏳",
    };

    let state_str = info.task_state.display_name();
    let mut lines = vec![format!("{} *Dedupe* agent {}", emoji, state_str)];

    lines.push("".to_string());

    if let Some(ref trigger_url) = info.trigger_issue_url {
        let mut trigger_display = info
            .trigger_issue_number
            .as_ref()
            .map(|n| format!("#{}", n))
            .unwrap_or_else(|| trigger_url.clone());

        if let Some(ref title) = info.trigger_issue_title {
            trigger_display = format!("{} - {}", trigger_display, title);
        }

        let label = if info.duplicates.is_empty() {
            "📋 *Bug searched:*"
        } else {
            "📋 *Bug deduped:*"
        };
        lines.push(label.to_string());
        lines.push(format!("   <{}|{}>", trigger_url, trigger_display));

        if let Some(ref body) = info.trigger_issue_body {
            let summary = summarize_or_truncate(openai_client, body, 200).await;
            if !summary.is_empty() {
                lines.push(format!("   \"{}\"", summary));
            }
        }

        lines.push("".to_string());
    }

    if !info.duplicates.is_empty() {
        let dup_word = if info.duplicates.len() == 1 {
            "duplicate"
        } else {
            "duplicates"
        };
        lines.push(format!(
            "🔍 *Found {} potential {}:*",
            info.duplicates.len(),
            dup_word
        ));
        for (idx, dup) in info.duplicates.iter().enumerate() {
            let mut dup_display = dup
                .issue_number
                .as_ref()
                .map(|n| format!("#{}", n))
                .unwrap_or_else(|| dup.issue_url.clone());

            if let Some(ref title) = dup.issue_title {
                dup_display = format!("{} - {}", dup_display, title);
            }

            lines.push(format!(
                "   {}. <{}|{}> ({:.2})",
                idx + 1,
                dup.issue_url,
                dup_display,
                dup.confidence
            ));

            if let Some(ref body) = dup.issue_body {
                let summary = summarize_or_truncate(openai_client, body, 100).await;
                if !summary.is_empty() {
                    lines.push(format!("      \"{}\"", summary));
                }
            }
        }

        lines.push("".to_string());
    } else {
        lines.push("🔍 *No duplicates found*".to_string());
        lines.push("".to_string());
    }

    if !info.duplicates.is_empty() {
        let canonical_num = info.canonical_issue_number.as_deref().unwrap_or("<num>");

        if let Some(ref canonical_url) = info.canonical_issue_url {
            let mut canonical_display = format!("#{}", canonical_num);
            if let Some(ref title) = info.canonical_issue_title {
                canonical_display = format!("{} - {}", canonical_display, title);
            }
            lines.push(format!(
                "👑 *Canonical issue:* <{}|{}>",
                canonical_url, canonical_display
            ));

            if let Some(ref body) = info.canonical_issue_body {
                let summary = summarize_or_truncate(openai_client, body, 200).await;
                if !summary.is_empty() {
                    lines.push(format!("   \"{}\"", summary));
                }
            }
        } else {
            lines.push(format!("👑 *Canonical issue:* #{}", canonical_num));
        }

        let mut dupes_to_close: Vec<&str> = info
            .duplicates
            .iter()
            .filter_map(|d| d.issue_number.as_deref())
            .collect();
        if let Some(ref trigger_num) = info.trigger_issue_number {
            dupes_to_close.insert(0, trigger_num.as_str());
        }
        if !dupes_to_close.is_empty() {
            lines.push("".to_string());
            lines.push(format!(
                "💡 *To close duplicates:*\n```power-fixer close-dupes --canonical {} --dupes {}```",
                canonical_num,
                dupes_to_close.join(",")
            ));
        }
    }

    if let Some(ref session) = info.session_url {
        lines.push("".to_string());
        lines.push(format!("• Session: <{}|View>", session));
    }

    lines.join("\n")
}

pub async fn broadcast_agent_state_change(
    slack_client: Option<&SlackClient>,
    info: &AgentStateChangeInfo,
) {
    let Some(client) = slack_client else {
        return;
    };

    let message = format_agent_state_change_message(info);
    if let Err(e) = client.post_message(&message).await {
        log::warn!("Failed to broadcast agent state change to Slack: {}", e);
    }
}

pub async fn broadcast_dedupe_completed(
    slack_client: Option<&SlackClient>,
    openai_client: Option<&OpenAIClient>,
    info: &AgentDedupeCompletedInfo,
) {
    let Some(client) = slack_client else {
        return;
    };

    let message = format_dedupe_completed_message(info, openai_client).await;
    if let Err(e) = client.post_message(&message).await {
        log::warn!("Failed to broadcast dedupe completion to Slack: {}", e);
    }
}
