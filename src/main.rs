//! PowerFixer Server entry point.
//!
//! This binary starts the PowerFixer API server, which acts as the central coordinator
//! for all agent-related operations. It:
//! - Connects to PostgreSQL and runs migrations
//! - Starts the Axum HTTP server with REST API endpoints
//! - Spawns a background polling loop to sync task states from Warp's API
//! - Provides WebSocket support for real-time updates to TUI clients

use std::env;
use std::io;

mod api;
mod config;
mod db;
mod prompts;
mod utils;

use log::{debug, error, info, LevelFilter};

struct Args {
    port: u16,
    log_level: LevelFilter,
}

fn parse_args() -> Args {
    let mut args = Args {
        port: 3001,
        log_level: LevelFilter::Info,
    };

    let mut iter = env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--port" => {
                if let Some(port_str) = iter.next() {
                    args.port = port_str.parse().unwrap_or(3001);
                }
            }
            "--log-level" => {
                if let Some(level) = iter.next() {
                    args.log_level = match level.to_lowercase().as_str() {
                        "trace" => LevelFilter::Trace,
                        "debug" => LevelFilter::Debug,
                        "info" => LevelFilter::Info,
                        "warn" => LevelFilter::Warn,
                        "error" => LevelFilter::Error,
                        "off" => LevelFilter::Off,
                        _ => LevelFilter::Info,
                    };
                }
            }
            _ => {}
        }
    }
    args
}

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    dotenvy::dotenv().ok();

    let args = parse_args();

    env_logger::Builder::new()
        .filter_level(args.log_level)
        .init();

    debug!("Debug logging enabled");
    info!("PowerFixer Server starting up");

    info!("Connecting to database...");
    let pool = match db::create_pool().await {
        Ok(pool) => {
            info!("Database connection established");
            pool
        }
        Err(e) => {
            error!("Database connection required: {}", e);
            std::process::exit(1);
        }
    };

    info!("Starting PowerFixer API server on port {}...", args.port);
    if let Err(e) = api::server::run_server(pool, args.port).await {
        error!("API server error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
