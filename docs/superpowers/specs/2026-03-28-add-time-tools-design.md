# CyberWeaver Timestamp And Reverse Geocode Tools Design

## Goal

Add two new automation tools to the existing CyberWeaver plugin system:

1. `timestamp_convert`
2. `reverse_geocode`

Both tools should be invokable through the current WebSocket `tool_execution` protocol, produce user-visible results inside the app, and be covered by automated tests.

## Existing Constraints

- The backend already supports generic Python tool plugins discovered from `plugins/`.
- Plugin discovery is text-based and requires `@register_tool("tool_name")`.
- Rust only consumes these plugin output fields:
  - `message`
  - `added_nodes`
  - `added_edges`
  - `tokens`
- The frontend already handles generic `tool_result`, `graph_update`, and `agent_token` events, but the toolbar is still hard-coded for `scan_port`.

## Approach

### Tool Integration

Keep the existing generic plugin execution pipeline unchanged. Implement both tools as Python plugins under `plugins/`, and surface them in the existing frontend with minimal new controls.

This keeps the core architecture stable:

- WebSocket protocol unchanged
- Rust plugin bridge unchanged for runtime behavior
- Frontend reuses `createToolExecutionCommand()`

### Tool 1: `timestamp_convert`

Input:

- `value`: string or number supplied by the user

Behavior:

- Accept raw timestamp-like input
- Try multiple interpretations:
  - Unix seconds
  - Unix milliseconds
  - Unix microseconds
  - Unix nanoseconds
  - Apple Cocoa seconds since `2001-01-01T00:00:00Z`
- For each valid interpretation, output:
  - epoch label
  - normalized UTC datetime
  - RFC3339 string
  - local time string
  - numeric timestamp value
- If the input cannot be interpreted, return a failure-style message and explanatory tokens

Presentation:

- `message` contains a concise summary
- `tokens` list each parsed interpretation
- `added_nodes` contains a note node with the full conversion report

### Tool 2: `reverse_geocode`

Input:

- `latitude`
- `longitude`

Behavior:

- Validate range:
  - latitude in `[-90, 90]`
  - longitude in `[-180, 180]`
- Query a reverse-geocoding provider over HTTP
- Use provider response to format:
  - display name
  - country
  - state / province
  - city / county / district when available
- Fail gracefully on timeout, empty results, and provider errors

Presentation:

- `message` contains a concise address summary
- `tokens` list structured location details
- `added_nodes` contains a note node with the full location report

### Frontend UX

Add a compact tool panel to the existing toolbar area:

- timestamp input field + execute button
- latitude input field
- longitude input field
- reverse geocode execute button

The UI should stay visually aligned with the current control bar and reuse the existing status/timeline behavior.

### Error Handling

- Invalid timestamp input returns a clear message rather than crashing
- Invalid coordinate input returns a clear message rather than attempting network access
- Reverse-geocode network failures return a clear message and no graph mutation unless a usable result exists

## Testing Strategy

### Python

Add unit tests for plugin logic:

- `plugins/tests/test_timestamp_convert.py`
- `plugins/tests/test_reverse_geocode.py`

Coverage:

- seconds / milliseconds / microseconds / nanoseconds / Cocoa interpretation
- invalid timestamp input
- negative timestamps
- valid coordinate lookup
- out-of-range coordinates
- network timeout / provider error

Reverse-geocode tests must mock network calls and avoid real external dependencies.

### Rust

Extend the existing `src-tauri/src/main.rs` test module to verify:

- plugin registration parsing recognizes the new tool names
- plugin execution emits `tool_result`
- successful executions emit `graph_update` when nodes are added

### TypeScript

Extend `src/ws.test.ts` to validate command creation for the new tool names and parameters if helper coverage is needed.

### Verification Commands

- `python -m unittest discover -s .\\plugins\\tests -p "test_*.py"`
- `node --test src/ws.test.ts`
- `npm run build`
- `cargo test`
- `cargo check`

## Tradeoffs

### Why not extend the protocol with structured `data`?

That would improve frontend rendering flexibility, but it increases surface area across Python, Rust, and TypeScript. For this feature, text-first results embedded in `message`, `tokens`, and note nodes keep the change focused and lower-risk.

### Why not implement reverse geocoding in Rust?

The current architecture already uses Python tools as execution units. Keeping the new network call inside the plugin avoids unnecessary Rust dependency growth and keeps the implementation aligned with the existing system.
