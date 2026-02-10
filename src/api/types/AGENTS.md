# Shared Types

This directory contains shared types and utilities used across the API layer.

## Files

### `api_state.rs`
The shared application state passed to all API handlers.
- `ApiState` - Contains the database pool, WebSocket broadcast sender, and WarpApiClient

### `callback.rs`
Request/response types for the agent callback API.
- `StatusUpdate` - Status update payload sent by agents
- `RejectedIssue` - Issue rejected during triage
- `DedupeCandidate` - Potential duplicate found by dedupe agent
- `GenericResponse` - Standard success/error response
- `HealthResponse` - Health check response
- `AgentInfo` (internal) - Agent info looked up by callback token

### `github.rs`
GitHub-related constants and utilities.
- `github_issue_url()` - Construct GitHub issue URL
- Constants: `DEFAULT_GITHUB_ORG`, `DEFAULT_PROJECT`, `DEFAULT_PROVIDER_CONFIG_ID`

### `state_info.rs`
State information types shared between HTTP endpoints and WebSocket messages.
- `AgentInfo` - Full agent information for TUI display
- `TriageRunInfo` - Triage run information
- `InboxStateInfo` - Inbox archived state
- `TriageResultInfo` - Individual triage result
- `DedupeResultInfo` - Dedupe analysis result
- `DuplicateCandidateInfo` - Duplicate issue candidate

## Re-exports

The `mod.rs` re-exports commonly used types for convenient imports:
```rust
use super::super::types::{ApiState, AgentInfo, github_issue_url, ...};
```

## Design Notes

- Types that are used by both HTTP endpoints and WebSocket messages live here
- Types specific to a single endpoint stay in that endpoint's module
- The `state_info` types avoid circular dependencies between client and websocket modules
- Warp API client and types are in the separate `warp_api/` module
