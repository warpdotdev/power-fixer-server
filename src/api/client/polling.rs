//! Background polling and task status endpoints.
//!
//! This module handles synchronization of agent task states with Warp's API,
//! both through a background polling loop and on-demand API endpoints.

use axum::{
    extract::{Json, Path, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use super::super::types::ApiState;
use super::super::warp_api::{parse_task_state, TaskResponse, WarpApiClient, WarpApiError};
use super::super::websocket::broadcast_agent_update;
use crate::db::models::DbAgent;
use crate::db::queries;
use crate::db::DbPool;

pub const BACKGROUND_POLL_INTERVAL: Duration = Duration::from_secs(30);

/// Updates an agent's state in the database if it has changed.
/// Returns true if the state was updated.
pub async fn sync_agent_from_task(pool: &DbPool, agent: &DbAgent, task: &TaskResponse) -> bool {
    let new_state = match parse_task_state(&task.state) {
        Some(s) => s,
        None => return false,
    };

    if agent.task_state == new_state {
        return false;
    }

    match queries::update_agent_full(
        pool,
        agent.id,
        new_state,
        task.session_link.as_deref(),
        None,
        None,
        None,
    )
    .await
    {
        Ok(_) => true,
        Err(e) => {
            log::error!("Failed to update agent {}: {}", agent.id, e);
            false
        }
    }
}

/// Runs the background polling loop that syncs task states from Warp's API.
pub async fn background_polling_loop(state: Arc<ApiState>) {
    log::info!("Background polling loop started (30 second interval)");

    loop {
        tokio::time::sleep(BACKGROUND_POLL_INTERVAL).await;
        poll_all_active_tasks(&state).await;
    }
}

async fn poll_all_active_tasks(state: &Arc<ApiState>) {
    let warp_client = match &state.warp_client {
        Some(client) => client,
        None => return,
    };

    let pending_agents = queries::get_pending_remote_agents(&state.pool)
        .await
        .unwrap_or_default();

    if pending_agents.is_empty() {
        return;
    }

    log::debug!("Polling {} pending remote agents", pending_agents.len());

    for agent in pending_agents {
        let task_id = match &agent.task_id {
            Some(id) => id.clone(),
            None => continue,
        };

        let updated = poll_and_update_agent(warp_client, &state.pool, &agent, &task_id).await;
        if updated {
            broadcast_agent_update(&state.pool, &state.ws_broadcast, agent.id).await;
        }
    }
}

async fn poll_and_update_agent(
    warp_client: &WarpApiClient,
    pool: &DbPool,
    agent: &DbAgent,
    task_id: &str,
) -> bool {
    let task = match warp_client.get_task(task_id).await {
        Ok(t) => t,
        Err(e) => {
            log::debug!("Task {} fetch error: {}", task_id, e);
            return false;
        }
    };
    let updated = sync_agent_from_task(pool, agent, &task).await;

    if updated {
        log::debug!(
            "Task {} state changed: {:?} -> {}",
            task_id,
            agent.task_state,
            task.state
        );
    }

    updated
}

// ============================================================================
// HTTP Endpoints
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct PollRequest {
    pub task_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PollResponse {
    pub statuses: Vec<PolledTaskStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PolledTaskStatus {
    pub task_id: String,
    pub state: String,
    pub session_link: Option<String>,
}

/// Polls Warp's API for the status of multiple tasks and updates the database.
pub async fn poll_agent_statuses(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<PollRequest>,
) -> impl IntoResponse {
    let warp_client = match &state.warp_client {
        Some(client) => client,
        None => {
            return json_err!(SERVICE_UNAVAILABLE, PollResponse { statuses: vec![] });
        }
    };

    let mut statuses = Vec::new();

    for task_id in &request.task_ids {
        let task = match warp_client.get_task(task_id).await {
            Ok(t) => t,
            Err(_) => continue,
        };

        if let Ok(Some(agent)) = queries::get_agent_by_task_id(&state.pool, task_id).await {
            sync_agent_from_task(&state.pool, &agent, &task).await;
        }

        statuses.push(PolledTaskStatus {
            task_id: task_id.clone(),
            state: task.state,
            session_link: task.session_link,
        });
    }

    json_ok!(PollResponse { statuses })
}

#[derive(Debug, Serialize)]
pub struct TaskStatusResponse {
    pub task_id: String,
    pub state: String,
    pub session_link: Option<String>,
    pub result: Option<String>,
    pub error_message: Option<String>,
}

/// Gets the status of a single task from Warp's API.
pub async fn get_task_status(
    State(state): State<Arc<ApiState>>,
    Path(task_id): Path<String>,
) -> impl IntoResponse {
    log::debug!("Getting status for task {}", task_id);

    let warp_client = match &state.warp_client {
        Some(client) => client,
        None => {
            log::warn!("No Warp API client available");
            return json_err!(
                SERVICE_UNAVAILABLE,
                TaskStatusResponse {
                    task_id,
                    state: "ERROR".to_string(),
                    session_link: None,
                    result: None,
                    error_message: Some("No Warp API key configured".to_string()),
                }
            );
        }
    };

    match warp_client.get_task_detail(&task_id).await {
        Ok(task) => {
            log::debug!("Task {} state: {}", task_id, task.state);
            json_ok!(TaskStatusResponse {
                task_id,
                state: task.state,
                session_link: task.session_link,
                result: task.result,
                error_message: task.error_message,
            })
        }
        Err(WarpApiError::ApiError { status, body }) => {
            log::error!("Warp API error for task {}: {} - {}", task_id, status, body);
            json_err!(
                BAD_GATEWAY,
                TaskStatusResponse {
                    task_id,
                    state: "ERROR".to_string(),
                    session_link: None,
                    result: None,
                    error_message: Some(format!("Warp API error: {} - {}", status, body)),
                }
            )
        }
        Err(e) => {
            log::error!("Failed to call Warp API: {}", e);
            json_err!(
                BAD_GATEWAY,
                TaskStatusResponse {
                    task_id,
                    state: "ERROR".to_string(),
                    session_link: None,
                    result: None,
                    error_message: Some(format!("Failed to call Warp API: {}", e)),
                }
            )
        }
    }
}
