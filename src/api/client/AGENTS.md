# Client API

This directory contains endpoints called by the **PowerFixer TUI** to manage agents and retrieve state.

## Files

### `dedupe.rs`
Deduplication result endpoints.
- `get_dedupe_result()` - Get dedupe analysis results for an agent
- `create_dedupe_closure()` - Record when a duplicate issue is closed

### `launch.rs`
Agent launching endpoint (`POST /api/v1/agent/launch`).
- `launch_agent()` - Launches a new agent task on Warp's platform
- Generates prompts based on agent type (Fix, Dedupe, Triage)
- Creates database records and broadcasts updates

### `local_agents.rs`
Local agent CRUD for agents running on user's machine.
- `create_local_agent()` - Register a local agent
- `delete_local_agent()` - Remove a local agent record

### `polling.rs`
Task status polling and background sync.
- `background_polling_loop()` - Polls Warp API every 30 seconds for active tasks
- `fetch_warp_task()` - Fetch single task status from Warp API
- `sync_agent_from_task()` - Update local database from Warp API response
- `poll_agent_statuses()` - On-demand polling for multiple tasks
- `get_task_status()` - Get status of a single task

### `state.rs`
TUI state synchronization endpoints.
- `get_full_state()` - Full state dump for TUI initial sync
- `build_agent_info()` - Build complete AgentInfo from database
- `update_agent_inbox_state()` - Update inbox archived state
- `delete_agent_by_id()` - Delete an agent
- `cache_issue_titles()` - Cache GitHub issue titles for display

### `triage.rs`
Triage run management and results.
- `create_triage_run()` - Create a new triage run with agents
- `delete_triage_run()` - Delete a triage run and its agents
- `get_excluded_issues()` - Get recently triaged issue numbers
- `create_triage_result_endpoint()` - Record a triage result
- `get_triage_results()` - Get all triage results
- `get_triage_summary()` - Get summary of triage runs
- `get_triage_coverage_endpoint()` - Get triage coverage statistics

## Common Patterns

All handlers follow a similar pattern:
1. Extract and validate request
2. Perform database operations via `crate::db::queries`
3. Broadcast state changes via `websocket::broadcast_*`
4. Return JSON response using `json_ok!` or `json_err!` macros
