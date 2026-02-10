//! Shared types and utilities for the API layer.
//!
//! This module contains:
//! - [`api_state`]: Shared application state (`ApiState`)
//! - [`callback`]: Request/response types for agent callbacks
//! - [`github`]: GitHub-related constants and utilities
//! - [`state_info`]: State information types shared across endpoints and WebSocket

pub mod api_state;
pub mod callback;
pub mod github;
pub mod github_api;
pub mod state_info;

pub use api_state::ApiState;
pub(crate) use callback::AgentInfo as CallbackAgentInfo;
pub use callback::{GenericResponse, HealthResponse, StatusUpdate};
pub use github::{
    github_issue_url, DEFAULT_GITHUB_ORG, DEFAULT_PROJECT, DEFAULT_PROVIDER_CONFIG_ID,
};
pub use github_api::{close_issue_async, fetch_issue_metadata_async, truncate_text};
pub use state_info::{
    AgentInfo, DedupeClosureInfo, DedupeResultInfo, DuplicateCandidateInfo, InboxStateInfo,
    TriageResultInfo, TriageRunInfo,
};
