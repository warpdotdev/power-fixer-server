//! Agent launching endpoint.
//!
//! This module handles launching new agent tasks on Warp's platform,
//! including prompt generation and database record creation.

use axum::{body::Bytes, extract::State, response::IntoResponse};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;

use super::super::types::{
    fetch_issue_metadata_async, ApiState, DEFAULT_PROJECT, DEFAULT_PROVIDER_CONFIG_ID,
};
use super::super::warp_api::{LaunchAgentRequest as WarpLaunchRequest, TaskConfig, WarpApiError};
use super::super::websocket::broadcast_agent_update;
use crate::config;
use crate::db::models::{
    generate_callback_token, AgentTaskState, AgentType, ExecutionMode, NewAgent, NewIssue,
    TriggerSource,
};
use crate::db::queries;
use crate::prompts::{get_dedupe_base_prompt, get_fix_base_prompt, get_remote_agent_prompt};

#[derive(Debug, Deserialize)]
pub struct LaunchRequest {
    pub external_id: String,
    pub repo: String,
    pub agent_type: AgentType,
    #[serde(default)]
    pub issue_url: Option<String>,
    #[serde(default)]
    pub additional_prompt: Option<String>,
    #[serde(default)]
    pub secrets: Option<Vec<String>>,
    #[serde(default)]
    pub custom_prompt: Option<String>,
    #[serde(default)]
    pub triggered_by: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LaunchResponse {
    pub success: bool,
    pub task_id: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

fn generate_prompt_for_task(
    request: &LaunchRequest,
    callback_token: &str,
    callback_url: &str,
) -> String {
    match request.agent_type {
        AgentType::Fix => {
            let issue_url = request.issue_url.clone().unwrap_or_else(|| {
                format!(
                    "https://github.com/{}/issues/{}",
                    request.repo, request.external_id
                )
            });
            let base_prompt = get_fix_base_prompt(
                &issue_url,
                request.additional_prompt.as_deref().unwrap_or(""),
            );
            get_remote_agent_prompt(&base_prompt, callback_token, callback_url)
        }
        AgentType::Dedupe => {
            let additional = request.additional_prompt.as_deref().unwrap_or("");
            let base_prompt =
                get_dedupe_base_prompt(&request.repo, &request.external_id, additional);
            get_remote_agent_prompt(&base_prompt, callback_token, callback_url)
        }
        AgentType::Triage => {
            unreachable!("Triage agents should use /triage/run endpoint")
        }
    }
}

/// Launches a new agent task on Warp's platform.
pub async fn launch_agent(State(state): State<Arc<ApiState>>, body: Bytes) -> impl IntoResponse {
    let body_str = String::from_utf8_lossy(&body);
    log::debug!("Raw request body: {}", body_str);

    let request: LaunchRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            log::warn!(
                "Failed to deserialize request: {} - body was: {}",
                e,
                body_str
            );
            return json_err!(
                UNPROCESSABLE_ENTITY,
                LaunchResponse {
                    success: false,
                    task_id: None,
                    message: format!(
                        "Failed to deserialize the JSON body into the target type: {}",
                        e
                    ),
                    prompt: None,
                }
            );
        }
    };

    log::info!(
        "Received launch request for issue {} in {} (type: {:?})",
        request.external_id,
        request.repo,
        request.agent_type
    );
    if let Some(ref additional) = request.additional_prompt {
        log::info!("Additional prompt provided: '{}'", additional);
    }

    if request.agent_type == AgentType::Triage {
        return json_err!(
            BAD_REQUEST,
            LaunchResponse {
                success: false,
                task_id: None,
                message: "Triage agents must be launched via POST /api/v1/triage/run endpoint"
                    .to_string(),
                prompt: None,
            }
        );
    }

    let warp_client = match &state.warp_client {
        Some(client) => client,
        None => {
            log::warn!("No Warp API client available");
            return json_err!(
                SERVICE_UNAVAILABLE,
                LaunchResponse {
                    success: false,
                    task_id: None,
                    message: "No Warp API key configured".to_string(),
                    prompt: None,
                }
            );
        }
    };

    let callback_token = generate_callback_token();
    let callback_url =
        env::var("POWERFIXER_CALLBACK_URL").unwrap_or_else(|_| config::default_callback_url());

    let prompt = if let Some(custom) = &request.custom_prompt {
        get_remote_agent_prompt(custom, &callback_token, &callback_url)
    } else {
        generate_prompt_for_task(&request, &callback_token, &callback_url)
    };

    log::debug!(
        "Generated prompt ({} chars) for agent type: {:?}",
        prompt.len(),
        request.agent_type
    );

    let environment_id = match request.agent_type {
        AgentType::Dedupe => match config::dedupe_environment_id() {
            Some(v) => v,
            None => {
                return json_err!(
                    SERVICE_UNAVAILABLE,
                    LaunchResponse {
                        success: false,
                        task_id: None,
                        message:
                            "POWERFIXER_DEDUPE_ENVIRONMENT_ID (or POWERFIXER_ENVIRONMENT_ID) is not configured"
                                .to_string(),
                        prompt: None,
                    }
                );
            }
        },
        _ => match config::environment_id() {
            Some(v) => v,
            None => {
                return json_err!(
                    SERVICE_UNAVAILABLE,
                    LaunchResponse {
                        success: false,
                        task_id: None,
                        message: "POWERFIXER_ENVIRONMENT_ID is not configured".to_string(),
                        prompt: None,
                    }
                );
            }
        },
    };
    let task_config = Some(TaskConfig { environment_id });

    let secrets = match request.agent_type {
        AgentType::Dedupe => request
            .secrets
            .clone()
            .or_else(|| config::dedupe_secret_name().map(|name| vec![name])),
        _ => request.secrets.clone(),
    };

    let warp_request = WarpLaunchRequest {
        prompt: prompt.clone(),
        config: task_config,
        agent_profile_id: config::agent_profile_id(),
        secrets,
        team: Some(config::team_scoped_launch()),
    };

    log::debug!("Calling Warp API to launch agent...");
    log::debug!("Using Warp API URL: {}", warp_client.base_url());

    let task_id = match warp_client.launch_agent(warp_request).await {
        Ok(response) => {
            log::info!("Warp API returned task_id: {}", response.task_id);
            response.task_id
        }
        Err(WarpApiError::ApiError { status, body }) => {
            log::error!("Warp API error: {} - {}", status, body);
            return json_err!(
                BAD_GATEWAY,
                LaunchResponse {
                    success: false,
                    task_id: None,
                    message: format!("Warp API error: {} - {}", status, body),
                    prompt: None,
                }
            );
        }
        Err(e) => {
            log::error!("Failed to call Warp API: {}", e);
            return json_err!(
                BAD_GATEWAY,
                LaunchResponse {
                    success: false,
                    task_id: None,
                    message: format!("Failed to call Warp API: {}", e),
                    prompt: None,
                }
            );
        }
    };

    let project = request
        .repo
        .split('/')
        .next_back()
        .unwrap_or(DEFAULT_PROJECT);
    let issue_url = request.issue_url.clone().unwrap_or_else(|| {
        format!(
            "https://github.com/{}/issues/{}",
            request.repo, request.external_id
        )
    });
    let kickoff_title = match fetch_issue_metadata_async(&request.repo, &request.external_id).await
    {
        Ok((t, _body)) => Some(t),
        Err(e) => {
            log::warn!("[LAUNCH] Failed to fetch issue metadata: {}", e);
            None
        }
    };

    let new_issue = NewIssue {
        provider_config_id: DEFAULT_PROVIDER_CONFIG_ID,
        external_id: request.external_id.clone(),
        external_url: Some(issue_url.clone()),
        project: project.to_string(),
        title: kickoff_title.clone(),
        labels: vec![],
    };

    log::debug!("Storing agent in database for task {}", task_id);

    match queries::upsert_issue(&state.pool, &new_issue).await {
        Ok(db_issue) => {
            let new_agent = NewAgent {
                agent_type: request.agent_type,
                execution_mode: ExecutionMode::Remote,
                trigger_issue_id: Some(db_issue.id),
                task_id: Some(task_id.clone()),
                callback_token,
                prompt: prompt.clone(),
                task_state: AgentTaskState::Queued,
                pid: None,
                log_path: None,
                trigger_source: TriggerSource::Tui,
                triggered_by: request.triggered_by.clone(),
                started_at: chrono::Utc::now(),
            };
            match queries::create_agent(&state.pool, &new_agent).await {
                Ok(created_agent) => {
                    log::info!("Agent created for issue {}", request.external_id);
                    broadcast_agent_update(&state.pool, &state.ws_broadcast, created_agent.id)
                        .await;
                }
                Err(e) => {
                    log::error!("Failed to create agent: {}", e);
                }
            }
        }
        Err(e) => {
            log::error!("Failed to upsert issue: {}", e);
        }
    }

    log::info!(
        "Launch complete: task_id={} for issue {}",
        task_id,
        request.external_id
    );

    json_ok!(LaunchResponse {
        success: true,
        task_id: Some(task_id),
        message: "Agent launched successfully".to_string(),
        prompt: Some(prompt),
    })
}
