# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Build & Run Commands

```bash
# Start the server (fetches secrets from gcloud, runs migrations)
./script/server              # Starts server on port 3001
./script/server --ngrok      # Starts server with ngrok tunnel for external access
./script/server --port 8080  # Use a different port

# Development commands
cargo check     # Check compilation
cargo fmt       # Format
cargo clippy    # Lint
cargo build --release  # Build release binary

# Run directly (after building)
./target/release/power-fixer-server --port 3001
```

## System Architecture

PowerFixer Server is an Axum-based API server designed to run in Google Cloud (Cloud Run). It is the **single source of truth** for all agent-related state.

### Components

1. **REST API** - Axum router handling all agent and state management operations
2. **WebSocket Server** - Pushes real-time updates to connected TUI clients
3. **Background Polling Loop** - Polls Warp's REST API every 5 seconds for active tasks
4. **Callback API** - Receives status updates from running cloud agents
5. **Postgres Database** - Stores all persistent state

### Data Flow

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

## Module Structure

### API Layer (`src/api/`)
- `server.rs` - Axum router setup, health endpoint, background polling loop
- `macros.rs` - Response helper macros
- `agent/` - Agent callback endpoint (`callback.rs`)
- `client/` - TUI client endpoints:
  - `launch.rs` - Agent launch endpoint
  - `state.rs` - Full state sync endpoint
  - `triage.rs` - Triage run management
  - `dedupe.rs` - Dedupe results and closures
  - `local_agents.rs` - Local agent management
  - `polling.rs` - Task status polling
  - `issue_actions.rs` - Issue action logging
- `types/` - Shared API types and request/response structs
- `warp_api/` - Warp REST API client
- `websocket/` - WebSocket server:
  - `handler.rs` - Connection handling
  - `broadcast.rs` - Broadcast helpers
  - `types.rs` - WebSocket message types

### Data Layer (`src/db/`)
- `mod.rs` - Connection pool creation, migration runner
- `models.rs` - SQLx data models and enums
- `queries.rs` - All database queries

### Other
- `main.rs` - Server entry point, CLI args, startup logic
- `prompts.rs` - Agent prompt template loading and generation
- `utils.rs` - Debug logging utilities

## API Endpoints

### Agent Management
- `POST /api/v1/agent/launch` - Launch an cloud agent (calls Warp API)
- `GET /api/v1/agent/task/:task_id` - Get task status from Warp API
- `POST /api/v1/agent/status` - Callback endpoint for agent status updates
- `POST /api/v1/agent/poll` - Poll multiple task statuses

### State Sync
- `GET /api/v1/state` - Full state dump for TUI initial sync
- `POST /api/v1/inbox/state` - Update inbox read/archived state

### WebSocket
- `GET /ws` - WebSocket connection for real-time updates

## Environment Variables

- `DATABASE_URL` - Postgres connection string (required)
- `WARP_API_KEY` - API key for Warp's management API
- `GITHUB_TOKEN` - GitHub personal access token with `repo` scope (optional; only needed for local server testing. In production, tokens are provided by TUI clients)
- `POWERFIXER_CALLBACK_PORT` - Port for server (default: 3001)
- `NGROK_DOMAIN` - ngrok domain for tunneling (optional)
- `RUST_LOG` - Log level (error, warn, info, debug, trace)

## Critical Invariants

1. **Server is source of truth** - All persistent state lives in Postgres
2. **WebSocket broadcasts on state change** - Any DB update should broadcast to connected clients
3. **Background polling keeps state fresh** - Polls Warp API every 30 seconds for active tasks
4. **Callback API is authenticated** - All callbacks require valid Bearer token
5. **All agent communication goes through server** - TUI clients never talk directly to Warp API

## Development Guidelines

### Import Style
- Always use `use` statements at the top of files instead of inline full paths
- Only use full paths inline (e.g., `crate::module::func()`) when necessary to avoid name conflicts
- Group imports: std first, then external crates, then crate/super imports

### Compilation Warnings
- **Always fix unused variable warnings** - They often indicate broken functionality
- **Run `cargo check` after changes** - Verify compilation before considering a change complete
- **Run `cargo fmt`** - Format code before committing

### Database Changes
- Add migrations in `migrations/` directory with timestamp prefix
- Migrations run automatically on server startup
- Use SQLx query macros for type-safe queries

### Adding API Endpoints
1. Add route in `server.rs` `create_router_with_state()`
2. Add handler function in `server.rs` or `callback.rs`
3. Add any needed queries in `db/queries.rs`
4. Add models in `db/models.rs` if needed
5. Broadcast via WebSocket if state changes

### Agent Prompts
- Templates are in `prompts/` directory
- `prompts.rs` loads and substitutes variables
- Server generates full prompts when launching agents

## Production Deployment

The server runs in GCP project `warp-power-fixer`. Terraform configuration is at `warp-terraform/environments/power-fixer/`.

### Infrastructure Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    powerfixer.warp.dev                          │
│                  (Global HTTPS Load Balancer)                   │
└─────────────────────────┬───────────────────────────────────────┘
                          │
              ┌───────────┴───────────┐
              │       URL Map         │
              │  (path-based routing) │
              └───────────┬───────────┘
                          │
          ┌───────────────┴───────────────┐
          │                               │
          ▼                               ▼
┌─────────────────────┐       ┌─────────────────────┐
│  IAP Backend        │       │  Callback Backend   │
│  (all other paths)  │       │  (/api/v1/agent/*)  │
│                     │       │  No IAP - public    │
│  Requires IAP auth  │       │  Bearer token auth  │
└─────────┬───────────┘       └─────────┬───────────┘
          │                             │
          └──────────────┬──────────────┘
                         │
                         ▼
          ┌─────────────────────────────┐
          │    Cloud Run Service        │
          │    (internal ingress only)  │
          │                             │
          │    power-fixer-server       │
          └─────────────────────────────┘
```

### Key Components

- **Cloud Run Service**: `power-fixer-server` with `INGRESS_TRAFFIC_INTERNAL_LOAD_BALANCER`
- **Load Balancer**: Global HTTPS LB at `powerfixer.warp.dev`
- **IAP Backend**: Protects TUI/admin endpoints (requires Google auth via FTE group)
- **Callback Backend**: Public endpoint for agent callbacks at `/api/v1/agent/*`
- **Cloud SQL**: Postgres 17 instance `power-fixer-db`

### Security Model

- **TUI/Admin endpoints**: Protected by IAP. Only `fte@warp.dev` group members can access.
- **Agent callbacks**: Public but authenticated via bearer token. Each agent gets a unique callback token when launched. Invalid tokens are rejected at the application layer.

### Environment Variables (Cloud Run)

- `DATABASE_URL` - Cloud SQL connection string (from Secret Manager)
- `WARP_API_KEY` - API key for Warp's management API (from Secret Manager)
- `POWERFIXER_CALLBACK_URL` - Set to `https://powerfixer.warp.dev` for agent prompts
- `RUST_LOG` - Log level (default: `info`)

### Deploying Changes

1. Build and push container: `gcloud builds submit --tag us-east4-docker.pkg.dev/warp-power-fixer/power-fixer/server:latest`
2. Deploy: `gcloud run deploy power-fixer-server --image us-east4-docker.pkg.dev/warp-power-fixer/power-fixer/server:latest --region us-east4 --project warp-power-fixer`

### Terraform Resources

Infrastructure is managed in `warp-terraform/environments/power-fixer/main.tf`:
- Cloud Run service and IAM
- Load balancer (IP, SSL cert, URL map, backends)
- Cloud SQL instance and database
- Secret Manager secrets
- IAP configuration
