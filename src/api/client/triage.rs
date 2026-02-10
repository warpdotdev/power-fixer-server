//! Triage agent endpoints.
//!
//! This module handles creating, deleting, and managing triage agents,
//! as well as triage runs, results, and coverage reporting.

use axum::{
    extract::{Json, Path, Query, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;

use super::super::types::ApiState;
use super::super::warp_api::{LaunchAgentRequest as WarpLaunchRequest, TaskConfig};
use super::super::websocket::{
    broadcast_agent_deleted, broadcast_agent_update, broadcast_triage_run_update,
};
use crate::config;
use crate::db::models::{
    generate_callback_token, AgentTaskState, AgentType, ExecutionMode, NewAgent, NewTriageResult,
    NewTriageRun, NewTriageRunAgent, TriageResultType, TriggerSource,
};
use crate::db::queries;
use crate::prompts::get_triage_prompt;

#[derive(Debug, Deserialize)]
pub struct CreateTriageRunRequest {
    pub repo: String,
    pub agents: Vec<TriageAgentSpec>,
    #[serde(default)]
    pub triggered_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TriageAgentSpec {
    pub external_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateTriageRunResponse {
    pub success: bool,
    pub run_id: Option<i32>,
    pub agent_ids: Vec<i32>,
    pub task_ids: Vec<String>,
    pub message: String,
}

/// Creates a new triage run with associated agents.
///
/// This endpoint handles everything:
/// 1. Generates prompts for each agent batch
/// 2. Calls Warp API to launch each agent
/// 3. Creates triage_run, agents, and triage_run_agents records
/// 4. Broadcasts all updates via WebSocket
pub async fn create_triage_run(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<CreateTriageRunRequest>,
) -> impl IntoResponse {
    let warp_client = match &state.warp_client {
        Some(client) => client,
        None => {
            return json_err!(
                SERVICE_UNAVAILABLE,
                CreateTriageRunResponse {
                    success: false,
                    run_id: None,
                    agent_ids: vec![],
                    task_ids: vec![],
                    message: "No Warp API key configured".to_string(),
                }
            );
        }
    };

    let all_external_ids: Vec<String> = request
        .agents
        .iter()
        .flat_map(|a| a.external_ids.iter().cloned())
        .collect();
    let min_external_id = all_external_ids.iter().min().cloned().unwrap_or_default();
    let max_external_id = all_external_ids.iter().max().cloned().unwrap_or_default();

    let new_run = NewTriageRun {
        started_at: chrono::Utc::now(),
        min_external_id,
        max_external_id,
    };

    let run = match queries::create_triage_run(&state.pool, &new_run).await {
        Ok(r) => r,
        Err(e) => {
            return json_err!(
                INTERNAL_SERVER_ERROR,
                CreateTriageRunResponse {
                    success: false,
                    run_id: None,
                    agent_ids: vec![],
                    task_ids: vec![],
                    message: format!("Failed to create triage run: {}", e),
                }
            );
        }
    };

    log::info!(
        "Created triage run {} for issues {}-{}",
        run.id,
        all_external_ids.first().unwrap_or(&String::new()),
        all_external_ids.last().unwrap_or(&String::new())
    );

    let callback_url =
        env::var("POWERFIXER_CALLBACK_URL").unwrap_or_else(|_| config::default_callback_url());

    let environment_id = match config::environment_id() {
        Some(v) => v,
        None => {
            return json_err!(
                SERVICE_UNAVAILABLE,
                CreateTriageRunResponse {
                    success: false,
                    run_id: None,
                    agent_ids: vec![],
                    task_ids: vec![],
                    message: "POWERFIXER_ENVIRONMENT_ID is not configured".to_string(),
                }
            );
        }
    };
    let triage_secret = config::triage_secret_name();

    let mut agent_ids = Vec::new();
    let mut task_ids = Vec::new();

    for agent_spec in &request.agents {
        let callback_token = generate_callback_token();
        let issue_ids_str = agent_spec
            .external_ids
            .iter()
            .map(|id| format!("#{}", id))
            .collect::<Vec<_>>()
            .join(", ");
        let prompt = get_triage_prompt(
            &request.repo,
            &issue_ids_str,
            &callback_token,
            &callback_url,
        );

        let warp_request = WarpLaunchRequest {
            prompt: prompt.clone(),
            config: Some(TaskConfig {
                environment_id: environment_id.clone(),
            }),
            agent_profile_id: config::agent_profile_id(),
            secrets: triage_secret.clone().map(|name| vec![name]),
            team: Some(config::team_scoped_launch()),
        };

        let task_id = match warp_client.launch_agent(warp_request).await {
            Ok(response) => {
                log::info!("Launched triage agent with task_id: {}", response.task_id);
                response.task_id
            }
            Err(e) => {
                log::error!("Failed to launch triage agent via Warp API: {}", e);
                continue;
            }
        };

        let new_agent = NewAgent {
            agent_type: AgentType::Triage,
            execution_mode: ExecutionMode::Remote,
            trigger_issue_id: None,
            task_id: Some(task_id.clone()),
            callback_token,
            prompt,
            task_state: AgentTaskState::Queued,
            pid: None,
            log_path: None,
            trigger_source: TriggerSource::Tui,
            triggered_by: request.triggered_by.clone(),
            started_at: chrono::Utc::now(),
        };

        match queries::create_agent(&state.pool, &new_agent).await {
            Ok(agent) => {
                let run_agent = NewTriageRunAgent {
                    triage_run_id: run.id,
                    agent_id: agent.id,
                    external_ids: agent_spec.external_ids.clone(),
                };
                if let Err(e) = queries::create_triage_run_agent(&state.pool, &run_agent).await {
                    log::error!("Failed to create triage run agent: {}", e);
                }
                agent_ids.push(agent.id);
                task_ids.push(task_id);
                broadcast_agent_update(&state.pool, &state.ws_broadcast, agent.id).await;
            }
            Err(e) => {
                log::error!("Failed to create triage agent: {}", e);
            }
        }
    }

    broadcast_triage_run_update(&state.pool, &state.ws_broadcast, run.id).await;

    log::info!(
        "Triage run {} complete: {} agents launched",
        run.id,
        agent_ids.len()
    );

    json_ok!(CreateTriageRunResponse {
        success: true,
        run_id: Some(run.id),
        agent_ids,
        task_ids,
        message: "Triage run created".to_string(),
    })
}

/// Deletes a triage run and its associated agents.
pub async fn delete_triage_run(
    State(state): State<Arc<ApiState>>,
    Path(run_id): Path<i32>,
) -> impl IntoResponse {
    if let Ok(run_agents) = queries::get_triage_run_agents(&state.pool, run_id).await {
        for ra in run_agents {
            if queries::delete_agent(&state.pool, ra.agent_id)
                .await
                .is_ok()
            {
                broadcast_agent_deleted(&state.ws_broadcast, ra.agent_id);
            }
        }
    }
    let _ = queries::delete_triage_results_for_run(&state.pool, run_id).await;

    match queries::delete_triage_run(&state.pool, run_id).await {
        Ok(_) => ok!("Triage run deleted"),
        Err(e) => err!(INTERNAL_SERVER_ERROR, "Failed to delete triage run: {}", e),
    }
}

#[derive(Debug, Deserialize)]
pub struct ExcludedIssuesQuery {
    #[serde(default = "default_days")]
    pub days: i32,
}

fn default_days() -> i32 {
    7
}

#[derive(Debug, Serialize)]
pub struct ExcludedIssuesResponse {
    pub external_ids: Vec<String>,
}

/// Returns external IDs that should be excluded from triage (recently triaged).
pub async fn get_excluded_issues(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<ExcludedIssuesQuery>,
) -> impl IntoResponse {
    match queries::get_recently_triaged_external_ids(&state.pool, query.days).await {
        Ok(ids) => json_ok!(ExcludedIssuesResponse { external_ids: ids }),
        Err(e) => {
            log::error!("Failed to get excluded issues: {}", e);
            json_err!(
                INTERNAL_SERVER_ERROR,
                ExcludedIssuesResponse {
                    external_ids: vec![]
                }
            )
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateTriageResultRequest {
    pub triage_run_id: i32,
    pub agent_id: i32,
    pub external_id: String,
    pub result: String,
    pub reason: String,
}

/// Records a triage result.
pub async fn create_triage_result_endpoint(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<CreateTriageResultRequest>,
) -> impl IntoResponse {
    let result_type = match request.result.as_str() {
        "candidate" => TriageResultType::Candidate,
        _ => TriageResultType::Rejected,
    };

    let entry = NewTriageResult {
        triage_run_id: request.triage_run_id,
        agent_id: request.agent_id,
        external_id: request.external_id,
        result: result_type,
        reason: request.reason,
    };

    match queries::create_triage_result(&state.pool, &entry).await {
        Ok(_) => ok!("Triage result recorded"),
        Err(e) => err!(
            INTERNAL_SERVER_ERROR,
            "Failed to record triage result: {}",
            e
        ),
    }
}

#[derive(Debug, Serialize)]
pub struct TriageResultResponse {
    pub external_id: String,
    pub result: String,
    pub reason: String,
    pub evaluated_at: String,
}

#[derive(Debug, Serialize)]
pub struct TriageResultsResponse {
    pub results: Vec<TriageResultResponse>,
}

/// Returns triage results (candidates and rejected issues).
pub async fn get_triage_results(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    let candidates = queries::get_triage_candidates(&state.pool)
        .await
        .unwrap_or_default();

    let results: Vec<TriageResultResponse> = candidates
        .into_iter()
        .map(|r| TriageResultResponse {
            external_id: r.external_id,
            result: r.result.as_str().to_string(),
            reason: r.reason,
            evaluated_at: r.evaluated_at.to_rfc3339(),
        })
        .collect();

    json_ok!(TriageResultsResponse { results })
}

#[derive(Debug, Serialize)]
pub struct TriageRunSummaryResponse {
    pub run_id: i32,
    pub started_at: String,
    pub min_external_id: String,
    pub max_external_id: String,
    pub agent_count: i64,
    pub candidates_count: i64,
    pub rejected_count: i64,
    pub is_complete: bool,
}

#[derive(Debug, Serialize)]
pub struct TriageSummaryResponse {
    pub runs: Vec<TriageRunSummaryResponse>,
}

/// Returns a summary of triage runs.
pub async fn get_triage_summary(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    match queries::get_triage_run_summaries(&state.pool, 20).await {
        Ok(summaries) => {
            let runs: Vec<TriageRunSummaryResponse> = summaries
                .into_iter()
                .map(|s| TriageRunSummaryResponse {
                    run_id: s.run_id,
                    started_at: s.started_at.to_rfc3339(),
                    min_external_id: s.min_external_id,
                    max_external_id: s.max_external_id,
                    agent_count: s.agent_count,
                    candidates_count: s.candidates_count,
                    rejected_count: s.rejected_count,
                    is_complete: s.is_complete,
                })
                .collect();
            json_ok!(TriageSummaryResponse { runs })
        }
        Err(e) => {
            log::error!("Failed to get triage summary: {}", e);
            json_ok!(TriageSummaryResponse { runs: vec![] })
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TriageCoverageResponse {
    pub examined_count: i64,
    pub candidates_count: i64,
    pub rejected_count: i64,
    pub agent_assigned_count: i64,
    pub min_triaged: Option<String>,
    pub max_triaged: Option<String>,
}

/// Returns triage coverage statistics.
pub async fn get_triage_coverage_endpoint(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    let coverage = match queries::get_triage_coverage(&state.pool).await {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to get coverage: {}", e);
            return json_ok!(TriageCoverageResponse {
                examined_count: 0,
                candidates_count: 0,
                rejected_count: 0,
                agent_assigned_count: 0,
                min_triaged: None,
                max_triaged: None,
            });
        }
    };

    json_ok!(TriageCoverageResponse {
        examined_count: coverage.examined_count,
        candidates_count: coverage.candidates_count,
        rejected_count: coverage.rejected_count,
        agent_assigned_count: coverage.agent_assigned_count,
        min_triaged: coverage.min_triaged,
        max_triaged: coverage.max_triaged,
    })
}
