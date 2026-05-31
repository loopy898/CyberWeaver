//! In-memory adjacency-graph construction and mutation.
//!
//! Wraps [`AdjacencyGraph`] with methods to add/remove nodes and edges,
//! query successors / predecessors / neighbours, and build a graph from
//! external data (e.g. rows loaded from SQLite).

use std::collections::HashMap;

use crate::graph::types::{AdjacencyGraph, EdgeInfo};

impl AdjacencyGraph {
    // ------------------------------------------------------------------
    // Construction
    // ------------------------------------------------------------------

    /// Create an empty graph.
    pub fn new() -> Self {
        Self {
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
            node_ids: Vec::new(),
        }
    }

    /// Build a graph from a list of node IDs and a list of edges.
    pub fn from_data(node_ids: Vec<String>, edges: Vec<EdgeInfo>) -> Self {
        let mut graph = Self::new();

        for node_id in &node_ids {
            graph.node_ids.push(node_id.clone());
            graph.outgoing.entry(node_id.clone()).or_default();
            graph.incoming.entry(node_id.clone()).or_default();
        }

        for edge in edges {
            graph
                .outgoing
                .entry(edge.source_id.clone())
                .or_default()
                .push(edge.clone());
            graph
                .incoming
                .entry(edge.target_id.clone())
                .or_default()
                .push(edge);
        }

        graph
    }

    // ------------------------------------------------------------------
    // Node mutation
    // ------------------------------------------------------------------

    /// Add a node. No-op if the node already exists.
    pub fn add_node(&mut self, node_id: String) {
        if !self.node_ids.contains(&node_id) {
            self.node_ids.push(node_id.clone());
            self.outgoing.entry(node_id.clone()).or_default();
            self.incoming.entry(node_id).or_default();
        }
    }

    /// Remove a node and all edges incident to it.
    pub fn remove_node(&mut self, node_id: &str) {
        self.node_ids.retain(|id| id != node_id);

        // Remove outgoing edges and clean up target node's incoming list.
        if let Some(out_edges) = self.outgoing.remove(node_id) {
            for edge in &out_edges {
                if let Some(incoming_list) = self.incoming.get_mut(&edge.target_id) {
                    incoming_list.retain(|e| e.source_id != node_id);
                }
            }
        }

        // Remove incoming edges and clean up source node's outgoing list.
        if let Some(in_edges) = self.incoming.remove(node_id) {
            for edge in &in_edges {
                if let Some(outgoing_list) = self.outgoing.get_mut(&edge.source_id) {
                    outgoing_list.retain(|e| e.target_id != node_id);
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Edge mutation
    // ------------------------------------------------------------------

    /// Add an edge. Both endpoints must already exist in the graph.
    pub fn add_edge(&mut self, edge: EdgeInfo) {
        self.outgoing
            .entry(edge.source_id.clone())
            .or_default()
            .push(edge.clone());
        self.incoming
            .entry(edge.target_id.clone())
            .or_default()
            .push(edge);
    }

    /// Remove the edge identified by `(relation_id, source_id, target_id)`.
    pub fn remove_edge(&mut self, relation_id: &str, source_id: &str, target_id: &str) {
        if let Some(out_list) = self.outgoing.get_mut(source_id) {
            out_list.retain(|e| e.relation_id != relation_id);
        }
        if let Some(in_list) = self.incoming.get_mut(target_id) {
            in_list.retain(|e| e.relation_id != relation_id);
        }
    }

    // ------------------------------------------------------------------
    // Query helpers
    // ------------------------------------------------------------------

    /// Direct successors reachable from `node_id`. Optionally filtered by
    /// `relation_type`.
    pub fn successors(&self, node_id: &str, relation_type: Option<&str>) -> Vec<&EdgeInfo> {
        self.outgoing
            .get(node_id)
            .map(|edges| {
                edges
                    .iter()
                    .filter(|e| relation_type.map_or(true, |rt| e.relation_type == rt))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Direct predecessors that point to `node_id`. Optionally filtered by
    /// `relation_type`.
    pub fn predecessors(&self, node_id: &str, relation_type: Option<&str>) -> Vec<&EdgeInfo> {
        self.incoming
            .get(node_id)
            .map(|edges| {
                edges
                    .iter()
                    .filter(|e| relation_type.map_or(true, |rt| e.relation_type == rt))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// All neighbouring node IDs (successors + predecessors), deduplicated.
    pub fn neighbors(&self, node_id: &str) -> Vec<String> {
        let mut neighbor_ids: Vec<String> = Vec::new();

        if let Some(out_edges) = self.outgoing.get(node_id) {
            for e in out_edges {
                if !neighbor_ids.contains(&e.target_id) {
                    neighbor_ids.push(e.target_id.clone());
                }
            }
        }
        if let Some(in_edges) = self.incoming.get(node_id) {
            for e in in_edges {
                if !neighbor_ids.contains(&e.source_id) {
                    neighbor_ids.push(e.source_id.clone());
                }
            }
        }

        neighbor_ids
    }

    /// Total number of nodes.
    pub fn node_count(&self) -> usize {
        self.node_ids.len()
    }

    /// Total number of edges.
    pub fn edge_count(&self) -> usize {
        self.outgoing.values().map(|v| v.len()).sum()
    }
}
