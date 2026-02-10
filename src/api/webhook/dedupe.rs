//! Webhook endpoint for triggering dedupe agents from GitHub Actions.
//!
//! This endpoint is called when a new issue is opened on the public issues repo.
//! It launches a dedupe agent in a codeless environment to find duplicate issues.

use axum::{
    extract::{Json, State},
    http::HeaderMap,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;

use super::validate_webhook_api_key;
use crate::api::types::{
    fetch_issue_metadata_async, ApiState, DEFAULT_PROJECT, DEFAULT_PROVIDER_CONFIG_ID,
};
use crate::api::warp_api::{LaunchAgentRequest as WarpLaunchRequest, TaskConfig, WarpApiError};
use crate::api::websocket::broadcast_agent_update;
use crate::config;
use crate::db::models::{
    generate_callback_token, AgentTaskState, AgentType, ExecutionMode, NewAgent, NewIssue,
    TriggerSource,
};
use crate::db::queries;
use crate::prompts::{get_dedupe_base_prompt, get_remote_agent_prompt};

#[derive(Debug, Deserialize)]
pub struct WebhookDedupeRequest {
    pub issue_number: i32,
    #[serde(default = "default_repo")]
    pub repo: String,
}

fn default_repo() -> String {
    format!(
        "{}/{}",
        crate::config::default_github_org(),
        crate::config::default_project()
    )
}

#[derive(Debug, Serialize)]
pub struct WebhookDedupeResponse {
    pub success: bool,
    pub task_id: Option<String>,
    pub agent_id: Option<i32>,
    pub message: String,
}

/// Webhook endpoint to trigger a dedupe agent for a newly opened issue.
///
/// Authentication: X-Webhook-Api-Key header must contain the configured API key.
/// Environment: Runs in a codeless environment (no source checkout) to prevent prompt injection.
pub async fn webhook_dedupe(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<WebhookDedupeRequest>,
) -> impl IntoResponse {
    if !validate_webhook_api_key(&headers) {
        return json_err!(
            UNAUTHORIZED,
            WebhookDedupeResponse {
                success: false,
                task_id: None,
                agent_id: None,
                message: "Invalid or missing API key".to_string(),
            }
        );
    }

    log::info!(
        "[WEBHOOK] Received dedupe request for issue #{} in {}",
        request.issue_number,
        request.repo
    );

    let warp_client = match &state.warp_client {
        Some(client) => client,
        None => {
            log::warn!("[WEBHOOK] No Warp API client available");
            return json_err!(
                SERVICE_UNAVAILABLE,
                WebhookDedupeResponse {
                    success: false,
                    task_id: None,
                    agent_id: None,
                    message: "No Warp API key configured".to_string(),
                }
            );
        }
    };

    let callback_token = generate_callback_token();
    let callback_url =
        env::var("POWERFIXER_CALLBACK_URL").unwrap_or_else(|_| config::default_callback_url());

    let environment_id = match config::dedupe_environment_id() {
        Some(v) => v,
        None => {
            return json_err!(
                SERVICE_UNAVAILABLE,
                WebhookDedupeResponse {
                    success: false,
                    task_id: None,
                    agent_id: None,
                    message:
                        "POWERFIXER_DEDUPE_ENVIRONMENT_ID (or POWERFIXER_ENVIRONMENT_ID) is not configured"
                            .to_string(),
                }
            );
        }
    };

    let external_id = request.issue_number.to_string();
    let base_prompt = get_dedupe_base_prompt(&request.repo, &external_id, "");
    let prompt = get_remote_agent_prompt(&base_prompt, &callback_token, &callback_url);

    let warp_request = WarpLaunchRequest {
        prompt: prompt.clone(),
        config: Some(TaskConfig { environment_id }),
        agent_profile_id: config::agent_profile_id(),
        secrets: config::dedupe_secret_name().map(|name| vec![name]),
        team: Some(config::team_scoped_launch()),
    };

    log::debug!("[WEBHOOK] Launching dedupe agent via Warp API (codeless environment)...");

    let task_id = match warp_client.launch_agent(warp_request).await {
        Ok(response) => {
            log::info!("[WEBHOOK] Warp API returned task_id: {}", response.task_id);
            response.task_id
        }
        Err(WarpApiError::ApiError { status, body }) => {
            log::error!("[WEBHOOK] Warp API error: {} - {}", status, body);
            return json_err!(
                BAD_GATEWAY,
                WebhookDedupeResponse {
                    success: false,
                    task_id: None,
                    agent_id: None,
                    message: format!("Warp API error: {} - {}", status, body),
                }
            );
        }
        Err(e) => {
            log::error!("[WEBHOOK] Failed to call Warp API: {}", e);
            return json_err!(
                BAD_GATEWAY,
                WebhookDedupeResponse {
                    success: false,
                    task_id: None,
                    agent_id: None,
                    message: format!("Failed to call Warp API: {}", e),
                }
            );
        }
    };

    let issue_title = match fetch_issue_metadata_async(&request.repo, &external_id).await {
        Ok((title, _body)) => {
            log::info!(
                "[WEBHOOK] Fetched metadata for issue #{}: {}",
                external_id,
                title
            );
            Some(title)
        }
        Err(e) => {
            log::warn!(
                "[WEBHOOK] Failed to fetch issue metadata for #{}: {}",
                external_id,
                e
            );
            None
        }
    };

    let project = request
        .repo
        .split('/')
        .next_back()
        .unwrap_or(DEFAULT_PROJECT);
    let issue_url = format!(
        "https://github.com/{}/issues/{}",
        request.repo, request.issue_number
    );
    let new_issue = NewIssue {
        provider_config_id: DEFAULT_PROVIDER_CONFIG_ID,
        external_id: external_id.clone(),
        external_url: Some(issue_url.clone()),
        project: project.to_string(),
        title: issue_title.clone(),
        labels: vec![],
    };

    let agent_id = match queries::upsert_issue(&state.pool, &new_issue).await {
        Ok(db_issue) => {
            let new_agent = NewAgent {
                agent_type: AgentType::Dedupe,
                execution_mode: ExecutionMode::Remote,
                trigger_issue_id: Some(db_issue.id),
                task_id: Some(task_id.clone()),
                callback_token,
                prompt,
                task_state: AgentTaskState::Queued,
                pid: None,
                log_path: None,
                trigger_source: TriggerSource::GithubWebhook,
                triggered_by: None,
                started_at: chrono::Utc::now(),
            };
            match queries::create_agent(&state.pool, &new_agent).await {
                Ok(created_agent) => {
                    log::info!(
                        "[WEBHOOK] Dedupe agent created: id={} for issue #{}",
                        created_agent.id,
                        request.issue_number
                    );
                    broadcast_agent_update(&state.pool, &state.ws_broadcast, created_agent.id)
                        .await;

                    Some(created_agent.id)
                }
                Err(e) => {
                    log::error!("[WEBHOOK] Failed to create agent: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            log::error!("[WEBHOOK] Failed to upsert issue: {}", e);
            None
        }
    };

    log::info!(
        "[WEBHOOK] Dedupe launch complete: task_id={} for issue #{}",
        task_id,
        request.issue_number
    );

    json_ok!(WebhookDedupeResponse {
        success: true,
        task_id: Some(task_id),
        agent_id,
        message: "Dedupe agent launched successfully".to_string(),
    })
}
