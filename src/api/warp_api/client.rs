//! HTTP client for Warp's REST API.

use std::process::Command;

use super::types::{
    LaunchAgentRequest, LaunchAgentResponse, TaskDetailResponse, TaskResponse, WarpApiError,
};
use crate::config;

/// Client for interacting with Warp's agent REST API.
///
/// This client holds a reusable HTTP client for connection pooling
/// and centralizes authentication and error handling.
pub struct WarpApiClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl WarpApiClient {
    /// Creates a new WarpApiClient, fetching the API key from environment or gcloud.
    ///
    /// Returns an error if no API key is available.
    pub fn new() -> Result<Self, WarpApiError> {
        let api_key = get_api_key().ok_or(WarpApiError::NoApiKey)?;
        let base_url = get_api_base_url();

        Ok(Self {
            http: reqwest::Client::new(),
            base_url,
            api_key,
        })
    }

    /// Creates a new WarpApiClient with explicit configuration.
    #[allow(dead_code)]
    pub fn with_config(api_key: String, base_url: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
            api_key,
        }
    }

    /// Launches a new agent task.
    ///
    /// Calls `POST /agent/run` to start a new agent with the given prompt and configuration.
    pub async fn launch_agent(
        &self,
        request: LaunchAgentRequest,
    ) -> Result<LaunchAgentResponse, WarpApiError> {
        let url = format!("{}/agent/run", self.base_url);

        log::debug!("Launching agent via {}", url);

        let response = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            response.json().await.map_err(|e| {
                WarpApiError::ParseError(format!("Failed to parse launch response: {}", e))
            })
        } else {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            Err(WarpApiError::ApiError { status, body })
        }
    }

    /// Gets the status of a task (basic response).
    ///
    /// Calls `GET /agent/tasks/{task_id}` to fetch the current task state.
    pub async fn get_task(&self, task_id: &str) -> Result<TaskResponse, WarpApiError> {
        let url = format!("{}/agent/tasks/{}", self.base_url, task_id);

        log::debug!("Fetching task status: {}", task_id);

        let response = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if response.status().is_success() {
            response.json().await.map_err(|e| {
                WarpApiError::ParseError(format!("Failed to parse task response: {}", e))
            })
        } else {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            Err(WarpApiError::ApiError { status, body })
        }
    }

    /// Gets the detailed status of a task.
    ///
    /// Calls `GET /agent/tasks/{task_id}` and returns detailed information
    /// including result and error_message fields.
    pub async fn get_task_detail(&self, task_id: &str) -> Result<TaskDetailResponse, WarpApiError> {
        let url = format!("{}/agent/tasks/{}", self.base_url, task_id);

        log::debug!("Fetching detailed task status: {}", task_id);

        let response = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if response.status().is_success() {
            response.json().await.map_err(|e| {
                WarpApiError::ParseError(format!("Failed to parse task detail response: {}", e))
            })
        } else {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            Err(WarpApiError::ApiError { status, body })
        }
    }

    /// Returns the base URL this client is configured to use.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

/// Gets the Warp API base URL from environment or uses the default.
pub fn get_api_base_url() -> String {
    config::warp_api_base_url()
}

/// Retrieves the API key from environment or Google Secret Manager.
pub fn get_api_key() -> Option<String> {
    config::warp_api_key().or_else(|| {
        let project = config::gcp_project()?;
        let secret = config::warp_api_key_secret_name();

        let output = Command::new("gcloud")
            .args([
                "secrets",
                "versions",
                "access",
                "latest",
                "--secret",
                &secret,
                "--project",
                &project,
            ])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let key = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if key.is_empty() {
            None
        } else {
            Some(key)
        }
    })
}
