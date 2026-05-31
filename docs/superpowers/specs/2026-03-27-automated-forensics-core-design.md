# Automated Forensics Core Design

## Goal

Complete the project plan's end-to-end MVP loop for CyberWeaver:

- graph-centric core data model with local durability
- backend-driven realtime updates over WebSocket
- frontend graph visualization pipeline with auto layout and LOD rendering
- command bridge for automated actions (`scan_port`)
- Python plugin runtime and minimal agent token streaming

This design targets a production-quality local-first baseline, not a demo-only spike.

## Scope

Included:

- Rust backend graph manager, SQLite schema (`nodes`, `edges`, `components`)
- REST API (`POST /node`, `POST /edge`, `GET /graph`)
- WebSocket command/event protocol extensions
- Python plugin SDK + plugin discovery and execution bridge
- Frontend graph rendering from backend graph delta events
- ELK layout pipeline and zoom-based LOD rendering
- Right-click `ScanPort` command dispatch
- Agent token stream rendering as canvas thought bubble

Excluded for this iteration:

- multi-user realtime collaboration conflict resolution
- true Nmap integration (use safe simulated scanner plugin)
- full LangChain/OpenAI dependency integration
- enterprise auth and remote deployment topology

## Architecture

### Backend (Rust + Axum + Tauri)

`RuntimeState` owns:

- `GraphManager` (in-memory hot graph cache)
- SQLite connection (WAL mode)
- plugin runtime (`PyO3` bridge)
- WebSocket broadcast channel

Mutation path:

1. command/REST/WS tool command arrives
2. validate and apply to `GraphManager`
3. persist to SQLite (transactional upsert/delete)
4. broadcast `graph_update` event

Tool path:

1. frontend sends `tool_execution`
2. backend executes registered Python tool
3. backend converts tool output into graph deltas + token events
4. backend persists and broadcasts updates

### Data Model (ECS-like projection)

- `Node`: topology + position + created timestamp
- `Component`: payload carrier (`fast_payload`, `custom_payload`)
- `Edge`: relation between node ids

Canvas-facing fields (`node_type`, `content`) are projection data stored through a `display_meta` component for compatibility with current frontend nodes.

### Frontend (React + Tldraw + ELK)

- Maintain local `graphState` map from backend snapshot/delta
- Recompute coordinates with ELK for each graph render cycle
- Render backend-managed shapes with a dedicated prefix (`shape:viz:`) so they do not pollute persistence listeners
- LOD:
  - zoom `< 0.5`: compact circle markers
  - zoom `>= 0.5`: note cards
- Right-click action sends JSON command:
  - `{"type":"tool_execution","tool":"scan_port","params":{"target_id":"..."}}`
- `agent_token` events update a thought-bubble canvas node

## Protocol

Commands:

- `debug_create_node`
- `tool_execution`

Events:

- `graph_update`
- `tool_result`
- `agent_token`

All messages are typed JSON envelopes to keep parsing deterministic and testable.

## Error Handling

- reject unknown command type without crashing socket
- plugin execution failure emits `tool_result` with `ok=false`
- invalid graph payload fragments are ignored (partial tolerance)
- websocket reconnect on frontend with bounded retry delay

## Testing

Backend:

- unit tests for graph mutation and command parsing
- `cargo test`
- `cargo check`

Frontend:

- parser tests for ws payload handling
- build/typecheck via `npm run build`

Manual flow:

1. open app
2. receive baseline graph
3. right-click node and trigger `ScanPort`
4. observe new node/edge + token bubble
5. reload app and verify persisted graph is restored

