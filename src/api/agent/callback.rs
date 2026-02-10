//! Agent callback endpoint for status updates.
//!
//! This module handles the `/api/v1/agent/status` endpoint where running agents
//! POST their progress. Agents are identified by their callback token (Bearer auth).

use axum::{
    extract::{Json, State},
    http::HeaderMap,
    response::IntoResponse,
};
use std::sync::Arc;

use super::super::slack::{
    broadcast_agent_state_change, broadcast_dedupe_completed, AgentDedupeCompletedInfo,
    AgentStateChangeInfo, DedupeDuplicateInfo,
};
use super::super::types::{fetch_issue_metadata_async, DEFAULT_GITHUB_ORG, DEFAULT_PROJECT};
use super::super::types::{ApiState, CallbackAgentInfo as AgentInfo, HealthResponse, StatusUpdate};
use super::super::websocket::broadcast_agent_update;
use crate::db::{
    models::{
        AgentTaskState, AgentType, NewAgentStatusUpdate, NewDedupeDuplicate, NewDedupeRun,
        NewTriageResult, TriageResultType,
    },
    queries,
};

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

fn parse_task_state(state: &str) -> Option<AgentTaskState> {
    match state.to_uppercase().as_str() {
        "QUEUED" => Some(AgentTaskState::Queued),
        "IN_PROGRESS" | "INPROGRESS" => Some(AgentTaskState::InProgress),
        "SUCCEEDED" | "SUCCESS" | "COMPLETED" => Some(AgentTaskState::Succeeded),
        "FAILED" | "FAILURE" => Some(AgentTaskState::Failed),
        _ => None,
    }
}

/// Handles agent status update callbacks.
pub async fn update_agent_status(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(update): Json<StatusUpdate>,
) -> impl IntoResponse {
    log::debug!(
        "Received agent callback: state={}, task_id={:?}, branch={:?}, pr_url={:?}, session={:?}, summary={:?}, candidates={:?}, rejected={:?}, dedupe={}",
        update.state,
        update.task_id,
        update.branch_name,
        update.pr_url,
        update.session_url,
        update.summary,
        update.candidates,
        update.rejected.as_ref().map(|r| r.len()),
        update.duplicates.is_some()
    );

    let token = match extract_bearer_token(&headers) {
        Some(t) => {
            log::debug!("Token received: {}...", &t[..t.len().min(16)]);
            t
        }
        None => {
            log::debug!("Missing Authorization header");
            return err!(UNAUTHORIZED, "Missing or invalid Authorization header");
        }
    };

    log::debug!("Looking up agent by callback token...");
    let agent = match queries::get_agent_by_callback_token(&state.pool, &token).await {
        Ok(Some(a)) => {
            log::debug!("Found agent id={} type={:?}", a.id, a.agent_type);
            let triage_run_id = if a.agent_type == AgentType::Triage {
                queries::get_triage_run_agent_by_agent_id(&state.pool, a.id)
                    .await
                    .ok()
                    .flatten()
                    .map(|tra| tra.triage_run_id)
            } else {
                None
            };
            AgentInfo {
                id: a.id,
                agent_type: a.agent_type,
                execution_mode: a.execution_mode,
                trigger_issue_id: a.trigger_issue_id,
                task_id: a.task_id,
                triage_run_id,
            }
        }
        Ok(None) => {
            log::debug!("No agent found for callback token");
            return err!(UNAUTHORIZED, "Invalid callback token");
        }
        Err(e) => {
            log::error!("Database error looking up callback token: {}", e);
            return err!(INTERNAL_SERVER_ERROR, "Database error");
        }
    };

    let task_state = match parse_task_state(&update.state) {
        Some(s) => s,
        None => {
            return err!(
                BAD_REQUEST,
                "Invalid state: {}. Expected one of: QUEUED, IN_PROGRESS, SUCCEEDED, FAILED",
                update.state
            );
        }
    };

    let session_url = update.session_url.as_deref().and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    if let Err(e) = queries::update_agent_full(
        &state.pool,
        agent.id,
        task_state,
        session_url.as_deref(),
        update.branch_name.as_deref(),
        update.pr_url.as_deref(),
        update.summary.as_deref(),
    )
    .await
    {
        log::error!("Failed to update agent: {}", e);
        return err!(INTERNAL_SERVER_ERROR, "Failed to update status");
    }

    let status_update = NewAgentStatusUpdate {
        agent_id: agent.id,
        state: update.state.clone(),
        message: update.summary.clone(),
    };
    if let Err(e) = queries::create_agent_status_update(&state.pool, &status_update).await {
        log::warn!("Failed to log agent status update: {}", e);
    }

    match agent.agent_type {
        AgentType::Dedupe => {
            if let (Some(canonical_url), Some(duplicates)) =
                (&update.canonical_issue_url, &update.duplicates)
            {
                let dedupe_run = NewDedupeRun {
                    agent_id: agent.id,
                    canonical_issue_url: canonical_url.clone(),
                    analysis_summary: update.summary.clone(),
                };
                match queries::create_dedupe_run(&state.pool, &dedupe_run).await {
                    Ok(run) => {
                        for dup in duplicates {
                            let duplicate = NewDedupeDuplicate {
                                dedupe_run_id: run.id,
                                issue_url: dup.issue_url.clone(),
                                confidence: dup.confidence,
                                reason: dup.reason.clone(),
                            };
                            if let Err(e) =
                                queries::create_dedupe_duplicate(&state.pool, &duplicate).await
                            {
                                log::error!("Failed to create dedupe duplicate: {}", e);
                            }
                        }
                    }
                    Err(e) => log::error!("Failed to create dedupe run: {}", e),
                }
            }
        }
        AgentType::Triage => {
            if task_state == AgentTaskState::Succeeded || task_state == AgentTaskState::Failed {
                if let Some(triage_run_id) = agent.triage_run_id {
                    if let Some(ref cands) = update.candidates {
                        for external_id in cands {
                            let result = NewTriageResult {
                                triage_run_id,
                                agent_id: agent.id,
                                external_id: external_id.clone(),
                                result: TriageResultType::Candidate,
                                reason: "Agent candidate".to_string(),
                            };
                            if let Err(e) =
                                queries::create_triage_result(&state.pool, &result).await
                            {
                                log::error!(
                                    "Failed to record triage result for candidate {}: {}",
                                    external_id,
                                    e
                                );
                            }
                        }
                    }
                    if let Some(ref rejected) = update.rejected {
                        for rej in rejected {
                            let result = NewTriageResult {
                                triage_run_id,
                                agent_id: agent.id,
                                external_id: rej.external_id.clone(),
                                result: TriageResultType::Rejected,
                                reason: rej.reason.clone(),
                            };
                            if let Err(e) =
                                queries::create_triage_result(&state.pool, &result).await
                            {
                                log::error!(
                                    "Failed to record triage result for rejected {}: {}",
                                    rej.external_id,
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }
        AgentType::Fix => {}
    }

    log::info!(
        "Agent status updated: id={}, type={:?}, mode={:?}, state={:?}",
        agent.id,
        agent.agent_type,
        agent.execution_mode,
        task_state
    );

    broadcast_agent_update(&state.pool, &state.ws_broadcast, agent.id).await;

    let (issue_url, issue_number, issue_title) = if let Some(issue_id) = agent.trigger_issue_id {
        queries::get_issue_by_id(&state.pool, issue_id)
            .await
            .ok()
            .flatten()
            .map(|i| (i.external_url, Some(i.external_id), i.title))
            .unwrap_or((None, None, None))
    } else {
        (None, None, None)
    };

    let trigger_issue_body = match issue_number.as_ref() {
        Some(num) => {
            fetch_issue_metadata_async(&format!("{}/{}", DEFAULT_GITHUB_ORG, DEFAULT_PROJECT), num)
                .await
                .ok()
                .map(|(_, body)| body)
        }
        None => None,
    };

    let (final_session_url, final_branch_name, final_pr_url) =
        if let Ok(Some(updated_agent)) = queries::get_agent_by_id(&state.pool, agent.id).await {
            (
                updated_agent.session_url,
                updated_agent.branch_name,
                updated_agent.pr_url,
            )
        } else {
            (
                session_url,
                update.branch_name.clone(),
                update.pr_url.clone(),
            )
        };

    if task_state == AgentTaskState::Succeeded || task_state == AgentTaskState::Failed {
        if agent.agent_type == AgentType::Dedupe {
            if let (Some(canonical_url), Some(duplicates)) =
                (&update.canonical_issue_url, &update.duplicates)
            {
                let canonical_issue_number =
                    extract_issue_number(canonical_url).map(|n| n.to_string());

                let (canonical_issue_title, canonical_issue_body) =
                    match canonical_issue_number.as_ref() {
                        Some(n) => fetch_issue_metadata_async(
                            &format!("{}/{}", DEFAULT_GITHUB_ORG, DEFAULT_PROJECT),
                            n,
                        )
                        .await
                        .ok()
                        .map(|(t, b)| (Some(t), Some(b)))
                        .unwrap_or((None, None)),
                        None => (None, None),
                    };

                let mut duplicate_infos: Vec<DedupeDuplicateInfo> =
                    Vec::with_capacity(duplicates.len());
                for d in duplicates {
                    let num_str = extract_issue_number(&d.issue_url).map(|n| n.to_string());
                    if num_str == issue_number {
                        continue;
                    }
                    let (title, body) = match num_str.as_ref() {
                        Some(n) => fetch_issue_metadata_async(
                            &format!("{}/{}", DEFAULT_GITHUB_ORG, DEFAULT_PROJECT),
                            n,
                        )
                        .await
                        .ok()
                        .map(|(t, b)| (Some(t), Some(b)))
                        .unwrap_or((None, None)),
                        None => (None, None),
                    };
                    duplicate_infos.push(DedupeDuplicateInfo {
                        issue_url: d.issue_url.clone(),
                        issue_number: num_str,
                        issue_title: title,
                        issue_body: body,
                        confidence: d.confidence,
                        reason: d.reason.clone(),
                    });
                }

                let dedupe_info = AgentDedupeCompletedInfo {
                    task_state,
                    session_url: final_session_url.clone(),
                    trigger_issue_url: issue_url.clone(),
                    trigger_issue_number: issue_number.clone(),
                    trigger_issue_title: issue_title.clone(),
                    trigger_issue_body: trigger_issue_body.clone(),
                    canonical_issue_url: Some(canonical_url.clone()),
                    canonical_issue_number,
                    canonical_issue_title,
                    canonical_issue_body,
                    duplicates: duplicate_infos,
                };
                broadcast_dedupe_completed(
                    state.slack_client.as_ref(),
                    state.openai_client.as_ref(),
                    &dedupe_info,
                )
                .await;
            } else {
                let slack_info = AgentStateChangeInfo {
                    agent_type: agent.agent_type,
                    task_state,
                    issue_url,
                    issue_number,
                    session_url: final_session_url,
                    branch_name: final_branch_name,
                    pr_url: final_pr_url,
                };
                broadcast_agent_state_change(state.slack_client.as_ref(), &slack_info).await;
            }
        } else {
            let slack_info = AgentStateChangeInfo {
                agent_type: agent.agent_type,
                task_state,
                issue_url,
                issue_number,
                session_url: final_session_url,
                branch_name: final_branch_name,
                pr_url: final_pr_url,
            };
            broadcast_agent_state_change(state.slack_client.as_ref(), &slack_info).await;
        }
    }

    ok!("Status updated")
}

fn extract_issue_number(url: &str) -> Option<u32> {
    url.rsplit('/').next().and_then(|s| s.parse().ok())
}

/// Simple health check endpoint.
pub async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}
