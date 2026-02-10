//! Warp REST API client.
//!
//! This module provides a clean abstraction for all interactions with Warp's
//! agent management API. It centralizes HTTP client management, authentication,
//! and error handling.
//!
//! - [`client`]: The `WarpApiClient` struct and helper functions
//! - [`types`]: Request/response types and error definitions

pub mod client;
pub mod types;

pub use client::WarpApiClient;
pub use types::{parse_task_state, LaunchAgentRequest, TaskConfig, TaskResponse, WarpApiError};
