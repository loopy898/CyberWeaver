//! Tauri command handlers — the IPC boundary between frontend and backend.
//!
//! Each sub-module groups related Tauri commands. Commands are thin wrappers
//! that deserialize inputs, delegate to the service layer, and serialize
//! results (or errors) back to the frontend.

pub mod export_cmd;
pub mod import_cmd;
pub mod llm_cmd;
pub mod node_cmd;
pub mod relation_cmd;
pub mod traversal_cmd;
