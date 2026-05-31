use std::collections::HashMap;

/// Adjacency-list representation of a directed graph.
#[derive(Debug, Clone, Default)]
pub struct AdjacencyGraph {
    /// node ID -> list of outgoing edges
    pub outgoing: HashMap<String, Vec<EdgeInfo>>,
    /// node ID -> list of incoming edges
    pub incoming: HashMap<String, Vec<EdgeInfo>>,
    /// All node IDs present in the graph
    pub node_ids: Vec<String>,
}

/// Lightweight edge/relationship representation for traversal.
#[derive(Debug, Clone)]
pub struct EdgeInfo {
    pub relation_id: String,
    pub source_id: String,
    pub target_id: String,
    pub relation_type: String,
    pub label: String,
}

/// A single traversal path through the graph.
#[derive(Debug, Clone)]
pub struct TraversalPath {
    /// Ordered sequence of node IDs visited along the path
    pub node_ids: Vec<String>,
    /// Relation IDs connecting consecutive node pairs (len = node_ids.len() - 1)
    pub relation_ids: Vec<String>,
    /// Relation types corresponding to each hop
    pub relation_types: Vec<String>,
}

/// Result of a path-finding query.
#[derive(Debug, Clone)]
pub struct PathResult {
    /// All paths discovered
    pub paths: Vec<TraversalPath>,
    /// Number of hops (edges) for each path
    pub total_hops: Vec<usize>,
}
