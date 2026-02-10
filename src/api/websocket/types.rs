//! WebSocket message type definitions.
//!
//! These types are serialized to JSON and sent over WebSocket connections.
//! They are shared between the server and TUI client.

use serde::{Deserialize, Serialize};

use super::super::types::{AgentInfo, InboxStateInfo, TriageRunInfo};

/// Top-level WebSocket message envelope.
///
/// Uses serde's internally tagged representation with a `type` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// Agent created or updated - client should upsert by id
    #[serde(rename = "agent_update")]
    AgentUpdate { agent: Box<AgentInfo> },
    /// Agent deleted - client should remove by id
    #[serde(rename = "agent_deleted")]
    AgentDeleted { agent_id: i32 },
    /// Triage run created or updated - client should upsert by id
    #[serde(rename = "triage_run_update")]
    TriageRunUpdate { triage_run: Box<TriageRunInfo> },
    /// Inbox state changed - client should update cache
    #[serde(rename = "inbox_state_update")]
    InboxStateUpdate { inbox_state: Box<InboxStateInfo> },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,
}
