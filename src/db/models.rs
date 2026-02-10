//! Database models and enum types.
//!
//! This module defines all SQLx-compatible data structures that map to database tables.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Source system for issues (GitHub or Linear).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "issue_provider", rename_all = "lowercase")]
#[allow(dead_code)]
pub enum IssueProvider {
    Github,
    Linear,
}

#[allow(dead_code)]
impl IssueProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            IssueProvider::Github => "github",
            IssueProvider::Linear => "linear",
        }
    }
}

/// Type of agent task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "agent_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AgentType {
    Fix,
    Dedupe,
    Triage,
}

impl AgentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentType::Fix => "fix",
            AgentType::Dedupe => "dedupe",
            AgentType::Triage => "triage",
        }
    }
}

/// How the agent is executed (local or remote).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "execution_mode", rename_all = "lowercase")]
pub enum ExecutionMode {
    Local,
    Remote,
}

impl ExecutionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionMode::Local => "local",
            ExecutionMode::Remote => "remote",
        }
    }
}

/// Execution state of an agent task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "agent_task_state", rename_all = "snake_case")]
pub enum AgentTaskState {
    Queued,
    InProgress,
    Succeeded,
    Failed,
}

impl AgentTaskState {
    pub fn display_name(&self) -> &'static str {
        match self {
            AgentTaskState::Queued => "QUEUED",
            AgentTaskState::InProgress => "IN_PROGRESS",
            AgentTaskState::Succeeded => "SUCCEEDED",
            AgentTaskState::Failed => "FAILED",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, AgentTaskState::Succeeded | AgentTaskState::Failed)
    }
}

/// Outcome of triage evaluation for an issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "triage_result_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TriageResultType {
    Candidate,
    Rejected,
}

impl TriageResultType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TriageResultType::Candidate => "candidate",
            TriageResultType::Rejected => "rejected",
        }
    }
}

/// Source of agent trigger (TUI or GitHub webhook).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[sqlx(type_name = "trigger_source", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TriggerSource {
    #[default]
    Tui,
    GithubWebhook,
}

impl TriggerSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            TriggerSource::Tui => "tui",
            TriggerSource::GithubWebhook => "github_webhook",
        }
    }
}

/// Type of action taken on an issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "issue_action_type", rename_all = "snake_case")]
pub enum IssueActionType {
    Comment,
    Close,
    LabelAdd,
    LabelRemove,
    Assign,
    Other,
}

// =============================================================================
// Provider Config
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ProviderConfig {
    pub id: i32,
    pub provider: IssueProvider,
    pub organization: String,
    pub base_url: Option<String>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// Issues
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbIssue {
    pub id: i32,
    pub provider_config_id: i32,
    pub external_id: String,
    pub external_url: Option<String>,
    pub project: String,
    pub title: Option<String>,
    pub labels: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewIssue {
    pub provider_config_id: i32,
    pub external_id: String,
    pub external_url: Option<String>,
    pub project: String,
    pub title: Option<String>,
    pub labels: Vec<String>,
}

// =============================================================================
// Unified Agents
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbAgent {
    pub id: i32,
    pub agent_type: AgentType,
    pub execution_mode: ExecutionMode,
    pub trigger_issue_id: Option<i32>,
    pub task_id: Option<String>,
    pub callback_token: String,
    pub prompt: String,
    pub task_state: AgentTaskState,
    pub session_url: Option<String>,
    pub branch_name: Option<String>,
    pub pr_url: Option<String>,
    pub summary: Option<String>,
    pub pid: Option<i32>,
    pub log_path: Option<String>,
    pub trigger_source: TriggerSource,
    pub triggered_by: Option<String>,
    pub started_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAgent {
    pub agent_type: AgentType,
    pub execution_mode: ExecutionMode,
    pub trigger_issue_id: Option<i32>,
    pub task_id: Option<String>,
    pub callback_token: String,
    pub prompt: String,
    pub task_state: AgentTaskState,
    pub pid: Option<i32>,
    pub log_path: Option<String>,
    pub trigger_source: TriggerSource,
    pub triggered_by: Option<String>,
    pub started_at: DateTime<Utc>,
}

// =============================================================================
// Agent Status Updates
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbAgentStatusUpdate {
    pub id: i32,
    pub agent_id: i32,
    pub state: String,
    pub message: Option<String>,
    pub received_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAgentStatusUpdate {
    pub agent_id: i32,
    pub state: String,
    pub message: Option<String>,
}

// =============================================================================
// Issue Actions
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbIssueAction {
    pub id: i32,
    pub issue_id: i32,
    pub action_type: IssueActionType,
    pub details: Option<serde_json::Value>,
    pub performed_by: String,
    pub performed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewIssueAction {
    pub issue_id: i32,
    pub action_type: IssueActionType,
    pub details: Option<serde_json::Value>,
    pub performed_by: String,
}

// =============================================================================
// Triage Runs
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbTriageRun {
    pub id: i32,
    pub started_at: DateTime<Utc>,
    pub min_external_id: String,
    pub max_external_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTriageRun {
    pub started_at: DateTime<Utc>,
    pub min_external_id: String,
    pub max_external_id: String,
}

// =============================================================================
// Triage Run Agents
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbTriageRunAgent {
    pub id: i32,
    pub triage_run_id: i32,
    pub agent_id: i32,
    pub external_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTriageRunAgent {
    pub triage_run_id: i32,
    pub agent_id: i32,
    pub external_ids: Vec<String>,
}

// =============================================================================
// Triage Results
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbTriageResult {
    pub id: i32,
    pub triage_run_id: i32,
    pub agent_id: i32,
    pub external_id: String,
    pub result: TriageResultType,
    pub reason: String,
    pub evaluated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTriageResult {
    pub triage_run_id: i32,
    pub agent_id: i32,
    pub external_id: String,
    pub result: TriageResultType,
    pub reason: String,
}

// =============================================================================
// Dedupe Runs
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbDedupeRun {
    pub id: i32,
    pub agent_id: i32,
    pub canonical_issue_url: String,
    pub analysis_summary: Option<String>,
    pub is_addressed: bool,
    pub addressed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDedupeRun {
    pub agent_id: i32,
    pub canonical_issue_url: String,
    pub analysis_summary: Option<String>,
}

// =============================================================================
// Dedupe Duplicates
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbDedupeDuplicate {
    pub id: i32,
    pub dedupe_run_id: i32,
    pub issue_url: String,
    pub confidence: f32,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDedupeDuplicate {
    pub dedupe_run_id: i32,
    pub issue_url: String,
    pub confidence: f32,
    pub reason: String,
}

// =============================================================================
// Dedupe Closures
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbDedupeClosure {
    pub id: i32,
    pub dedupe_run_id: i32,
    pub closed_issue_url: String,
    pub closed_at: DateTime<Utc>,
    pub closed_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDedupeClosure {
    pub dedupe_run_id: i32,
    pub closed_issue_url: String,
    pub closed_by: String,
}

// =============================================================================
// Inbox State
// =============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbAgentInboxState {
    pub id: i32,
    pub agent_id: i32,
    pub is_archived: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct DbIssueInboxState {
    pub id: i32,
    pub issue_id: i32,
    pub is_archived: bool,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// Utility Functions
// =============================================================================

pub fn generate_callback_token() -> String {
    Uuid::new_v4().to_string()
}
