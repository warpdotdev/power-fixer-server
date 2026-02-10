//! Shared application state for API handlers.

use tokio::sync::broadcast;

use super::super::openai::OpenAIClient;
use super::super::slack::SlackClient;
use super::super::warp_api::WarpApiClient;
use super::super::websocket::types::WsMessage;
use crate::db::DbPool;

/// Shared application state passed to all API handlers.
pub struct ApiState {
    pub pool: DbPool,
    pub ws_broadcast: broadcast::Sender<WsMessage>,
    pub warp_client: Option<WarpApiClient>,
    pub slack_client: Option<SlackClient>,
    pub openai_client: Option<OpenAIClient>,
}
