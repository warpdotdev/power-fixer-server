//! Client API endpoints called by the PowerFixer TUI.
//!
//! These endpoints handle requests from the TUI client for:
//! - [`dedupe`]: Retrieving deduplication results and recording closures
//! - [`issue_actions`]: Logging user actions on issues for audit trail
//! - [`launch`]: Launching new agent tasks
//! - [`local_agents`]: Managing local agent records
//! - [`polling`]: Task status polling and background sync
//! - [`state`]: Full state synchronization and inbox management
//! - [`triage`]: Triage run management and results

pub mod dedupe;
pub mod issue_actions;
pub mod launch;
pub mod local_agents;
pub mod polling;
pub mod state;
pub mod triage;

pub use dedupe::{
    close_duplicates, create_dedupe_closure, get_dedupe_result, mark_dedupe_addressed,
};
pub use issue_actions::log_issue_action;
pub use launch::launch_agent;
pub use local_agents::{create_local_agent, delete_local_agent};
pub use polling::{
    background_polling_loop, get_task_status, poll_agent_statuses, sync_agent_from_task,
    BACKGROUND_POLL_INTERVAL,
};
pub use state::{cache_issue_titles, delete_agent_by_id, get_full_state, update_agent_inbox_state};
pub use triage::{
    create_triage_result_endpoint, create_triage_run, delete_triage_run, get_excluded_issues,
    get_triage_coverage_endpoint, get_triage_results, get_triage_summary,
};
