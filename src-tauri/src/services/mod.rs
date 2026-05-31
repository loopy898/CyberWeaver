//! Business-logic service layer.
//!
//! Services contain the application's core logic. They orchestrate repository
//! calls, enforce business rules, and are consumed by Tauri commands. Services
//! should never depend on Tauri or HTTP types directly.

pub mod graph_svc;
pub mod import;
pub mod llm;
pub mod node_svc;
pub mod relation_svc;
pub mod report;
