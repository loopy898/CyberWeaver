//! WebSocket layer — real-time communication with the frontend.
//!
//! Handles WebSocket upgrade, message framing, and broadcasting graph deltas
//! to all connected clients so the tldraw canvas stays in sync.

pub mod handler;
pub mod protocol;
pub mod relay;
