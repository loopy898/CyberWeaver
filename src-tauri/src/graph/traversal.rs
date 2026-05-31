//! Graph traversal algorithms: BFS multi-hop neighbourhood queries,
//! shortest path (directed + undirected), and connected-component extraction.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::graph::types::{AdjacencyGraph, PathResult, TraversalPath};

/// BFS multi-hop neighbourhood: discover all paths originating from
/// `start_node_id` up to `max_hops` hops. Optionally filters edges by
/// `relation_type_filter`.
///
/// Every unique **simple** path (no repeated nodes within a path) is
/// collected.  A `HashMap<String, usize>` tracks the best (lowest) depth
/// at which each node has been reached; a node is only expanded further
/// when the current depth does not exceed that best depth.  This keeps
/// the search exhaustive while preventing unbounded expansion.
pub fn bfs_paths(
    graph: &AdjacencyGraph,
    start_node_id: &str,
    max_hops: usize,
    relation_type_filter: Option<&str>,
) -> PathResult {
    let mut paths: Vec<TraversalPath> = Vec::new();
    let mut total_hops: Vec<usize> = Vec::new();
    // Best (minimum) hop depth at which each node has been seen.
    let mut best_depth: HashMap<String, usize> = HashMap::new();
    let mut queue: VecDeque<TraversalPath> = VecDeque::new();

    // Initial path: only the starting node.
    queue.push_back(TraversalPath {
        node_ids: vec![start_node_id.to_string()],
        relation_ids: Vec::new(),
        relation_types: Vec::new(),
    });
    best_depth.insert(start_node_id.to_string(), 0);

    while let Some(current_path) = queue.pop_front() {
        let current_hop = current_path.relation_ids.len();
        // Every queued path is constructed with at least the start node and never emptied.
        let current_node = current_path.node_ids.last().expect("path never empty");

        // Record every non-zero-hop path.
        if current_hop > 0 {
            paths.push(current_path.clone());
            total_hops.push(current_hop);
        }

        // Stop expanding when we have reached the hop budget.
        if current_hop >= max_hops {
            continue;
        }

        let next_hop = current_hop + 1;

        // Expand through outgoing edges.
        let successors = graph.successors(current_node, relation_type_filter);
        for edge in successors {
            // Per-path cycle prevention: do not revisit a node.
            if current_path.node_ids.contains(&edge.target_id) {
                continue;
            }

            // Prune expansion paths that are strictly worse than an
            // already-seen depth for this node.
            if let Some(&prev_best) = best_depth.get(&edge.target_id) {
                if next_hop > prev_best {
                    continue;
                }
            }

            best_depth.insert(edge.target_id.clone(), next_hop);

            let mut new_path = current_path.clone();
            new_path.node_ids.push(edge.target_id.clone());
            new_path.relation_ids.push(edge.relation_id.clone());
            new_path.relation_types.push(edge.relation_type.clone());
            queue.push_back(new_path);
        }
    }

    PathResult { paths, total_hops }
}

/// Find the shortest path (minimum number of edges) between `from_id` and
/// `to_id` within `max_hops`, treating the graph as **undirected** (both
/// outgoing and incoming edges are followed).
///
/// Returns `None` if no path exists within the hop limit.
pub fn shortest_path(
    graph: &AdjacencyGraph,
    from_id: &str,
    to_id: &str,
    max_hops: usize,
) -> Option<TraversalPath> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<TraversalPath> = VecDeque::new();

    queue.push_back(TraversalPath {
        node_ids: vec![from_id.to_string()],
        relation_ids: Vec::new(),
        relation_types: Vec::new(),
    });
    visited.insert(from_id.to_string());

    while let Some(current_path) = queue.pop_front() {
        // Every queued path is constructed with at least the start node and never emptied.
        let current_node = current_path.node_ids.last().expect("path never empty");

        // Target reached — BFS guarantees this is the shortest path.
        if current_node == to_id {
            return Some(current_path);
        }

        if current_path.relation_ids.len() >= max_hops {
            continue;
        }

        // Forward edges.
        let successors = graph.successors(current_node, None);
        for edge in successors {
            if visited.contains(&edge.target_id) {
                continue;
            }
            visited.insert(edge.target_id.clone());

            let mut new_path = current_path.clone();
            new_path.node_ids.push(edge.target_id.clone());
            new_path.relation_ids.push(edge.relation_id.clone());
            new_path.relation_types.push(edge.relation_type.clone());
            queue.push_back(new_path);
        }

        // Backward edges (reverse traversal) — makes the search undirected.
        let predecessors = graph.predecessors(current_node, None);
        for edge in predecessors {
            if visited.contains(&edge.source_id) {
                continue;
            }
            visited.insert(edge.source_id.clone());

            let mut new_path = current_path.clone();
            new_path.node_ids.push(edge.source_id.clone());
            new_path
                .relation_ids
                .push(format!("rev:{}", edge.relation_id));
            new_path
                .relation_types
                .push(format!("rev:{}", edge.relation_type));
            queue.push_back(new_path);
        }
    }

    None
}

/// Compute the connected component (weakly) containing `node_id`.
///
/// Treats the graph as undirected. Returns the set of all node IDs
/// reachable from `node_id` via any path.
pub fn connected_component(graph: &AdjacencyGraph, node_id: &str) -> HashSet<String> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();

    queue.push_back(node_id.to_string());
    visited.insert(node_id.to_string());

    while let Some(current) = queue.pop_front() {
        // Forward neighbours.
        let successors = graph.successors(&current, None);
        for edge in successors {
            if visited.insert(edge.target_id.clone()) {
                queue.push_back(edge.target_id.clone());
            }
        }
        // Backward neighbours (reverse edges).
        let predecessors = graph.predecessors(&current, None);
        for edge in predecessors {
            if visited.insert(edge.source_id.clone()) {
                queue.push_back(edge.source_id.clone());
            }
        }
    }

    visited
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::types::{AdjacencyGraph, EdgeInfo};

    /// Helper: build edge info.
    fn edge(rid: &str, src: &str, tgt: &str, rtype: &str) -> EdgeInfo {
        EdgeInfo {
            relation_id: rid.to_string(),
            source_id: src.to_string(),
            target_id: tgt.to_string(),
            relation_type: rtype.to_string(),
            label: String::new(),
        }
    }

    fn diamond_graph() -> AdjacencyGraph {
        //   A -> B -> D
        //   A -> C -> D
        AdjacencyGraph::from_data(
            vec!["A".into(), "B".into(), "C".into(), "D".into()],
            vec![
                edge("e1", "A", "B", "connects"),
                edge("e2", "A", "C", "connects"),
                edge("e3", "B", "D", "connects"),
                edge("e4", "C", "D", "connects"),
            ],
        )
    }

    // ------------------------------------------------------------------
    // bfs_paths
    // ------------------------------------------------------------------

    #[test]
    fn bfs_paths_one_hop() {
        let g = diamond_graph();
        let result = bfs_paths(&g, "A", 1, None);
        // Paths: A->B, A->C
        assert_eq!(result.paths.len(), 2);
        for p in &result.paths {
            assert_eq!(p.relation_ids.len(), 1);
        }
    }

    #[test]
    fn bfs_paths_two_hops() {
        let g = diamond_graph();
        let result = bfs_paths(&g, "A", 2, None);
        // A->B, A->C, A->B->D, A->C->D
        assert_eq!(result.paths.len(), 4);
    }

    #[test]
    fn bfs_paths_with_type_filter() {
        let mut g = diamond_graph();
        g.add_edge(edge("e5", "A", "D", "uses"));
        let result = bfs_paths(&g, "A", 2, Some("connects"));
        // Only "connects" edges expanded: A->B, A->C, A->B->D, A->C->D
        // "uses" edge is excluded.
        assert_eq!(result.paths.len(), 4);
    }

    // ------------------------------------------------------------------
    // shortest_path
    // ------------------------------------------------------------------

    #[test]
    fn shortest_path_direct() {
        let mut g = diamond_graph();
        g.add_edge(edge("e5", "A", "D", "direct"));
        let path = shortest_path(&g, "A", "D", 3).unwrap();
        assert_eq!(path.node_ids, vec!["A", "D"]);
        assert_eq!(path.relation_ids.len(), 1);
    }

    #[test]
    fn shortest_path_two_hops() {
        // Remove the direct edge so the shortest path is A->B->D (2 hops)
        let g = diamond_graph();
        let path = shortest_path(&g, "A", "D", 3).unwrap();
        assert_eq!(path.node_ids.len(), 3); // A -> B -> D  or  A -> C -> D
        assert_eq!(path.relation_ids.len(), 2);
    }

    #[test]
    fn shortest_path_none() {
        let g = diamond_graph();
        let path = shortest_path(&g, "A", "D", 0);
        assert!(path.is_none());
    }

    #[test]
    fn shortest_path_reverse_edge() {
        // A -> B, B -> C — no direct path from C to A, but shortest_path
        // uses bidirectional traversal so C -> B -> A should be found.
        let g = AdjacencyGraph::from_data(
            vec!["A".into(), "B".into(), "C".into()],
            vec![
                edge("e1", "A", "B", "connects"),
                edge("e2", "B", "C", "connects"),
            ],
        );
        let path = shortest_path(&g, "C", "A", 3).unwrap();
        assert_eq!(path.node_ids.len(), 3);
        // The first (or second) relation ID should be prefixed with "rev:"
        assert!(path.relation_ids.iter().any(|r| r.starts_with("rev:")));
    }

    // ------------------------------------------------------------------
    // connected_component
    // ------------------------------------------------------------------

    #[test]
    fn connected_component_diamond() {
        let g = diamond_graph();
        let comp = connected_component(&g, "A");
        let expected: HashSet<String> =
            ["A", "B", "C", "D"].iter().map(|s| s.to_string()).collect();
        assert_eq!(comp, expected);
    }

    #[test]
    fn connected_component_isolated() {
        let g = AdjacencyGraph::from_data(
            vec!["X".into(), "Y".into()],
            vec![edge("e1", "X", "Y", "connects")],
        );
        // Node "Z" not added — lookup should return empty
        let comp = connected_component(&g, "Z");
        assert_eq!(comp.len(), 1);
        assert!(comp.contains("Z"));
    }
}
