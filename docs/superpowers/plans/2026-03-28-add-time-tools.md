# Add Timestamp And Reverse Geocode Tools Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add timestamp conversion and reverse geocoding tools to the existing CyberWeaver plugin system, expose them in the app UI, and verify them with Python, Rust, and frontend checks.

**Architecture:** Keep the existing generic WebSocket and Rust plugin bridge intact. Implement the new tool behavior as Python plugins, add minimal React toolbar controls to invoke them, and extend test coverage around plugin behavior and execution.

**Tech Stack:** React, TypeScript, Tauri, Rust, PyO3, Python unittest

---

## Chunk 1: Python Plugin TDD

### Task 1: Add failing tests for timestamp conversion

**Files:**
- Create: `plugins/tests/test_timestamp_convert.py`
- Create: `plugins/tests/__init__.py`
- Test: `plugins/tests/test_timestamp_convert.py`

- [ ] **Step 1: Write failing timestamp tests**
- [ ] **Step 2: Run `python -m unittest discover -s .\\plugins\\tests -p "test_*.py"` and verify the new tests fail for the expected reason**
- [ ] **Step 3: Implement `plugins/timestamp_convert.py` minimally**
- [ ] **Step 4: Re-run the Python tests until they pass**

### Task 2: Add failing tests for reverse geocoding

**Files:**
- Create: `plugins/tests/test_reverse_geocode.py`
- Create: `plugins/reverse_geocode.py`
- Test: `plugins/tests/test_reverse_geocode.py`

- [ ] **Step 1: Write failing reverse-geocode tests with mocked network calls**
- [ ] **Step 2: Run `python -m unittest discover -s .\\plugins\\tests -p "test_*.py"` and verify the new tests fail for the expected reason**
- [ ] **Step 3: Implement `plugins/reverse_geocode.py` minimally**
- [ ] **Step 4: Re-run the Python tests until they pass**

## Chunk 2: Frontend Invocation

### Task 3: Add UI controls for the two tools

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/App.css`

- [ ] **Step 1: Add minimal React state for timestamp and coordinate inputs**
- [ ] **Step 2: Add toolbar controls and send generic `tool_execution` messages**
- [ ] **Step 3: Keep existing `scan_port` behavior unchanged**
- [ ] **Step 4: Run `npm run build` and fix any TypeScript or rendering issues**

## Chunk 3: Rust Integration Tests

### Task 4: Extend backend tests for plugin discovery and execution

**Files:**
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Add failing Rust tests for new tool registration and execution**
- [ ] **Step 2: Run `cargo test` and verify the new tests fail for the expected reason**
- [ ] **Step 3: Adjust shared test helpers only as needed**
- [ ] **Step 4: Re-run `cargo test` until the new tests pass**

## Chunk 4: Final Verification

### Task 5: Run full verification suite

**Files:**
- No code changes expected

- [ ] **Step 1: Run `python -m unittest discover -s .\\plugins\\tests -p "test_*.py"`**
- [ ] **Step 2: Run `node --test src/ws.test.ts`**
- [ ] **Step 3: Run `npm run build`**
- [ ] **Step 4: Run `cargo test` in `src-tauri`**
- [ ] **Step 5: Run `cargo check` in `src-tauri`**
- [ ] **Step 6: Review git diff and prepare commit**
