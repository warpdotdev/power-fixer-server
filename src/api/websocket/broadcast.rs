//! WebSocket broadcast helpers.
//!
//! Centralized helpers for broadcasting state changes to all connected clients.
//! Every mutation should broadcast its changes through these helpers.

use sqlx::PgPool;
use tokio::sync::broadcast;

use super::super::client::state::build_agent_info;
use super::super::types::{InboxStateInfo, TriageRunInfo};
use super::types::WsMessage;
use crate::db::queries;

/// Broadcasts an agent update (create or update) to all connected clients.
pub async fn broadcast_agent_update(
    pool: &PgPool,
    ws_tx: &broadcast::Sender<WsMessage>,
    agent_id: i32,
) {
    if let Ok(Some(db_agent)) = queries::get_agent_by_id(pool, agent_id).await {
        let agent_info = build_agent_info(pool, db_agent).await;
        let _ = ws_tx.send(WsMessage::AgentUpdate {
            agent: Box::new(agent_info),
        });
    }
}

/// Broadcasts an agent deletion to all connected clients.
pub fn broadcast_agent_deleted(ws_tx: &broadcast::Sender<WsMessage>, agent_id: i32) {
    let _ = ws_tx.send(WsMessage::AgentDeleted { agent_id });
}

/// Broadcasts a triage run update to all connected clients.
pub async fn broadcast_triage_run_update(
    pool: &PgPool,
    ws_tx: &broadcast::Sender<WsMessage>,
    triage_run_id: i32,
) {
    if let Ok(Some(run)) = queries::get_triage_run_by_id(pool, triage_run_id).await {
        let run_agents = queries::get_triage_run_agents(pool, run.id)
            .await
            .unwrap_or_default();
        let triage_run = TriageRunInfo {
            id: run.id,
            started_at: run.started_at.to_rfc3339(),
            min_external_id: run.min_external_id.clone(),
            max_external_id: run.max_external_id.clone(),
            agent_ids: run_agents.iter().map(|a| a.agent_id).collect(),
        };
        let _ = ws_tx.send(WsMessage::TriageRunUpdate {
            triage_run: Box::new(triage_run),
        });
    }
}

/// Broadcasts an inbox state change to all connected clients.
pub fn broadcast_inbox_state_update(
    ws_tx: &broadcast::Sender<WsMessage>,
    agent_id: i32,
    is_archived: bool,
) {
    let inbox_state = InboxStateInfo {
        agent_id,
        is_archived,
    };
    let _ = ws_tx.send(WsMessage::InboxStateUpdate {
        inbox_state: Box::new(inbox_state),
    });
}
