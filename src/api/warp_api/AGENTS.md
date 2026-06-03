# Warp REST API Client

This directory contains the client for interacting with Oz's agent management REST API.

## Files

### `client.rs`
The HTTP client for Warp's API.

**`WarpApiClient`** - Main client struct with methods:
- `new()` - Creates client, fetching API key from env/gcloud
- `with_config(api_key, base_url)` - Creates client with explicit config
- `launch_agent(request)` - `POST /agent/run` to launch a new agent
- `get_task(task_id)` - `GET /agent/tasks/{id}` for basic status
- `get_task_detail(task_id)` - `GET /agent/tasks/{id}` for detailed status
- `base_url()` - Returns configured base URL

**Helper functions:**
- `get_api_key()` - Gets API key from `WARP_API_KEY` env var or GCP Secret Manager
- `get_api_base_url()` - Gets base URL from `WARP_API_BASE_URL` or uses production default

### `types.rs`
Request/response types and error handling.

**Error type:**
- `WarpApiError` - Enum covering NoApiKey, HttpError, ApiError, ParseError

**Request types:**
- `LaunchAgentRequest` - Request body for launching an agent
- `TaskConfig` - Environment configuration for tasks

**Response types:**
- `LaunchAgentResponse` - Response from launch (contains task_id)
- `TaskResponse` - Basic task status response
- `TaskDetailResponse` - Detailed task status with result/error

**Utilities:**
- `parse_task_state()` - Converts Warp API state strings to `AgentTaskState` enum

## Usage

The `WarpApiClient` is created once at server startup and stored in `ApiState`:

```rust
let warp_client = WarpApiClient::new()?;

// Launch an agent
let response = warp_client.launch_agent(LaunchAgentRequest {
    prompt: "...",
    config: Some(TaskConfig { environment_id: "..." }),
    agent_profile_id: Some("..."),
    secrets: None,
}).await?;

// Get task status
let task = warp_client.get_task(&task_id).await?;
```

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/agent/run` | Launch a new agent task |
| GET | `/agent/tasks/{task_id}` | Get task status |

## Environment Variables

- `WARP_API_KEY` - API key for authentication
- `WARP_API_BASE_URL` - Override base URL (default: `https://warp.dev/api/v1`). Honored
  verbatim, so it may point at an internal Private Service Connect endpoint such as
  `http://<internal-ip>/api/v1` (plain HTTP, no TLS assumptions).
- `POWERFIXER_GCP_PROJECT` - GCP project for Secret Manager fallback
- `POWERFIXER_WARP_API_KEY_SECRET` - Secret name for API key (default: `powerfixer-warp-api-key`)

## Run / session links

`session_link` (polling responses) and `session_url` (agent callbacks) are produced
upstream by Warp's API and by the agents themselves; the server never derives them from
`WARP_API_BASE_URL`. Pointing the API base URL at an internal endpoint therefore has no
effect on the public Oz run links surfaced to users (e.g. in Slack), so no separate
run-link base URL is required.
