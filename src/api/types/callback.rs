//! Types for the agent callback API.
//!
//! This module contains all request/response types used by the callback endpoint
//! that agents use to report their status back to the server.

use serde::{Deserialize, Serialize};

use crate::db::models::{AgentType as DbAgentType, ExecutionMode};

/// Status update payload sent by agents via HTTP callback.
#[derive(Debug, Deserialize)]
pub struct StatusUpdate {
    #[allow(dead_code)]
    pub task_id: Option<String>,
    pub state: String,
    pub branch_name: Option<String>,
    pub pr_url: Option<String>,
    pub session_url: Option<String>,
    pub summary: Option<String>,
    #[allow(dead_code)]
    pub error_message: Option<String>,
    pub canonical_issue_url: Option<String>,
    pub duplicates: Option<Vec<DedupeCandidate>>,
    pub candidates: Option<Vec<String>>,
    pub rejected: Option<Vec<RejectedIssue>>,
}

/// An issue rejected during triage with the reason why.
#[derive(Debug, Deserialize)]
pub struct RejectedIssue {
    pub external_id: String,
    pub reason: String,
}

/// A potential duplicate issue found by a dedupe agent.
#[derive(Debug, Deserialize)]
pub struct DedupeCandidate {
    pub issue_url: String,
    pub confidence: f32,
    pub reason: String,
}

/// Generic API response for success/error operations.
#[derive(Debug, Serialize)]
pub struct GenericResponse {
    pub success: bool,
    pub message: String,
}

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
}

/// Internal: Information about an agent looked up by callback token.
pub(crate) struct AgentInfo {
    pub id: i32,
    pub agent_type: DbAgentType,
    pub execution_mode: ExecutionMode,
    #[allow(dead_code)]
    pub trigger_issue_id: Option<i32>,
    #[allow(dead_code)]
    pub task_id: Option<String>,
    pub triage_run_id: Option<i32>,
}
