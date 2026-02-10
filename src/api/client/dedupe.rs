//! Deduplication endpoints.
//!
//! This module handles retrieving deduplication results and recording
//! closure actions when duplicate issues are closed.

use axum::{
    extract::{Json, Path, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::super::types::{
    close_issue_async, github_issue_url, ApiState, DuplicateCandidateInfo, DEFAULT_GITHUB_ORG,
    DEFAULT_PROJECT,
};
use crate::db::models::NewDedupeClosure;
use crate::db::queries;

#[derive(Debug, Serialize)]
pub struct DedupeResultResponse {
    pub found: bool,
    pub canonical_issue_url: Option<String>,
    pub canonical_issue_number: Option<u32>,
    pub duplicates: Vec<DuplicateCandidateInfo>,
    pub analysis_summary: Option<String>,
}

/// Retrieves deduplication results for an agent.
pub async fn get_dedupe_result(
    State(state): State<Arc<ApiState>>,
    Path(agent_id): Path<i32>,
) -> impl IntoResponse {
    match queries::get_dedupe_run_by_agent_id(&state.pool, agent_id).await {
        Ok(Some(run)) => {
            let canonical_issue_number =
                DuplicateCandidateInfo::extract_issue_number(&run.canonical_issue_url);
            let duplicates = queries::get_dedupe_duplicates(&state.pool, run.id)
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
            json_ok!(DedupeResultResponse {
                found: true,
                canonical_issue_url: Some(run.canonical_issue_url),
                canonical_issue_number,
                duplicates,
                analysis_summary: run.analysis_summary,
            })
        }
        Ok(None) => json_ok!(DedupeResultResponse {
            found: false,
            canonical_issue_url: None,
            canonical_issue_number: None,
            duplicates: vec![],
            analysis_summary: None,
        }),
        Err(e) => json_err!(
            INTERNAL_SERVER_ERROR,
            DedupeResultResponse {
                found: false,
                canonical_issue_url: None,
                canonical_issue_number: None,
                duplicates: vec![],
                analysis_summary: Some(format!("Database error: {}", e)),
            }
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateDedupeClosureRequest {
    pub dedupe_run_id: i32,
    pub closed_issue_url: String,
}

/// Records a closure action when a duplicate issue is closed.
pub async fn create_dedupe_closure(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<CreateDedupeClosureRequest>,
) -> impl IntoResponse {
    let closure = NewDedupeClosure {
        dedupe_run_id: request.dedupe_run_id,
        closed_issue_url: request.closed_issue_url,
        closed_by: "PowerFixer".to_string(),
    };
    match queries::create_dedupe_closure(&state.pool, &closure).await {
        Ok(_) => ok!("Dedupe closure recorded"),
        Err(e) => err!(INTERNAL_SERVER_ERROR, "Failed to record closure: {}", e),
    }
}

#[derive(Debug, Serialize)]
pub struct MarkAddressedResponse {
    pub success: bool,
    pub message: String,
}

/// Marks a dedupe run as addressed (reviewed and acted upon).
pub async fn mark_dedupe_addressed(
    State(state): State<Arc<ApiState>>,
    Path(agent_id): Path<i32>,
) -> impl IntoResponse {
    match queries::get_dedupe_run_by_agent_id(&state.pool, agent_id).await {
        Ok(Some(run)) => match queries::mark_dedupe_run_addressed(&state.pool, run.id).await {
            Ok(_) => json_ok!(MarkAddressedResponse {
                success: true,
                message: "Dedupe marked as addressed".to_string(),
            }),
            Err(e) => json_err!(
                INTERNAL_SERVER_ERROR,
                MarkAddressedResponse {
                    success: false,
                    message: format!("Failed to mark addressed: {}", e),
                }
            ),
        },
        Ok(None) => json_err!(
            NOT_FOUND,
            MarkAddressedResponse {
                success: false,
                message: "No dedupe run found for this agent".to_string(),
            }
        ),
        Err(e) => json_err!(
            INTERNAL_SERVER_ERROR,
            MarkAddressedResponse {
                success: false,
                message: format!("Database error: {}", e),
            }
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct CloseDuplicatesRequest {
    pub canonical_issue_number: u32,
    pub duplicate_issue_numbers: Vec<u32>,
    #[serde(default = "default_repo")]
    pub repo: String,
    pub github_token: String,
}

fn default_repo() -> String {
    format!("{}/{}", DEFAULT_GITHUB_ORG, DEFAULT_PROJECT)
}

#[derive(Debug, Serialize)]
pub struct IssueCloseResult {
    pub issue_number: u32,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CloseDuplicatesResponse {
    pub success: bool,
    pub results: Vec<IssueCloseResult>,
    pub message: String,
}

/// Closes duplicate issues via GitHub API and records closures.
pub async fn close_duplicates(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<CloseDuplicatesRequest>,
) -> impl IntoResponse {
    if request.github_token.is_empty() {
        return json_err!(
            BAD_REQUEST,
            CloseDuplicatesResponse {
                success: false,
                results: vec![],
                message: "GitHub token is required".to_string(),
            }
        );
    }

    log::info!(
        "[CLOSE_DUPLICATES] Closing {} duplicates of #{} in {}",
        request.duplicate_issue_numbers.len(),
        request.canonical_issue_number,
        request.repo
    );

    let canonical_issue_url = github_issue_url(
        request.repo.split('/').next().unwrap_or(DEFAULT_GITHUB_ORG),
        request.repo.split('/').nth(1).unwrap_or(DEFAULT_PROJECT),
        request.canonical_issue_number,
    );

    // Find dedupe_run for this canonical issue
    let dedupe_run_id =
        match queries::get_dedupe_run_by_canonical_url(&state.pool, &canonical_issue_url).await {
            Ok(Some(run)) => Some(run.id),
            Ok(None) => {
                log::warn!(
                    "[CLOSE_DUPLICATES] No dedupe run found for canonical issue #{}",
                    request.canonical_issue_number
                );
                None
            }
            Err(e) => {
                log::error!(
                    "[CLOSE_DUPLICATES] Database error finding dedupe run: {}",
                    e
                );
                None
            }
        };

    let mut results = Vec::new();
    let mut success_count = 0;

    for dup_num in request.duplicate_issue_numbers {
        log::debug!("[CLOSE_DUPLICATES] Attempting to close issue #{}", dup_num);

        match close_issue_async(
            &request.repo,
            dup_num,
            request.canonical_issue_number,
            &request.github_token,
        )
        .await
        {
            Ok(()) => {
                log::info!("[CLOSE_DUPLICATES] Successfully closed issue #{}", dup_num);
                success_count += 1;

                if let Some(run_id) = dedupe_run_id {
                    let dup_url = github_issue_url(
                        request.repo.split('/').next().unwrap_or(DEFAULT_GITHUB_ORG),
                        request.repo.split('/').nth(1).unwrap_or(DEFAULT_PROJECT),
                        dup_num,
                    );
                    let closure = NewDedupeClosure {
                        dedupe_run_id: run_id,
                        closed_issue_url: dup_url,
                        closed_by: "PowerFixer".to_string(),
                    };
                    if let Err(e) = queries::create_dedupe_closure(&state.pool, &closure).await {
                        log::warn!(
                            "[CLOSE_DUPLICATES] Failed to record closure for #{}: {}",
                            dup_num,
                            e
                        );
                    }
                }

                results.push(IssueCloseResult {
                    issue_number: dup_num,
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                log::warn!(
                    "[CLOSE_DUPLICATES] Failed to close issue #{}: {}",
                    dup_num,
                    e
                );
                results.push(IssueCloseResult {
                    issue_number: dup_num,
                    success: false,
                    error: Some(e),
                });
            }
        }
    }

    let all_success = success_count == results.len();
    let message = if all_success {
        format!("Successfully closed all {} duplicate issues", success_count)
    } else {
        format!(
            "Closed {} out of {} duplicate issues",
            success_count,
            results.len()
        )
    };

    log::info!("[CLOSE_DUPLICATES] {}", message);

    json_ok!(CloseDuplicatesResponse {
        success: all_success,
        results,
        message,
    })
}
