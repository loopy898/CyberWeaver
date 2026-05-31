//! Repository traits and implementations.
//!
//! Repositories abstract the database access behind trait interfaces so that
//! service layers are decoupled from the ORM. Each repository corresponds to
//! one aggregate root (e.g. `NodeRepository` for nodes).

pub mod node_repo;
pub mod relation_repo;

pub use node_repo::{CreateNodeData, NodeRepo, UpdateNodeData};
pub use relation_repo::{CreateRelationData, RelationRepo};
