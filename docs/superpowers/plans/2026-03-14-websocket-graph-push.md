# WebSocket Graph Push Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a minimal Axum WebSocket path that lets the Rust backend push a sample graph node to the Tldraw frontend after a frontend-issued debug command.

**Architecture:** Keep the existing Tauri persistence path intact and layer WebSocket transport beside it. Rust owns the `/ws` endpoint and message fan-out, while React owns message parsing and canvas application; pushed nodes then flow into SQLite through the already-existing local save listener.

**Tech Stack:** Axum WebSocket, Tokio broadcast, Tauri, React, TypeScript, Tldraw

---

## Chunk 1: Protocol And Backend Transport

### Task 1: Add protocol types and failing backend tests

**Files:**
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Write a failing Rust test for `debug_create_node` event generation**

```rust
#[test]
fn debug_create_node_command_emits_graph_update_message() {
    let message = handle_ws_command(r#"{"type":"debug_create_node"}"#).unwrap();
    assert!(message.contains(r#""type":"graph_update""#));
    assert!(message.contains(r#""node_type":"note""#));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test debug_create_node_command_emits_graph_update_message -- --exact`
Expected: FAIL because `handle_ws_command` does not exist yet

- [ ] **Step 3: Implement minimal protocol parsing and event generation**

Add:

```rust
#[derive(Deserialize)]
#[serde(tag = "type")]
enum WsCommand {
    #[serde(rename = "debug_create_node")]
    DebugCreateNode,
}

fn handle_ws_command(raw: &str) -> Option<String> {
    // parse command and serialize graph_update event
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test debug_create_node_command_emits_graph_update_message -- --exact`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/main.rs
git commit -m "feat: add websocket graph update protocol"
```

### Task 2: Add `/ws` endpoint and broadcast plumbing

**Files:**
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Write a failing Rust test for unknown command handling**

```rust
#[test]
fn unknown_ws_command_is_rejected() {
    assert!(handle_ws_command(r#"{"type":"unknown"}"#).is_none());
}
```

- [ ] **Step 2: Run test to verify it fails for the expected reason**

Run: `cargo test unknown_ws_command_is_rejected -- --exact`
Expected: FAIL until `handle_ws_command` rejects unknown commands

- [ ] **Step 3: Implement Axum WebSocket state and handlers**

Add:

- app state with `broadcast::Sender<String>`
- `/ws` route
- connection reader for frontend messages
- connection writer subscribed to broadcast events

- [ ] **Step 4: Run targeted tests**

Run: `cargo test -- --nocapture`
Expected: PASS for protocol tests

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/main.rs
git commit -m "feat: add axum websocket broadcast endpoint"
```

## Chunk 2: Frontend WebSocket Client

### Task 3: Extract frontend message helpers with a failing test

**Files:**
- Create: `src/ws.ts`
- Create: `src/ws.test.mjs`

- [ ] **Step 1: Write a failing Node test for graph update parsing**

```javascript
import test from 'node:test'
import assert from 'node:assert/strict'
import { parseGraphUpdateMessage } from './ws.js'

test('parses added note node from graph_update message', () => {
  const result = parseGraphUpdateMessage(JSON.stringify({
    type: 'graph_update',
    delta: { added_nodes: [{ id: '1', node_type: 'note', x: 1, y: 2, content: 'x' }], updated_nodes: [] }
  }))
  assert.equal(result.addedNodes.length, 1)
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test src/ws.test.mjs`
Expected: FAIL because helper module does not exist yet

- [ ] **Step 3: Implement pure frontend helpers**

Add parsing helpers that:

- ignore invalid JSON
- accept only `graph_update`
- normalize `node_type`
- map payloads into a frontend-friendly node structure

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test src/ws.test.mjs`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/ws.ts src/ws.test.mjs
git commit -m "test: add websocket graph update parser coverage"
```

### Task 4: Wire the WebSocket client into the app

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/ws.ts`

- [ ] **Step 1: Write the next failing frontend test for unknown node type rejection**

```javascript
test('ignores unsupported pushed node types', () => {
  const result = parseGraphUpdateMessage(JSON.stringify({
    type: 'graph_update',
    delta: { added_nodes: [{ id: '1', node_type: 'arrow', x: 0, y: 0, content: '' }], updated_nodes: [] }
  }))
  assert.equal(result.addedNodes.length, 0)
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test src/ws.test.mjs`
Expected: FAIL until filtering is implemented

- [ ] **Step 3: Implement the WebSocket lifecycle in `App.tsx`**

Add:

- socket connect after editor mount
- reconnect on close
- debug button that sends `debug_create_node`
- graph update handler that creates or updates Tldraw shapes

- [ ] **Step 4: Run frontend verification**

Run: `node --test src/ws.test.mjs`
Expected: PASS

Run: `npm run build`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/App.tsx src/ws.ts src/ws.test.mjs
git commit -m "feat: render websocket-pushed graph nodes"
```

## Chunk 3: Final Verification

### Task 5: Run end-to-end verification commands

**Files:**
- Modify: `README.md` (only if startup or debug instructions need updating)

- [ ] **Step 1: Verify frontend build**

Run: `npm run build`
Expected: PASS

- [ ] **Step 2: Verify backend compile path**

Run: `cargo check`
Expected: PASS, or document the exact local environment blocker if system policy prevents Rust build scripts

- [ ] **Step 3: Verify working tree state**

Run: `git status -sb`
Expected: only intended files changed

- [ ] **Step 4: Commit any doc or instruction update**

```bash
git add README.md
git commit -m "docs: document websocket graph push debug flow"
```
