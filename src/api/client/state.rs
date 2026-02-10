//! TUI state synchronization endpoints.
//!
//! This module provides endpoints for TUI clients to fetch complete state
//! snapshots and update inbox read/archived states.

use axum::{
    extract::{Json, Path, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::super::types::{
    github_issue_url, AgentInfo, ApiState, DedupeClosureInfo, DedupeResultInfo,
    DuplicateCandidateInfo, InboxStateInfo, TriageResultInfo, TriageRunInfo, DEFAULT_GITHUB_ORG,
    DEFAULT_PROJECT, DEFAULT_PROVIDER_CONFIG_ID,
};
use super::super::websocket::broadcast_inbox_state_update;
use crate::db::models::{AgentType, DbAgent, NewIssue};
use crate::db::queries;

#[derive(Debug, Serialize)]
pub struct FullStateResponse {
    pub agents: Vec<AgentInfo>,
    pub triage_runs: Vec<TriageRunInfo>,
    pub inbox_states: Vec<InboxStateInfo>,
}

/// Returns the complete state for TUI initial synchronization.
pub async fn get_full_state(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    log::debug!("Getting full state for TUI");

    let db_agents = queries::get_all_agents(&state.pool)
        .await
        .unwrap_or_default();
    let mut agents = Vec::new();

    for agent in db_agents {
        let agent_info = build_agent_info(&state.pool, agent).await;
        agents.push(agent_info);
    }

    let db_triage_runs = queries::get_all_triage_runs(&state.pool)
        .await
        .unwrap_or_default();
    let mut triage_runs = Vec::new();
    for run in db_triage_runs {
        let run_agents = queries::get_triage_run_agents(&state.pool, run.id)
            .await
            .unwrap_or_default();
        triage_runs.push(TriageRunInfo {
            id: run.id,
            started_at: run.started_at.to_rfc3339(),
            min_external_id: run.min_external_id.clone(),
            max_external_id: run.max_external_id.clone(),
            agent_ids: run_agents.iter().map(|a| a.agent_id).collect(),
        });
    }

    let inbox_states: Vec<InboxStateInfo> = queries::get_all_agent_inbox_states(&state.pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|s| InboxStateInfo {
            agent_id: s.agent_id,
            is_archived: s.is_archived,
        })
        .collect();

    log::debug!(
        "Returning {} agents, {} triage runs, {} inbox states",
        agents.len(),
        triage_runs.len(),
        inbox_states.len()
    );

    json_ok!(FullStateResponse {
        agents,
        triage_runs,
        inbox_states,
    })
}

/// Builds a fully populated AgentInfo from a DbAgent, including type-specific data.
pub async fn build_agent_info(pool: &sqlx::PgPool, agent: DbAgent) -> AgentInfo {
    let (issue_number, issue_title, issue_url) = if let Some(issue_id) = agent.trigger_issue_id {
        queries::get_issue_by_id(pool, issue_id)
            .await
            .ok()
            .flatten()
            .map(|i| (Some(i.external_id), i.title, i.external_url))
            .unwrap_or((None, None, None))
    } else {
        (None, None, None)
    };

    let (external_ids, triage_results) = if agent.agent_type == AgentType::Triage {
        let run_agent = queries::get_triage_run_agent_by_agent_id(pool, agent.id)
            .await
            .ok()
            .flatten();
        let ext_ids = run_agent.map(|ra| ra.external_ids);

        let db_results = queries::get_triage_results_for_agent(pool, agent.id)
            .await
            .unwrap_or_default();
        let results: Vec<TriageResultInfo> = db_results
            .into_iter()
            .map(|r| TriageResultInfo {
                external_id: r.external_id,
                result: r.result,
                reason: r.reason,
            })
            .collect();
        let results_opt = if results.is_empty() {
            None
        } else {
            Some(results)
        };
        (ext_ids, results_opt)
    } else {
        (None, None)
    };

    let dedupe_result = if agent.agent_type == AgentType::Dedupe {
        if let Some(dedupe_run) = queries::get_dedupe_run_by_agent_id(pool, agent.id)
            .await
            .ok()
            .flatten()
        {
            let duplicates = queries::get_dedupe_duplicates(pool, dedupe_run.id)
                .await
                .unwrap_or_default()
                .into_iter()
                .filter_map(|d| {
                    let issue_number = DuplicateCandidateInfo::extract_issue_number(&d.issue_url)?;
                    Some(DuplicateCandidateInfo {
                        issue_url: d.issue_url,
                        issue_number,
                        confidence: d.confidence,
                        reason: d.reason,
                    })
                })
                .collect();
            let closures = queries::get_dedupe_closures(pool, dedupe_run.id)
                .await
                .unwrap_or_default()
                .into_iter()
                .filter_map(|c| {
                    let issue_number =
                        DedupeClosureInfo::extract_issue_number(&c.closed_issue_url)?;
                    Some(DedupeClosureInfo {
                        closed_issue_url: c.closed_issue_url,
                        closed_issue_number: issue_number,
                        closed_at: c.closed_at.to_rfc3339(),
                    })
                })
                .collect();
            Some(DedupeResultInfo {
                canonical_issue_url: dedupe_run.canonical_issue_url,
                analysis_summary: dedupe_run.analysis_summary,
                duplicates,
                is_addressed: dedupe_run.is_addressed,
                addressed_at: dedupe_run.addressed_at.map(|t| t.to_rfc3339()),
                closures,
            })
        } else {
            None
        }
    } else {
        None
    };

    AgentInfo {
        id: agent.id,
        agent_type: agent.agent_type.as_str().to_string(),
        execution_mode: agent.execution_mode.as_str().to_string(),
        trigger_issue_id: agent.trigger_issue_id,
        trigger_issue_number: issue_number,
        trigger_issue_title: issue_title,
        trigger_issue_url: issue_url,
        task_id: agent.task_id,
        task_state: agent.task_state.display_name().to_string(),
        session_url: agent.session_url,
        branch_name: agent.branch_name,
        pr_url: agent.pr_url,
        summary: agent.summary,
        started_at: agent.started_at.to_rfc3339(),
        trigger_source: Some(agent.trigger_source.as_str().to_string()),
        pid: agent.pid,
        log_path: agent.log_path,
        external_ids,
        triage_results,
        dedupe_result,
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateAgentInboxStateRequest {
    pub agent_id: i32,
    pub is_archived: bool,
}

/// Updates inbox archived state for an agent.
pub async fn update_agent_inbox_state(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<UpdateAgentInboxStateRequest>,
) -> impl IntoResponse {
    match queries::upsert_agent_inbox_state(&state.pool, request.agent_id, request.is_archived)
        .await
    {
        Ok(_) => {
            broadcast_inbox_state_update(
                &state.ws_broadcast,
                request.agent_id,
                request.is_archived,
            );
            ok!("Agent inbox state updated")
        }
        Err(e) => err!(
            INTERNAL_SERVER_ERROR,
            "Failed to update agent inbox state: {}",
            e
        ),
    }
}

/// Deletes an agent by ID.
pub async fn delete_agent_by_id(
    State(state): State<Arc<ApiState>>,
    Path(agent_id): Path<i32>,
) -> impl IntoResponse {
    log::debug!("Deleting agent id={}", agent_id);

    match queries::delete_agent(&state.pool, agent_id).await {
        Ok(_) => {
            super::super::websocket::broadcast_agent_deleted(&state.ws_broadcast, agent_id);
            ok!("Agent deleted")
        }
        Err(e) => err!(INTERNAL_SERVER_ERROR, "Failed to delete agent: {}", e),
    }
}

#[derive(Debug, Deserialize)]
pub struct CacheIssueTitleRequest {
    pub issue_number: i32,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct CacheIssueTitlesRequest {
    pub issues: Vec<CacheIssueTitleRequest>,
}

/// Caches issue titles for TUI display.
pub async fn cache_issue_titles(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<CacheIssueTitlesRequest>,
) -> impl IntoResponse {
    let mut cached_count = 0;

    for issue in request.issues {
        let new_issue = NewIssue {
            provider_config_id: DEFAULT_PROVIDER_CONFIG_ID,
            external_id: issue.issue_number.to_string(),
            external_url: Some(github_issue_url(
                DEFAULT_GITHUB_ORG,
                DEFAULT_PROJECT,
                issue.issue_number,
            )),
            project: DEFAULT_PROJECT.to_string(),
            title: Some(issue.title),
            labels: vec![],
        };

        if queries::upsert_issue(&state.pool, &new_issue).await.is_ok() {
            cached_count += 1;
        }
    }

    ok!("Cached {} issue titles", cached_count)
}
