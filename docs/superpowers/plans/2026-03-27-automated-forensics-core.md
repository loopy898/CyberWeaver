# Automated Forensics Core Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement an end-to-end local-first automated forensics MVP across backend graph core, plugin execution, realtime protocol, and frontend visualization/interaction.

**Architecture:** Extend the current Tauri + Axum sidecar with a typed graph/runtime state manager and plugin execution bridge. Keep persistence and event broadcasting explicit, then consume typed graph events on the React/Tldraw side with ELK layout and zoom LOD rendering.

**Tech Stack:** Rust (Axum, Tokio, SeaORM/sqlite), Python (plugin SDK via subprocess bridge), React/TypeScript, Tldraw, ELK.js

---

## Chunk 1: Backend Graph Core + Protocol

### Task 1: Introduce graph schema and in-memory manager

**Files:**
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Write failing backend tests for graph manager CRUD behavior**
- [ ] **Step 2: Run `cargo test graph_manager -- --nocapture` and confirm red**
- [ ] **Step 3: Implement `GraphManager` for nodes/edges/components**
- [ ] **Step 4: Re-run targeted test to green**
- [ ] **Step 5: Commit**

### Task 2: Extend SQLite schema and REST API

**Files:**
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Write failing tests for REST payload validation and graph snapshot serialization**
- [ ] **Step 2: Run targeted tests and confirm red**
- [ ] **Step 3: Add/upgrade DB tables (`nodes`, `edges`, `components`) + CRUD handlers**
- [ ] **Step 4: Verify tests pass**
- [ ] **Step 5: Commit**

### Task 3: Expand WebSocket command/event protocol

**Files:**
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Add failing tests for `tool_execution` command parsing**
- [ ] **Step 2: Run targeted tests and confirm red**
- [ ] **Step 3: Implement protocol parser + `graph_update`/`tool_result`/`agent_token` emitters**
- [ ] **Step 4: Run `cargo test`**
- [ ] **Step 5: Commit**

## Chunk 2: Plugin Bridge + Agent Runtime Slice

### Task 4: Add Python plugin SDK and loader

**Files:**
- Create: `plugins/sdk.py`
- Create: `plugins/scan_port.py`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Write failing Rust tests for tool registry discovery**
- [ ] **Step 2: Run targeted tests and confirm red**
- [ ] **Step 3: Implement plugin bootstrap and execution bridge**
- [ ] **Step 4: Re-run tests to green**
- [ ] **Step 5: Commit**

### Task 5: Implement observer and token stream plumbing

**Files:**
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Add failing tests for graph context projection + token event shape**
- [ ] **Step 2: Run targeted tests and confirm red**
- [ ] **Step 3: Implement observer context and simulated token streaming**
- [ ] **Step 4: Run `cargo test`**
- [ ] **Step 5: Commit**

## Chunk 3: Frontend Visualization + Interaction

### Task 6: Add frontend graph protocol helpers and layout pipeline

**Files:**
- Modify: `src/ws.ts`
- Modify: `src/ws.test.ts`
- Create: `src/layout.ts`

- [ ] **Step 1: Add failing parser tests for new event types**
- [ ] **Step 2: Run `node --test src/ws.test.ts` and confirm red**
- [ ] **Step 3: Implement parsing and graph-to-layout transform helpers**
- [ ] **Step 4: Re-run tests to green**
- [ ] **Step 5: Commit**

### Task 7: Wire LOD rendering + ScanPort interaction

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/canvasNodes.ts`

- [ ] **Step 1: Add failing tests for command generator / event handling paths**
- [ ] **Step 2: Run tests and confirm red**
- [ ] **Step 3: Implement zoom-aware rendering, context action dispatch, thought bubble updates**
- [ ] **Step 4: Run `node --test src/ws.test.ts` and `npm run build`**
- [ ] **Step 5: Commit**

## Chunk 4: Verification + Docs

### Task 8: End-to-end verification and docs refresh

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Run backend verification `cargo test` + `cargo check`**
- [ ] **Step 2: Run frontend verification `node --test src/ws.test.ts` + `npm run build`**
- [ ] **Step 3: Update README with new protocol + plugin usage**
- [ ] **Step 4: Verify git status contains only expected files**
- [ ] **Step 5: Commit**

