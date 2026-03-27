# WebSocket Graph Push Design

## Goal

Add a minimal WebSocket channel on top of the existing Axum sidecar so the Rust backend can actively push graph node updates to the React/Tldraw frontend. This iteration only targets the first usable loop: the frontend sends a debug command, the backend responds by pushing a sample node, and the frontend renders that node onto the canvas.

## Why This Slice

The project plan separates the roadmap into:

- Phase 2: stabilize persistence and local canvas sync
- Phase 3: add Axum WebSocket real-time communication and backend-driven canvas operations

Phase 2 now has a workable persistence path. The next highest-leverage step is Phase 3's transport layer, because later backend-driven drawing, tool execution, and AI-assisted graph updates all depend on a reliable push channel.

## Scope

This design includes:

- An Axum `/ws` endpoint
- A shared Rust broadcast channel for backend-to-frontend events
- A minimal JSON command from frontend to backend: `debug_create_node`
- A minimal JSON event from backend to frontend: `graph_update`
- Frontend WebSocket connection lifecycle and reconnect behavior
- Frontend application of pushed node updates to Tldraw

This design explicitly excludes:

- Authentication
- Multi-room or multi-session isolation
- Edge synchronization
- Persisting graph changes through the WebSocket path itself
- AI agent orchestration
- Generic tool execution framework

## Architecture

### Backend

The existing Axum sidecar remains the network surface. A shared `broadcast::Sender<String>` is added to application state and cloned into each WebSocket connection handler.

The `/ws` handler upgrades the connection and splits it into:

- a reader loop for frontend commands
- a writer loop subscribed to the broadcast channel for outbound events

When the backend receives:

```json
{ "type": "debug_create_node" }
```

it creates a sample graph event and sends:

```json
{
  "type": "graph_update",
  "delta": {
    "added_nodes": [
      {
        "id": "ws-demo-1",
        "node_type": "note",
        "x": 320,
        "y": 180,
        "content": "来自 Rust WebSocket 的测试节点"
      }
    ],
    "updated_nodes": []
  }
}
```

through the broadcast channel.

### Frontend

The React app opens a WebSocket connection to `ws://127.0.0.1:3000/ws` after Tldraw mounts. The socket listener parses JSON and only handles `graph_update` events. Each pushed node is converted into a Tldraw shape using the same node-type restrictions already used by persistence:

- `geo`
- `text`
- `note`

Unknown node types are ignored.

The pushed shapes are created or updated on the canvas. The existing local persistence listener then writes those canvas changes into SQLite through Tauri commands. This keeps responsibilities separate:

- WebSocket: transport and backend push
- Tldraw: in-memory canvas state
- Tauri commands: local persistence

## Data Flow

1. Frontend mounts Tldraw
2. Frontend opens WebSocket connection
3. User clicks a debug trigger in the UI
4. Frontend sends `debug_create_node`
5. Backend receives command and emits `graph_update`
6. Frontend receives `graph_update`
7. Frontend maps pushed nodes into Tldraw shapes
8. Existing persistence logic captures the shape change and saves it into SQLite

## Error Handling

- Frontend reconnects after disconnect with a short delay
- Invalid JSON messages are ignored with logging
- Unknown event types are ignored
- Unknown pushed node types are ignored
- A single broken client must not stop the Axum service or other subscribers

## Testing Strategy

The implementation should favor testable pure helpers:

- Rust: protocol message serialization and command-to-event mapping
- Frontend: event parsing and node-to-shape mapping helpers

Runtime verification should confirm:

- the frontend builds successfully
- the backend still compiles as far as the local environment allows
- clicking the debug action results in a server-pushed node appearing on the canvas
- after reload, the node can still be loaded from SQLite

## Acceptance Criteria

- Frontend successfully connects to `/ws`
- Frontend can send `debug_create_node`
- Backend responds by pushing `graph_update`
- Frontend renders the pushed node without triggering validation errors
- The pushed node is persisted through the existing SQLite sync path
