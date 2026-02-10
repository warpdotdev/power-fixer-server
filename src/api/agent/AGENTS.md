# Agent API

This directory contains endpoints called by **running agents** to report their status back to the server.

## Files

### `callback.rs`
The main agent callback endpoint (`POST /api/v1/agent/status`).

**Key functions:**
- `update_agent_status()` - Handles status updates from running agents
- `health_check()` - Simple health check endpoint (`GET /health`)

**Authentication:** Agents authenticate using a Bearer token in the Authorization header. This token is generated when the agent is launched and stored in the database.

## Data Flow

```
Running Agent
     │
     │ POST /api/v1/agent/status
     │ Authorization: Bearer <callback_token>
     ▼
┌─────────────────────────────────────┐
│  callback.rs::update_agent_status   │
│                                     │
│  1. Validate callback token         │
│  2. Parse status update             │
│  3. Update agent in database        │
│  4. Process agent-type-specific     │
│     results (dedupe/triage)         │
│  5. Broadcast via WebSocket         │
└─────────────────────────────────────┘
```

## Status Update Payloads

Agents send JSON payloads containing:
- `state`: Current state (QUEUED, IN_PROGRESS, SUCCEEDED, FAILED)
- `branch_name`: Git branch created (optional)
- `pr_url`: Pull request URL (optional)
- `session_url`: Warp session URL (optional)
- `summary`: Human-readable summary (optional)
- Agent-type-specific fields (dedupe results, triage candidates, etc.)
