//! Issue action logging endpoint.
//!
//! This module handles logging user actions taken on issues (comments, labels, etc.)
//! for audit trail and analytics purposes.

use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::super::types::{ApiState, DEFAULT_PROVIDER_CONFIG_ID};
use crate::db::models::{IssueActionType, NewIssue, NewIssueAction};
use crate::db::queries;

#[derive(Debug, Deserialize)]
pub struct LogIssueActionRequest {
    pub external_id: String,
    pub repo: String,
    pub action_type: IssueActionType,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub comment: Option<String>,
    pub performed_by: String,
}

#[derive(Debug, Serialize)]
pub struct LogIssueActionResponse {
    pub success: bool,
    pub action_id: Option<i32>,
    pub message: String,
}

pub async fn log_issue_action(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<LogIssueActionRequest>,
) -> impl IntoResponse {
    let new_issue = NewIssue {
        provider_config_id: DEFAULT_PROVIDER_CONFIG_ID,
        external_id: request.external_id.clone(),
        external_url: None,
        project: request.repo.clone(),
        title: None,
        labels: vec![],
    };

    let issue = match queries::upsert_issue(&state.pool, &new_issue).await {
        Ok(issue) => issue,
        Err(e) => {
            log::warn!(
                "Failed to upsert issue for action log ({}#{}): {}",
                request.repo,
                request.external_id,
                e
            );
            return json_err!(
                INTERNAL_SERVER_ERROR,
                LogIssueActionResponse {
                    success: false,
                    action_id: None,
                    message: format!("Failed to log issue action: {}", e),
                }
            );
        }
    };

    let details = build_action_details(&request);

    let new_action = NewIssueAction {
        issue_id: issue.id,
        action_type: request.action_type,
        details,
        performed_by: request.performed_by,
    };

    match queries::create_issue_action(&state.pool, &new_action).await {
        Ok(action) => {
            log::debug!(
                "Logged {:?} action on {}#{} by {}",
                request.action_type,
                request.repo,
                request.external_id,
                new_action.performed_by
            );
            json_ok!(LogIssueActionResponse {
                success: true,
                action_id: Some(action.id),
                message: "Action logged".to_string(),
            })
        }
        Err(e) => {
            log::warn!("Failed to create issue action: {}", e);
            json_err!(
                INTERNAL_SERVER_ERROR,
                LogIssueActionResponse {
                    success: false,
                    action_id: None,
                    message: format!("Failed to log issue action: {}", e),
                }
            )
        }
    }
}

fn build_action_details(request: &LogIssueActionRequest) -> Option<serde_json::Value> {
    let mut details = serde_json::Map::new();

    if let Some(ref label) = request.label {
        details.insert(
            "label".to_string(),
            serde_json::Value::String(label.clone()),
        );
    }

    if let Some(ref comment) = request.comment {
        details.insert(
            "comment".to_string(),
            serde_json::Value::String(comment.clone()),
        );
    }

    if details.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(details))
    }
}
