use std::collections::{BTreeMap, HashMap, HashSet};

use rmcp::{handler::server::wrapper::Parameters, schemars::JsonSchema, tool_router};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use tauri_app_lib::{
    db::{
        entities::{investigation, node, relation},
        repositories::{node_repo::NodeRepo, relation_repo::RelationRepo},
    },
    error::AppError,
    graph::{
        traversal::{bfs_paths, connected_component, shortest_path},
        types::{AdjacencyGraph, EdgeInfo, PathResult, TraversalPath},
    },
};

use crate::server::CyberWeaverMcp;

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchNodesParams {
    investigation_id: String,
    node_type: Option<String>,
    keyword: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetNodeParams {
    node_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetNodeNeighborhoodParams {
    node_id: String,
    investigation_id: String,
    max_hops: Option<usize>,
    relation_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct FindPathParams {
    from_id: String,
    to_id: String,
    investigation_id: String,
    max_hops: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetGraphSummaryParams {
    investigation_id: String,
}

#[derive(Debug, Serialize)]
struct TraversalPathJson {
    node_ids: Vec<String>,
    relation_ids: Vec<String>,
    relation_types: Vec<String>,
}

impl From<TraversalPath> for TraversalPathJson {
    fn from(path: TraversalPath) -> Self {
        Self {
            node_ids: path.node_ids,
            relation_ids: path.relation_ids,
            relation_types: path.relation_types,
        }
    }
}

#[derive(Debug, Serialize)]
struct NeighborhoodResult {
    center_node: node::Model,
    neighbors: Vec<node::Model>,
    relations: Vec<relation::Model>,
}

#[derive(Debug, Serialize)]
struct GraphSummaryResult {
    node_count: usize,
    type_distribution: BTreeMap<String, usize>,
    relation_count: usize,
    component_count: usize,
}

struct GraphContext {
    graph: AdjacencyGraph,
    nodes: Vec<node::Model>,
    relations: Vec<relation::Model>,
    node_map: HashMap<String, node::Model>,
}

#[tool_router(router = read_tool_router, vis = "pub(crate)")]
impl CyberWeaverMcp {
    #[rmcp::tool(description = "List all investigations in the database.")]
    async fn list_investigations(&self, _: Parameters<()>) -> String {
        let investigations = investigation::Entity::find()
            .all(self.db())
            .await
            .unwrap_or_else(|error| panic!("failed to list investigations: {error}"));

        to_pretty_json(&investigations)
    }

    #[rmcp::tool(description = "Search nodes within an investigation with optional type and keyword filters.")]
    async fn search_nodes(&self, Parameters(params): Parameters<SearchNodesParams>) -> String {
        let repo = NodeRepo::new(self.db());
        let nodes = match params.node_type.as_deref() {
            Some(node_type) => repo
                .find_by_type(&params.investigation_id, node_type)
                .await
                .unwrap_or_else(|error| panic!("failed to search nodes by type: {error}")),
            None => repo
                .find_by_investigation(&params.investigation_id)
                .await
                .unwrap_or_else(|error| panic!("failed to search nodes: {error}")),
        };

        let filtered_nodes = match params.keyword.as_deref() {
            Some(keyword) => nodes
                .into_iter()
                .filter(|node| {
                    node.label.contains(keyword) || node.description.contains(keyword)
                })
                .collect::<Vec<_>>(),
            None => nodes,
        };

        to_pretty_json(&filtered_nodes)
    }

    #[rmcp::tool(description = "Fetch a single node by ID.")]
    async fn get_node(&self, Parameters(params): Parameters<GetNodeParams>) -> String {
        let repo = NodeRepo::new(self.db());
        let node = repo
            .find_by_id(&params.node_id)
            .await
            .unwrap_or_else(|error| panic!("failed to load node {}: {error}", params.node_id))
            .unwrap_or_else(|| panic!("node not found: {}", params.node_id));

        to_pretty_json(&node)
    }

    #[rmcp::tool(description = "Get a node neighborhood by BFS within an investigation graph.")]
    async fn get_node_neighborhood(
        &self,
        Parameters(params): Parameters<GetNodeNeighborhoodParams>,
    ) -> String {
        let graph_ctx = load_graph(self.db(), &params.investigation_id)
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "failed to load graph for investigation {}: {error}",
                    params.investigation_id
                )
            });

        let center_node = graph_ctx
            .node_map
            .get(&params.node_id)
            .cloned()
            .unwrap_or_else(|| panic!("node not found in investigation: {}", params.node_id));

        let path_result: PathResult = bfs_paths(
            &graph_ctx.graph,
            &params.node_id,
            params.max_hops.unwrap_or(2),
            params.relation_type.as_deref(),
        );

        let mut neighbor_ids = HashSet::new();
        let mut relation_ids = HashSet::new();

        for path in path_result.paths {
            for node_id in path.node_ids {
                if node_id != params.node_id {
                    neighbor_ids.insert(node_id);
                }
            }
            for relation_id in path.relation_ids {
                let relation_id = relation_id.strip_prefix("rev:").unwrap_or(&relation_id);
                relation_ids.insert(relation_id.to_string());
            }
        }

        let mut neighbors = graph_ctx
            .nodes
            .iter()
            .filter(|node| neighbor_ids.contains(&node.id))
            .cloned()
            .collect::<Vec<_>>();
        neighbors.sort_by(|left, right| left.created_at.cmp(&right.created_at));

        let mut relations = graph_ctx
            .relations
            .iter()
            .filter(|relation| relation_ids.contains(&relation.id))
            .cloned()
            .collect::<Vec<_>>();
        relations.sort_by(|left, right| left.created_at.cmp(&right.created_at));

        to_pretty_json(&NeighborhoodResult {
            center_node,
            neighbors,
            relations,
        })
    }

    #[rmcp::tool(description = "Find the shortest path between two nodes within an investigation graph.")]
    async fn find_path(&self, Parameters(params): Parameters<FindPathParams>) -> String {
        let graph_ctx = load_graph(self.db(), &params.investigation_id)
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "failed to load graph for investigation {}: {error}",
                    params.investigation_id
                )
            });

        let path = shortest_path(
            &graph_ctx.graph,
            &params.from_id,
            &params.to_id,
            params.max_hops.unwrap_or(5),
        )
        .map(TraversalPathJson::from);

        to_pretty_json(&path)
    }

    #[rmcp::tool(description = "Get summary statistics for an investigation graph.")]
    async fn get_graph_summary(&self, Parameters(params): Parameters<GetGraphSummaryParams>) -> String {
        let graph_ctx = load_graph(self.db(), &params.investigation_id)
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "failed to load graph for investigation {}: {error}",
                    params.investigation_id
                )
            });

        let mut type_distribution = BTreeMap::new();
        for node in &graph_ctx.nodes {
            *type_distribution.entry(node.node_type.clone()).or_insert(0) += 1;
        }

        let mut visited = HashSet::new();
        let mut component_count = 0;
        for node_id in &graph_ctx.graph.node_ids {
            if visited.contains(node_id) {
                continue;
            }

            let component = connected_component(&graph_ctx.graph, node_id);
            visited.extend(component);
            component_count += 1;
        }

        to_pretty_json(&GraphSummaryResult {
            node_count: graph_ctx.nodes.len(),
            type_distribution,
            relation_count: graph_ctx.relations.len(),
            component_count,
        })
    }
}

async fn load_graph(
    db: &sea_orm::DatabaseConnection,
    investigation_id: &str,
) -> Result<GraphContext, AppError> {
    let node_repo = NodeRepo::new(db);
    let relation_repo = RelationRepo::new(db);

    let nodes = node_repo.find_by_investigation(investigation_id).await?;
    let relations = relation_repo.find_by_investigation(investigation_id).await?;

    let node_ids = nodes.iter().map(|node| node.id.clone()).collect::<Vec<_>>();
    let edges = relations
        .iter()
        .map(|relation| EdgeInfo {
            relation_id: relation.id.clone(),
            source_id: relation.source_node_id.clone(),
            target_id: relation.target_node_id.clone(),
            relation_type: relation.relation_type.clone(),
            label: relation.label.clone(),
        })
        .collect::<Vec<_>>();

    let node_map = nodes
        .iter()
        .cloned()
        .map(|node| (node.id.clone(), node))
        .collect::<HashMap<_, _>>();
    Ok(GraphContext {
        graph: AdjacencyGraph::from_data(node_ids, edges),
        nodes,
        relations,
        node_map,
    })
}

fn to_pretty_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value)
        .unwrap_or_else(|error| panic!("failed to serialize tool response: {error}"))
}
