# WebSocket Server

This directory handles real-time updates to connected TUI clients via WebSocket.

## Files

### `handler.rs`
WebSocket connection handling.
- `ws_handler()` - Handles WebSocket upgrade requests at `GET /ws`
- `handle_socket()` - Manages individual WebSocket connections
- Subscribes to broadcast channel and forwards messages to clients
- Handles incoming pings and connection lifecycle

### `types.rs`
WebSocket message type definitions.
- `WsMessage` - Tagged enum of all message types:
  - `AgentUpdate` - Agent created or updated
  - `AgentDeleted` - Agent removed
  - `TriageRunUpdate` - Triage run created or updated
  - `InboxStateUpdate` - Inbox state changed
  - `Ping` / `Pong` - Keep-alive messages

### `broadcast.rs`
Helper functions to broadcast state changes.
- `broadcast_agent_update()` - Broadcasts agent create/update
- `broadcast_agent_deleted()` - Broadcasts agent deletion
- `broadcast_triage_run_update()` - Broadcasts triage run changes
- `broadcast_inbox_state_update()` - Broadcasts inbox state changes

## Architecture

```
                    ┌─────────────────────┐
                    │  broadcast::Sender  │◄── State mutations call
                    │    (100 msg buffer) │    broadcast_* helpers
                    └──────────┬──────────┘
                               │
           ┌───────────────────┼───────────────────┐
           │                   │                   │
           ▼                   ▼                   ▼
    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
    │  WS Client 1 │    │  WS Client 2 │    │  WS Client N │
    │   (TUI)      │    │   (TUI)      │    │   (TUI)      │
    └──────────────┘    └──────────────┘    └──────────────┘
```

## Usage

Every database mutation that changes visible state should call the appropriate broadcast function to push updates to connected TUI clients in real-time.
