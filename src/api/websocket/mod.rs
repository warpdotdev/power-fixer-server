//! WebSocket server for real-time updates to TUI clients.
//!
//! This module handles WebSocket connections and message broadcasting:
//! - [`broadcast`]: Helper functions to broadcast state changes
//! - [`handler`]: WebSocket upgrade handler and connection management
//! - [`types`]: Message type definitions

pub mod broadcast;
pub mod handler;
pub mod types;

pub use broadcast::{
    broadcast_agent_deleted, broadcast_agent_update, broadcast_inbox_state_update,
    broadcast_triage_run_update,
};
pub use handler::ws_handler;
pub use types::WsMessage;
