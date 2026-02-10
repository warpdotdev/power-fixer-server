//! Database layer for PostgreSQL persistence.
//!
//! This module provides database connection management, models, and queries
//! for all PowerFixer state:
//!
//! - [`models`]: SQLx data models and enum types
//! - [`queries`]: All database operations
//!
//! # Connection Pool
//!
//! Use [`create_pool`] to establish a connection pool from the `DATABASE_URL`
//! environment variable. Migrations are run automatically at startup.

pub mod models;
pub mod queries;

use log::info;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

/// Type alias for the PostgreSQL connection pool.
pub type DbPool = PgPool;

/// Creates a database connection pool from the `DATABASE_URL` environment variable
/// and runs any pending migrations.
pub async fn create_pool() -> Result<DbPool, Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(30))
        .connect(&database_url)
        .await?;

    info!("Running database migrations...");
    sqlx::migrate!().run(&pool).await?;
    info!("Migrations complete");

    Ok(pool)
}
