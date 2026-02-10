//! Agent callback endpoints.
//!
//! This module handles status updates from running agents.
//! Agents call these endpoints to report their progress back to the server.
//!
//! - [`callback`]: The `/api/v1/agent/status` endpoint

pub mod callback;

pub use callback::{health_check, update_agent_status};
