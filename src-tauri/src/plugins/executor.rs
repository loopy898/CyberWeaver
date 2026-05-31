use std::collections::HashMap;

use cw_plugin_sdk::{DiscoveredNode, DiscoveredRelation, ToolInput, ToolOutput};
use serde::Serialize;
use uuid::Uuid;

use crate::db::entities::{node, relation};
use crate::db::repositories::{CreateNodeData, CreateRelationData, NodeRepo, RelationRepo};
use crate::error::AppError;
use crate::models::domain::{NodeType, RelationType};
use crate::plugins::loader::PluginError;
use crate::state::AppState;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolExecutionResult {
    pub nodes_created: usize,
    pub relations_created: usize,
    pub text_summary: String,
    pub new_node_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
struct GraphMergeEvent<'a> {
    r#type: &'static str,
    delta: GraphMergeDelta<'a>,
}

#[derive(Debug, Serialize)]
struct GraphMergeDelta<'a> {
    new_nodes: &'a [node::Model],
    new_relations: &'a [relation::Model],
}

/// Execute a tool and merge its output into the investigation graph.
pub async fn execute_and_merge(
    state: &AppState,
    tool_name: &str,
    input: ToolInput,
    investigation_id: &str,
) -> Result<ToolExecutionResult, AppError> {
    let output = state
        .tool_registry
        .execute(tool_name, input)
        .await
        .map_err(map_plugin_error)?;

    merge_tool_output(state, investigation_id, output).await
}

async fn merge_tool_output(
    state: &AppState,
    investigation_id: &str,
    output: ToolOutput,
) -> Result<ToolExecutionResult, AppError> {
    let node_repo = NodeRepo::new(&state.db);
    let relation_repo = RelationRepo::new(&state.db);

    let mut label_to_node_id = node_repo
        .find_by_investigation(investigation_id)
        .await?
        .into_iter()
        .map(|node| (node.label, node.id))
        .collect::<HashMap<_, _>>();

    let mut created_nodes = Vec::with_capacity(output.new_nodes.len());
    let mut created_relations = Vec::with_capacity(output.new_relations.len());
    let mut new_node_ids = Vec::with_capacity(output.new_nodes.len());

    for discovered in output.new_nodes {
        let node_type = parse_node_type(&discovered)?;
        let create_data = CreateNodeData {
            fixed_id: Uuid::new_v4().to_string(),
            investigation_id: investigation_id.to_string(),
            node_type: enum_to_db_string(&node_type)?,
            label: discovered.label,
            description: discovered.description,
            confidence: discovered.confidence,
            properties: serde_json::to_string(&discovered.properties)?,
            pos_x: 0.0,
            pos_y: 0.0,
        };

        let created = node_repo.create(create_data).await?;
        label_to_node_id.insert(created.label.clone(), created.id.clone());
        new_node_ids.push(created.id.clone());
        created_nodes.push(created);
    }

    for discovered in output.new_relations {
        let relation_type = parse_relation_type(&discovered)?;
        let source_node_id = resolve_node_id(&label_to_node_id, &discovered.source_label)?;
        let target_node_id = resolve_node_id(&label_to_node_id, &discovered.target_label)?;

        let create_data = CreateRelationData {
            investigation_id: investigation_id.to_string(),
            relation_type: enum_to_db_string(&relation_type)?,
            source_node_id,
            target_node_id,
            label: discovered.label,
            confidence: discovered.confidence,
            first_seen: None,
            last_seen: None,
            properties: "{}".to_string(),
        };

        let created = relation_repo.create(create_data).await?;
        created_relations.push(created);
    }

    let payload = serde_json::to_string(&GraphMergeEvent {
        r#type: "graph_update",
        delta: GraphMergeDelta {
            new_nodes: &created_nodes,
            new_relations: &created_relations,
        },
    })?;
    let _ = state.ws_broadcast.send(payload);

    Ok(ToolExecutionResult {
        nodes_created: created_nodes.len(),
        relations_created: created_relations.len(),
        text_summary: output.text_summary,
        new_node_ids,
    })
}

fn resolve_node_id(
    label_to_node_id: &HashMap<String, String>,
    label: &str,
) -> Result<String, AppError> {
    label_to_node_id
        .get(label)
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("node label not found for relation mapping: {label}")))
}

fn parse_node_type(discovered: &DiscoveredNode) -> Result<NodeType, AppError> {
    serde_json::from_value(serde_json::Value::String(discovered.node_type.clone())).map_err(
        |_| AppError::InvalidInput(format!("invalid node_type from tool output: {}", discovered.node_type)),
    )
}

fn parse_relation_type(discovered: &DiscoveredRelation) -> Result<RelationType, AppError> {
    serde_json::from_value(serde_json::Value::String(discovered.relation_type.clone())).map_err(
        |_| {
            AppError::InvalidInput(format!(
                "invalid relation_type from tool output: {}",
                discovered.relation_type
            ))
        },
    )
}

fn enum_to_db_string<T: Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_value(value)?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| AppError::Internal("expected enum to serialize as string".to_string()))
}

fn map_plugin_error(error: PluginError) -> AppError {
    match error {
        PluginError::ExecutionFailed(message) => AppError::InvalidInput(message),
        PluginError::Json(error) => AppError::Serialization(error),
        PluginError::LoadError(error) => AppError::Internal(format!("plugin load error: {error}")),
        PluginError::VersionMismatch { expected, got } => AppError::Internal(format!(
            "plugin SDK version mismatch: expected {expected}, got {got}"
        )),
        PluginError::InitFailed => AppError::Internal("plugin initialization failed".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use cw_plugin_sdk::InvestigationTool;
    use sea_orm::{ActiveModelTrait, Database, Set};

    use crate::db::entities::investigation;
    use crate::db::migrations::run_migrations;
    use crate::plugins::registry::ToolRegistry;

    struct StubTool {
        manifest_name: &'static str,
        output: ToolOutput,
    }

    impl InvestigationTool for StubTool {
        fn manifest(&self) -> cw_plugin_sdk::ToolManifest {
            cw_plugin_sdk::ToolManifest {
                name: self.manifest_name.to_string(),
                display_name: "Stub".to_string(),
                description: "Stub test tool".to_string(),
                version: "1.0.0".to_string(),
                author: "tests".to_string(),
                parameters: Vec::new(),
                input_types: vec!["ip_address".to_string()],
                output_types: vec!["domain".to_string()],
            }
        }

        fn execute(&self, _input: ToolInput) -> Result<ToolOutput, String> {
            Ok(self.output.clone())
        }
    }

    async fn setup_state_with_tool(tool: Arc<dyn InvestigationTool>) -> AppState {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite should connect");
        run_migrations(&db)
            .await
            .expect("test migrations should succeed");

        investigation::ActiveModel {
            id: Set("inv-1".to_string()),
            name: Set("Investigation".to_string()),
            description: Set("Test investigation".to_string()),
            created_at: Set(crate::db::repositories::node_repo::chrono_now()),
            updated_at: Set(crate::db::repositories::node_repo::chrono_now()),
        }
        .insert(&db)
        .await
        .expect("test investigation should insert");

        let mut tool_registry = ToolRegistry::new();
        tool_registry.register_builtin(tool);
        AppState::new(db, tool_registry)
    }

    #[tokio::test]
    async fn executes_tool_and_merges_new_graph_objects() {
        let tool = Arc::new(StubTool {
            manifest_name: "vt_lookup",
            output: ToolOutput {
                new_nodes: vec![
                    DiscoveredNode {
                        node_type: "ip_address".to_string(),
                        label: "8.8.8.8".to_string(),
                        description: "Suspicious resolver".to_string(),
                        properties: serde_json::json!({"type": "ip_address", "address": "8.8.8.8"}),
                        confidence: 0.9,
                    },
                    DiscoveredNode {
                        node_type: "domain".to_string(),
                        label: "evil.example".to_string(),
                        description: "Observed domain".to_string(),
                        properties: serde_json::json!({"type": "domain", "domain": "evil.example"}),
                        confidence: 0.8,
                    },
                ],
                new_relations: vec![DiscoveredRelation {
                    source_label: "8.8.8.8".to_string(),
                    target_label: "evil.example".to_string(),
                    relation_type: "resolves_to".to_string(),
                    label: "resolves".to_string(),
                    confidence: 0.75,
                }],
                enriched_properties: serde_json::json!({}),
                text_summary: "VirusTotal found one domain.".to_string(),
            },
        });
        let state = setup_state_with_tool(tool).await;
        let mut rx = state.ws_broadcast.subscribe();

        let result = execute_and_merge(
            &state,
            "vt_lookup",
            ToolInput {
                node_id: None,
                params: serde_json::json!({}),
            },
            "inv-1",
        )
        .await
        .expect("tool execution should succeed");

        assert_eq!(result.nodes_created, 2);
        assert_eq!(result.relations_created, 1);
        assert_eq!(result.text_summary, "VirusTotal found one domain.");
        assert_eq!(result.new_node_ids.len(), 2);

        let node_repo = NodeRepo::new(&state.db);
        let relation_repo = RelationRepo::new(&state.db);
        assert_eq!(
            node_repo
                .find_by_investigation("inv-1")
                .await
                .expect("nodes should be queryable")
                .len(),
            2
        );
        assert_eq!(
            relation_repo
                .find_by_investigation("inv-1")
                .await
                .expect("relations should be queryable")
                .len(),
            1
        );

        let event = rx.recv().await.expect("merge should broadcast an event");
        let value: serde_json::Value =
            serde_json::from_str(&event).expect("event payload should be valid json");
        assert_eq!(value["type"], "graph_update");
        assert_eq!(value["delta"]["new_nodes"].as_array().map(Vec::len), Some(2));
        assert_eq!(
            value["delta"]["new_relations"].as_array().map(Vec::len),
            Some(1)
        );
    }

    #[tokio::test]
    async fn resolves_relation_labels_against_existing_nodes() {
        let tool = Arc::new(StubTool {
            manifest_name: "whois_domain",
            output: ToolOutput {
                new_nodes: vec![DiscoveredNode {
                    node_type: "domain".to_string(),
                    label: "evil.example".to_string(),
                    description: "Observed domain".to_string(),
                    properties: serde_json::json!({"type": "domain", "domain": "evil.example"}),
                    confidence: 0.8,
                }],
                new_relations: vec![DiscoveredRelation {
                    source_label: "10.0.0.5".to_string(),
                    target_label: "evil.example".to_string(),
                    relation_type: "connects_to".to_string(),
                    label: "contacts".to_string(),
                    confidence: 0.7,
                }],
                enriched_properties: serde_json::json!({}),
                text_summary: "WHOIS found an observed connection.".to_string(),
            },
        });
        let state = setup_state_with_tool(tool).await;

        let node_repo = NodeRepo::new(&state.db);
        let existing = node_repo
            .create(CreateNodeData {
                fixed_id: Uuid::new_v4().to_string(),
                investigation_id: "inv-1".to_string(),
                node_type: "ip_address".to_string(),
                label: "10.0.0.5".to_string(),
                description: "Existing host".to_string(),
                confidence: 1.0,
                properties: serde_json::json!({"type": "ip_address", "address": "10.0.0.5"})
                    .to_string(),
                pos_x: 10.0,
                pos_y: 20.0,
            })
            .await
            .expect("existing node should insert");

        let result = execute_and_merge(
            &state,
            "whois_domain",
            ToolInput {
                node_id: Some(existing.id.clone()),
                params: serde_json::json!({}),
            },
            "inv-1",
        )
        .await
        .expect("tool execution should succeed");

        assert_eq!(result.nodes_created, 1);
        assert_eq!(result.relations_created, 1);

        let relations = RelationRepo::new(&state.db)
            .find_by_investigation("inv-1")
            .await
            .expect("relations should be queryable");
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].source_node_id, existing.id);
    }
}
