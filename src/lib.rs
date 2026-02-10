//! PowerFixer Server library crate.
//!
//! This crate provides the core functionality for the PowerFixer API server,
//! which coordinates agent operations for automated GitHub issue processing.
//!
//! # Modules
//!
//! - [`api`]: HTTP and WebSocket API handlers
//! - [`db`]: Database models, queries, and connection management
//! - [`prompts`]: Agent prompt template loading and generation
//! - [`utils`]: Debug logging utilities

pub mod api;
pub mod config;
pub mod db;
pub mod prompts;
pub mod utils;
