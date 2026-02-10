# Power Fixer Server

The API server component of Power Fixer, providing the backend for GitHub issue triage with AI agent support. This server is the **single source of truth** for all agent-related state and is designed to run in Google Cloud (Cloud Run).

Quick setup guide: [`warpdotdev/power-fixer-setup`](https://github.com/warpdotdev/power-fixer-setup)

## Quick Start

### Prerequisites
- **Rust** (install via [rustup](https://rustup.rs/))
- **PostgreSQL** - Database for state management
- **Warp API Key** - set `WARP_API_KEY` in your environment (or configure Secret Manager fallback)
- **(Optional) ngrok** - For local development with external agent callbacks
- **(Optional) gcloud CLI** - `brew install google-cloud-sdk` then `gcloud auth login` (for Secret Manager and Cloud Run workflows)

### Database Setup
```bash
# Create the database (requires PostgreSQL running)
createdb powerfixer

# Copy and configure environment
cp .env.example .env
# Edit .env with your DATABASE_URL (default: postgres://postgres@localhost/powerfixer)
```

### Build & Run
```bash
# Build
cargo build --release

# Run with the helper script (optionally loads secrets from Secret Manager)
./script/server

# Or run directly
./target/release/power-fixer-server --port 3001

# With ngrok tunnel for external access
./script/server --ngrok
```

## Configuration

Create a `.env` file (see `.env.example`):

```bash
# Database connection (required)
DATABASE_URL=postgres://postgres@localhost/powerfixer

# Warp Platform API key (required for launching remote agents)
WARP_API_KEY=your_api_key
# WARP_API_BASE_URL=https://warp.dev/api/v1

# Server port (default: 3001)
POWERFIXER_CALLBACK_PORT=3001

# ngrok domain for external access (optional)
NGROK_DOMAIN=your-domain.ngrok-free.dev

# Optional Secret Manager fallback (when WARP_API_KEY is unset)
# POWERFIXER_GCP_PROJECT=your-gcp-project
# POWERFIXER_WARP_API_KEY_SECRET=powerfixer-warp-api-key

# Log level
RUST_LOG=info
```

## API Endpoints

### Agent Management
- `POST /api/v1/agent/launch` - Launch an cloud agent
- `GET /api/v1/agent/task/:task_id` - Get task status
- `POST /api/v1/agent/status` - Callback endpoint for agent status updates
- `POST /api/v1/agent/poll` - Poll multiple task statuses
- `DELETE /api/v1/agent/:id` - Delete agent record by ID

### State Synchronization
- `GET /api/v1/state` - Get full state for TUI sync
- `POST /api/v1/inbox/agent` - Update agent inbox read/archived state

### Local Agents
- `POST /api/v1/local-agent` - Create local agent record
- `DELETE /api/v1/local-agent/:id` - Delete local agent
- `PATCH /api/v1/local-agent/:id/pid` - Update local agent PID

### Triage Agents
- `POST /api/v1/triage/run` - Create triage run with one or more agents
- `DELETE /api/v1/triage/run/:run_id` - Delete triage run and linked agents
- `GET /api/v1/triage/excluded-issues` - Get recently triaged issues
- `GET /api/v1/triage/summary` - Get triage run summary
- `GET /api/v1/triage/coverage` - Get triage coverage stats
- `GET /api/v1/triage/results` - Get triage results

### Dedupe
- `GET /api/v1/dedupe/:agent_id` - Get dedupe result
- `POST /api/v1/dedupe/closure` - Record dedupe closure
- `POST /api/v1/dedupe/:agent_id/addressed` - Mark dedupe run as reviewed
- `POST /api/v1/dedupe/close-duplicates` - Close selected duplicates in GitHub

### Other
- `GET /ws` - WebSocket connection for real-time updates
- `GET /health` - Health check
- `POST /api/v1/issues/cache` - Cache issue titles
- `POST /api/v1/webhook/dedupe` - Trigger dedupe from external webhook

## Architecture

The server has three main components:

### 1. REST API
Axum-based HTTP server handling all agent and state management operations.

### 2. WebSocket Server
Pushes real-time updates to connected TUI clients when agent states change.

### 3. Background Polling Loop
Runs every 5 seconds, polling Warp's REST API for all active tasks and broadcasting state changes via WebSocket.

```
┌─────────────────────────────────────────┐
│           PowerFixer Server             │
│  ┌─────────────┐  ┌───────────────────┐ │
│  │  REST API   │  │  WebSocket Server │ │
│  └─────────────┘  └───────────────────┘ │
│         │                   ▲           │
│         ▼                   │           │
│  ┌─────────────────────────────────────┐│
│  │         Postgres Database           ││
│  └─────────────────────────────────────┘│
│         ▲                   ▲           │
│         │                   │           │
│  ┌──────┴──────┐    ┌───────┴────────┐  │
│  │ Callback API│    │ Warp API Poll  │  │
│  └─────────────┘    └────────────────┘  │
└─────────┬───────────────────┬───────────┘
          │                   │
    Cloud Agents       Warp REST API
```

## Deployment

### Docker

```bash
docker build -t power-fixer-server .
docker run -p 3001:3001 \
  -e DATABASE_URL=postgres://... \
  -e WARP_API_KEY=... \
  power-fixer-server
```

### Cloud Run

The included Dockerfile and workflow templates can be used for Cloud Run deployment:

```bash
# Build and push to Artifact Registry
gcloud builds submit --tag us-central1-docker.pkg.dev/PROJECT_ID/power-fixer/server

# Deploy to Cloud Run
gcloud run deploy power-fixer-server \
  --image us-central1-docker.pkg.dev/PROJECT_ID/power-fixer/server \
  --platform managed \
  --set-env-vars DATABASE_URL=... \
  --set-secrets WARP_API_KEY=powerfixer-warp-api-key:latest
```

You can also provide `WARP_API_KEY` directly with `--set-env-vars` if you are not using Secret Manager.

## Database

The server uses PostgreSQL with the following tables:
- **provider_configs** - Issue provider configuration (GitHub org, etc.)
- **issues** - Tracked issues with external IDs and metadata
- **agents** - Unified agent table (fix, dedupe, triage types)
- **agent_status_updates** - History of agent state changes
- **issue_actions** - Audit log of actions taken on issues
- **triage_runs** - Power triage batch run metadata
- **triage_run_agents** - Junction table linking triage runs to agents
- **triage_results** - Per-issue triage evaluation results
- **dedupe_runs** - Dedupe analysis runs
- **dedupe_duplicates** - Potential duplicates found by dedupe
- **dedupe_closures** - Record of issues closed as duplicates
- **agent_inbox_state** - Read/archived status for agent inbox
- **issue_inbox_state** - Read/archived status for issue inbox

Migrations are in the `migrations/` directory and run automatically on startup.

## Agent Callback Protocol

Agents report status via the callback API:

1. Agent receives callback token at launch
2. Agent makes authenticated POST to `/api/v1/agent/status`:
```json
{
  "state": "IN_PROGRESS",
  "summary": "Investigating issue...",
  "branch_name": "fix/issue-123",
  "pr_url": "https://github.com/...",
  "session_url": "https://..."
}
```
3. Server updates DB and broadcasts via WebSocket

**Note:** Agents use a Python script for callbacks (not curl) due to environment restrictions on some runner machines.

## Development Model

Internal development continues in private repositories. The open-source repository mirrors selected releases and changes.
