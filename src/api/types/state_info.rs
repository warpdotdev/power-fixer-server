//! Shared state information types.
//!
//! These types are used both by the state synchronization endpoints and
//! by WebSocket messages for real-time updates.

use serde::{Deserialize, Serialize};

use crate::db::models::TriageResultType;

/// Information about an agent for TUI display and WebSocket updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: i32,
    pub agent_type: String,
    pub execution_mode: String,
    pub trigger_issue_id: Option<i32>,
    pub trigger_issue_number: Option<String>,
    pub trigger_issue_title: Option<String>,
    pub trigger_issue_url: Option<String>,
    pub task_id: Option<String>,
    pub task_state: String,
    pub session_url: Option<String>,
    pub branch_name: Option<String>,
    pub pr_url: Option<String>,
    pub summary: Option<String>,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triage_results: Option<Vec<TriageResultInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedupe_result: Option<DedupeResultInfo>,
}

/// Information about a triage run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageRunInfo {
    pub id: i32,
    pub started_at: String,
    pub min_external_id: String,
    pub max_external_id: String,
    pub agent_ids: Vec<i32>,
}

/// Inbox state for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxStateInfo {
    pub agent_id: i32,
    pub is_archived: bool,
}

/// Result from a triage evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageResultInfo {
    pub external_id: String,
    pub result: TriageResultType,
    pub reason: String,
}

/// Result from a dedupe analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupeResultInfo {
    pub canonical_issue_url: String,
    pub analysis_summary: Option<String>,
    pub duplicates: Vec<DuplicateCandidateInfo>,
    pub is_addressed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addressed_at: Option<String>,
    pub closures: Vec<DedupeClosureInfo>,
}

/// A closure record from a dedupe run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupeClosureInfo {
    pub closed_issue_url: String,
    pub closed_issue_number: u32,
    pub closed_at: String,
}

impl DedupeClosureInfo {
    pub fn extract_issue_number(url: &str) -> Option<u32> {
        url.rsplit('/').next().and_then(|s| s.parse().ok())
    }
}

/// A potential duplicate issue candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateCandidateInfo {
    pub issue_url: String,
    pub issue_number: u32,
    pub confidence: f32,
    pub reason: String,
}

impl DuplicateCandidateInfo {
    pub fn extract_issue_number(url: &str) -> Option<u32> {
        url.rsplit('/').next().and_then(|s| s.parse().ok())
    }
}
