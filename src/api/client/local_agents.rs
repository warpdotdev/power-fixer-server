//! Local agent CRUD endpoints.
//!
//! This module handles creating, deleting, and updating local agent records
//! for agents running on the user's local machine.

use axum::{
    extract::{Json, Path, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::super::types::{ApiState, DEFAULT_PROJECT, DEFAULT_PROVIDER_CONFIG_ID};
use super::super::websocket::{broadcast_agent_deleted, broadcast_agent_update};
use crate::db::models::{
    generate_callback_token, AgentTaskState, AgentType, ExecutionMode, NewAgent, NewIssue,
    TriggerSource,
};
use crate::db::queries;

#[derive(Debug, Deserialize)]
pub struct CreateLocalAgentRequest {
    #[serde(default)]
    pub external_id: Option<String>,
    #[serde(default)]
    pub issue_title: Option<String>,
    #[serde(default)]
    pub issue_url: Option<String>,
    pub prompt: String,
    #[serde(default)]
    pub log_path: Option<String>,
    #[serde(default)]
    pub callback_token: Option<String>,
    #[serde(default)]
    pub triggered_by: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateLocalAgentResponse {
    pub success: bool,
    pub id: Option<i32>,
    pub message: String,
}

/// Creates a new local agent record.
pub async fn create_local_agent(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<CreateLocalAgentRequest>,
) -> impl IntoResponse {
    let callback_token = request
        .callback_token
        .unwrap_or_else(generate_callback_token);

    let trigger_issue_id = if let Some(ref ext_id) = request.external_id {
        let new_issue = NewIssue {
            provider_config_id: DEFAULT_PROVIDER_CONFIG_ID,
            external_id: ext_id.clone(),
            external_url: request.issue_url.clone(),
            project: DEFAULT_PROJECT.to_string(),
            title: request.issue_title.clone(),
            labels: vec![],
        };
        match queries::upsert_issue(&state.pool, &new_issue).await {
            Ok(issue) => Some(issue.id),
            Err(e) => {
                log::warn!("Failed to upsert issue for local agent: {}", e);
                None
            }
        }
    } else {
        None
    };

    let new_agent = NewAgent {
        agent_type: AgentType::Fix,
        execution_mode: ExecutionMode::Local,
        trigger_issue_id,
        task_id: None,
        callback_token,
        prompt: request.prompt,
        task_state: AgentTaskState::InProgress,
        pid: None,
        log_path: request.log_path,
        trigger_source: TriggerSource::Tui,
        triggered_by: request.triggered_by,
        started_at: chrono::Utc::now(),
    };

    match queries::create_agent(&state.pool, &new_agent).await {
        Ok(agent) => {
            broadcast_agent_update(&state.pool, &state.ws_broadcast, agent.id).await;
            json_ok!(CreateLocalAgentResponse {
                success: true,
                id: Some(agent.id),
                message: "Local agent created".to_string(),
            })
        }
        Err(e) => json_err!(
            INTERNAL_SERVER_ERROR,
            CreateLocalAgentResponse {
                success: false,
                id: None,
                message: format!("Failed to create local agent: {}", e),
            }
        ),
    }
}

/// Deletes a local agent by ID.
pub async fn delete_local_agent(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match queries::delete_agent(&state.pool, id).await {
        Ok(_) => {
            broadcast_agent_deleted(&state.ws_broadcast, id);
            ok!("Local agent deleted")
        }
        Err(e) => err!(INTERNAL_SERVER_ERROR, "Failed to delete local agent: {}", e),
    }
}
