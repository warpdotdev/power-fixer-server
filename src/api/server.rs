//! Main API server and router setup.
//!
//! This module contains the Axum HTTP server setup and route definitions.
//! The actual endpoint handlers are organized into domain-specific modules:
//!
//! - [`agent`]: Endpoints called by running agents (status callbacks)
//! - [`client`]: Endpoints called by the PowerFixer TUI
//! - [`websocket`]: WebSocket server for real-time updates

use axum::{
    body::Body,
    http::Request,
    middleware::{self, Next},
    response::Response,
    routing::{delete, get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};

use super::agent;
use super::client;
use super::openai::OpenAIClient;
use super::slack::SlackClient;
use super::types::ApiState;
use super::warp_api::WarpApiClient;
use super::webhook;
use super::websocket;
use super::websocket::WsMessage;
use crate::db::queries;
use crate::db::DbPool;

fn classify_request_source(path: &str) -> &'static str {
    if path == "/api/v1/agent/status" {
        "CALLBACK"
    } else if path.starts_with("/api/v1/webhook/") {
        "WEBHOOK"
    } else if path == "/ws" {
        "WEBSOCKET"
    } else if path == "/health" {
        "HEALTH"
    } else {
        "TUI"
    }
}

async fn request_logger(request: Request<Body>, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let source = classify_request_source(&path);

    let start = Instant::now();
    let response = next.run(request).await;
    let duration = start.elapsed();

    let status = response.status();
    log::info!(
        "[{}] {} {} -> {} ({:.2?})",
        source,
        method,
        path,
        status.as_u16(),
        duration
    );

    response
}

fn create_router_with_state(state: Arc<ApiState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(agent::health_check))
        .route("/api/v1/agent/launch", post(client::launch_agent))
        .route("/api/v1/agent/task/:task_id", get(client::get_task_status))
        .route("/api/v1/agent/status", post(agent::update_agent_status))
        .route("/api/v1/agent/poll", post(client::poll_agent_statuses))
        .route("/api/v1/state", get(client::get_full_state))
        .route(
            "/api/v1/inbox/agent",
            post(client::update_agent_inbox_state),
        )
        .route("/api/v1/agent/:id", delete(client::delete_agent_by_id))
        .route("/api/v1/local-agent", post(client::create_local_agent))
        .route(
            "/api/v1/local-agent/:id",
            delete(client::delete_local_agent),
        )
        .route("/api/v1/triage/run", post(client::create_triage_run))
        .route(
            "/api/v1/triage/run/:run_id",
            delete(client::delete_triage_run),
        )
        .route(
            "/api/v1/triage/excluded-issues",
            get(client::get_excluded_issues),
        )
        .route(
            "/api/v1/triage/result",
            post(client::create_triage_result_endpoint),
        )
        .route("/api/v1/triage/summary", get(client::get_triage_summary))
        .route(
            "/api/v1/triage/coverage",
            get(client::get_triage_coverage_endpoint),
        )
        .route("/api/v1/dedupe/:agent_id", get(client::get_dedupe_result))
        .route(
            "/api/v1/dedupe/closure",
            post(client::create_dedupe_closure),
        )
        .route(
            "/api/v1/dedupe/:agent_id/addressed",
            post(client::mark_dedupe_addressed),
        )
        .route(
            "/api/v1/dedupe/close-duplicates",
            post(client::close_duplicates),
        )
        .route("/api/v1/triage/results", get(client::get_triage_results))
        .route("/api/v1/issues/cache", post(client::cache_issue_titles))
        .route("/api/v1/issue/action", post(client::log_issue_action))
        // Webhook endpoints (API key auth, no IAP)
        .route("/api/v1/webhook/dedupe", post(webhook::webhook_dedupe))
        .route("/ws", get(websocket::ws_handler))
        .layer(middleware::from_fn(request_logger))
        .layer(cors)
        .with_state(state)
}

/// Syncs pending tasks from Warp's API on server startup.
async fn sync_pending_tasks_on_startup(pool: &DbPool, warp_client: &WarpApiClient) {
    let pending = match queries::get_pending_remote_agents(pool).await {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to get pending agents: {}", e);
            return;
        }
    };

    if pending.is_empty() {
        log::info!("No pending agent tasks to sync");
        return;
    }

    log::info!(
        "Syncing {} pending agent tasks from Warp API",
        pending.len()
    );

    let mut synced = 0;

    for agent in pending {
        let task_id = match &agent.task_id {
            Some(id) => id,
            None => continue,
        };

        if let Ok(task) = warp_client.get_task(task_id).await {
            if client::sync_agent_from_task(pool, &agent, &task).await {
                log::info!(
                    "Synced task {}: {:?} -> {}",
                    task_id,
                    agent.task_state,
                    task.state
                );
                synced += 1;
            }
        }
    }

    log::info!("Startup sync complete: {} tasks updated", synced);
}

/// Starts the PowerFixer API server.
///
/// This function:
/// 1. Creates the WarpApiClient (if API key available)
/// 2. Syncs pending tasks from Warp's API on startup
/// 3. Initializes the WebSocket broadcast channel
/// 4. Spawns the background polling loop
/// 5. Starts the Axum HTTP server
pub async fn run_server(pool: DbPool, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Starting callback API server on port {}", port);

    let warp_client = match WarpApiClient::new() {
        Ok(client) => {
            log::info!("Warp API client initialized ({})", client.base_url());
            Some(client)
        }
        Err(e) => {
            log::warn!("Warp API client not available: {}", e);
            None
        }
    };

    if let Some(ref client) = warp_client {
        sync_pending_tasks_on_startup(&pool, client).await;
    } else {
        log::warn!("Skipping startup sync - no Warp API client");
    }

    let slack_client = match SlackClient::new() {
        Ok(client) => {
            log::info!("Slack client initialized");
            Some(client)
        }
        Err(e) => {
            log::warn!("Slack client not available: {}", e);
            None
        }
    };

    let openai_client = match OpenAIClient::new() {
        Ok(client) => {
            log::info!("OpenAI client initialized (for issue summarization)");
            Some(client)
        }
        Err(e) => {
            log::warn!("OpenAI client not available: {}", e);
            None
        }
    };

    let (ws_broadcast, _) = broadcast::channel::<WsMessage>(100);
    let state = Arc::new(ApiState {
        pool: pool.clone(),
        ws_broadcast,
        warp_client,
        slack_client,
        openai_client,
    });

    tokio::spawn(client::background_polling_loop(state.clone()));

    let app = create_router_with_state(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    log::info!("API server listening on {}", addr);
    log::info!(
        "Background polling started (interval: {:?})",
        client::BACKGROUND_POLL_INTERVAL
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
