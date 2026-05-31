//! Tauri commands for graph traversal and query operations.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::repositories::{NodeRepo, RelationRepo};
use crate::error::AppError;
use crate::graph::traversal;
use crate::graph::types::{AdjacencyGraph, EdgeInfo, TraversalPath};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Transfer structs (serialised to the frontend)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct TraversalPathData {
    pub node_ids: Vec<String>,
    pub relation_ids: Vec<String>,
    pub relation_types: Vec<String>,
}

impl From<TraversalPath> for TraversalPathData {
    fn from(p: TraversalPath) -> Self {
        Self {
            node_ids: p.node_ids,
            relation_ids: p.relation_ids,
            relation_types: p.relation_types,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExpandNodeResult {
    pub paths: Vec<TraversalPathData>,
    pub total_hops: Vec<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphSummary {
    pub node_count: usize,
    pub edge_count: usize,
    pub node_ids: Vec<String>,
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// 展开节点的多跳邻域：从某节点出发，查找 max_hops 跳内所有可达路径
#[tauri::command]
pub async fn expand_node(
    investigation_id: String,
    start_node_id: String,
    max_hops: usize,
    relation_type_filter: Option<String>,
    state: State<'_, AppState>,
) -> Result<ExpandNodeResult, String> {
    let db = &state.db;
    let (graph, _node_map) = load_graph(db, &investigation_id)
        .await
        .map_err(|e| e.to_string())?;

    let filter = relation_type_filter.as_deref();
    let result = traversal::bfs_paths(&graph, &start_node_id, max_hops, filter);

    Ok(ExpandNodeResult {
        paths: result
            .paths
            .into_iter()
            .map(TraversalPathData::from)
            .collect(),
        total_hops: result.total_hops,
    })
}

/// 查找两个节点之间的最短路径（无向遍历）
#[tauri::command]
pub async fn find_path(
    investigation_id: String,
    from_node_id: String,
    to_node_id: String,
    max_hops: usize,
    state: State<'_, AppState>,
) -> Result<Option<TraversalPathData>, String> {
    let db = &state.db;
    let (graph, _node_map) = load_graph(db, &investigation_id)
        .await
        .map_err(|e| e.to_string())?;

    let result = traversal::shortest_path(&graph, &from_node_id, &to_node_id, max_hops);
    Ok(result.map(TraversalPathData::from))
}

/// 获取节点的连通分量（无向）
#[tauri::command]
pub async fn get_component(
    investigation_id: String,
    node_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let db = &state.db;
    let (graph, _node_map) = load_graph(db, &investigation_id)
        .await
        .map_err(|e| e.to_string())?;

    let component = traversal::connected_component(&graph, &node_id);
    Ok(component.into_iter().collect())
}

/// 获取调查案件中所有节点和关系的列表（用于图概览）
#[tauri::command]
pub async fn get_graph_summary(
    investigation_id: String,
    state: State<'_, AppState>,
) -> Result<GraphSummary, String> {
    let db = &state.db;
    let (graph, _node_map) = load_graph(db, &investigation_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(GraphSummary {
        node_count: graph.node_count(),
        edge_count: graph.edge_count(),
        node_ids: graph.node_ids,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// 从 SQLite 加载调查案件的全部节点和关系，构建内存邻接图。
///
/// 返回邻接图以及节点 ID -> 数据模型的映射（预留用于后续增强路径结果）。
async fn load_graph(
    db: &sea_orm::DatabaseConnection,
    investigation_id: &str,
) -> Result<
    (
        AdjacencyGraph,
        HashMap<String, crate::db::entities::node::Model>,
    ),
    AppError,
> {
    let node_repo = NodeRepo::new(db);
    let relation_repo = RelationRepo::new(db);

    let nodes = node_repo.find_by_investigation(investigation_id).await?;
    let relations = relation_repo
        .find_by_investigation(investigation_id)
        .await?;

    let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let mut node_map = HashMap::new();
    for node in &nodes {
        node_map.insert(node.id.clone(), node.clone());
    }

    let edges: Vec<EdgeInfo> = relations
        .iter()
        .map(|r| EdgeInfo {
            relation_id: r.id.clone(),
            source_id: r.source_node_id.clone(),
            target_id: r.target_node_id.clone(),
            relation_type: r.relation_type.clone(),
            label: r.label.clone(),
        })
        .collect();

    let graph = AdjacencyGraph::from_data(node_ids, edges);

    Ok((graph, node_map))
}
