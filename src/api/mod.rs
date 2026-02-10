//! HTTP and WebSocket API layer.
//!
//! This module contains all API handlers for the PowerFixer server:
//!
//! - [`agent`]: Endpoints called by running agents to report status
//! - [`client`]: Endpoints called by the PowerFixer TUI
//! - [`macros`]: Response helper macros
//! - [`openai`]: OpenAI integration for summarizing GitHub issues
//! - [`server`]: Main Axum server and router setup
//! - [`slack`]: Slack integration for broadcasting agent events
//! - [`types`]: Shared types and utilities
//! - [`warp_api`]: Client for Warp's REST API
//! - [`webhook`]: Webhook endpoints for external integrations (GitHub Actions)
//! - [`websocket`]: WebSocket server for real-time updates

#[macro_use]
pub mod macros;

pub mod agent;
pub mod client;
pub mod openai;
pub mod server;
pub mod slack;
pub mod types;
pub mod warp_api;
pub mod webhook;
pub mod websocket;
