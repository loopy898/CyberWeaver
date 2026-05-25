//! Graph engine — in-memory graph representation and algorithms.
//!
//! This module provides a lightweight in-memory graph structure (adjacency
//! list) used for traversal queries, pattern matching, and sub-graph
//! extraction without requiring a dedicated graph database.

pub mod engine;
pub mod traversal;
pub mod types;
