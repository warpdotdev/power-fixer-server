//! Request and response types for Warp's REST API.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::db::models::AgentTaskState;

/// Error type for Warp API operations.
#[derive(Debug)]
pub enum WarpApiError {
    NoApiKey,
    HttpError(reqwest::Error),
    ApiError { status: u16, body: String },
    ParseError(String),
}

impl fmt::Display for WarpApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoApiKey => write!(f, "No Warp API key configured"),
            Self::HttpError(e) => write!(f, "HTTP error: {}", e),
            Self::ApiError { status, body } => write!(f, "Warp API error {}: {}", status, body),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for WarpApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::HttpError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for WarpApiError {
    fn from(err: reqwest::Error) -> Self {
        Self::HttpError(err)
    }
}

/// Request to launch a new agent task.
#[derive(Debug, Serialize)]
pub struct LaunchAgentRequest {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<TaskConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secrets: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<bool>,
}

/// Configuration for launching a task.
#[derive(Debug, Serialize)]
pub struct TaskConfig {
    pub environment_id: String,
}

/// Response from launching an agent task.
#[derive(Debug, Deserialize)]
pub struct LaunchAgentResponse {
    pub task_id: String,
}

/// Response from getting task status (basic).
#[derive(Debug, Deserialize)]
pub struct TaskResponse {
    #[allow(dead_code)]
    pub task_id: String,
    pub state: String,
    pub session_link: Option<String>,
}

/// Response from getting task status (detailed).
#[derive(Debug, Deserialize)]
pub struct TaskDetailResponse {
    #[allow(dead_code)]
    pub task_id: String,
    pub state: String,
    pub session_link: Option<String>,
    pub result: Option<String>,
    pub error_message: Option<String>,
}

/// Parses a Warp API state string into our internal AgentTaskState enum.
pub fn parse_task_state(state: &str) -> Option<AgentTaskState> {
    match state.to_uppercase().as_str() {
        "QUEUED" => Some(AgentTaskState::Queued),
        "IN_PROGRESS" | "INPROGRESS" | "RUNNING" => Some(AgentTaskState::InProgress),
        "SUCCEEDED" | "SUCCESS" | "COMPLETED" => Some(AgentTaskState::Succeeded),
        "FAILED" | "FAILURE" => Some(AgentTaskState::Failed),
        _ => None,
    }
}
