use std::collections::{HashMap, HashSet};

use rmcp::{handler::server::wrapper::Parameters, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use tauri_app_lib::{
    ai::agent::ForensicsAgent,
    db::repositories::{node_repo::NodeRepo, relation_repo::RelationRepo},
    error::AppError,
    services::llm::{
        client::LlmClient,
        extractor::{extract_entities, extract_relations, ExtractedEntity},
    },
    state::LlmConfig,
};

use crate::{error::McpError, server::CyberWeaverMcp, tools::write::serialize_json};

#[derive(Debug, Deserialize, JsonSchema)]
struct ExtractFromTextParams {
    text: String,
    api_base: String,
    api_key: String,
    model: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ExtractRelationsParams {
    text: String,
    entities_json: String,
    api_base: String,
    api_key: String,
    model: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AgentAnalyzeParams {
    investigation_id: String,
    node_ids: Option<Vec<String>>,
    api_base: String,
    api_key: String,
    model: String,
}

#[tool_router(router = ai_tool_router, vis = "pub(crate)")]
impl CyberWeaverMcp {
    #[rmcp::tool(description = "Extract entities from unstructured text using the configured LLM.")]
    async fn extract_from_text(
        &self,
        Parameters(params): Parameters<ExtractFromTextParams>,
    ) -> Result<String, rmcp::ErrorData> {
        extract_from_text_impl(params).await.map_err(Into::into)
    }

    #[rmcp::tool(description = "Extract relations from text using previously extracted entities.")]
    async fn extract_relations(
        &self,
        Parameters(params): Parameters<ExtractRelationsParams>,
    ) -> Result<String, rmcp::ErrorData> {
        extract_relations_impl(params).await.map_err(Into::into)
    }

    #[rmcp::tool(description = "Analyze investigation graph context and propose AI-assisted next actions.")]
    async fn agent_analyze(
        &self,
        Parameters(params): Parameters<AgentAnalyzeParams>,
    ) -> Result<String, rmcp::ErrorData> {
        agent_analyze_impl(self, params).await.map_err(Into::into)
    }
}

async fn extract_from_text_impl(params: ExtractFromTextParams) -> Result<String, McpError> {
    let config = llm_config_from_parts(
        &params.api_base,
        &params.api_key,
        &params.model,
    );
    let client = LlmClient::new(config);
    let entities = extract_entities(&client, &params.text).await?;
    serialize_json(&entities)
}

async fn extract_relations_impl(params: ExtractRelationsParams) -> Result<String, McpError> {
    let entities: Vec<ExtractedEntity> = serde_json::from_str(&params.entities_json)
        .map_err(|error| McpError::App(AppError::Serialization(error)))?;
    let config = llm_config_from_parts(
        &params.api_base,
        &params.api_key,
        &params.model,
    );
    let client = LlmClient::new(config);
    let relations = extract_relations(&client, &entities, &params.text).await?;
    serialize_json(&relations)
}

async fn agent_analyze_impl(
    server: &CyberWeaverMcp,
    params: AgentAnalyzeParams,
) -> Result<String, McpError> {
    let node_repo = NodeRepo::new(server.db());
    let relation_repo = RelationRepo::new(server.db());

    let all_nodes = node_repo.find_by_investigation(&params.investigation_id).await?;
    let selected_node_ids = params
        .node_ids
        .as_ref()
        .map(|ids| ids.iter().cloned().collect::<HashSet<_>>());

    let nodes = filter_nodes(all_nodes, selected_node_ids.as_ref())?;
    let node_map: HashMap<String, tauri_app_lib::db::entities::node::Model> = nodes
        .iter()
        .cloned()
        .map(|node| (node.id.clone(), node))
        .collect();

    let all_relations = relation_repo
        .find_by_investigation(&params.investigation_id)
        .await?;
    let relations = filter_relations(all_relations, selected_node_ids.as_ref());

    let node_summaries = nodes.iter().map(node_summary).collect::<Vec<_>>();
    let relation_summaries = relations
        .iter()
        .filter_map(|relation| relation_summary(relation, &node_map))
        .collect::<Vec<_>>();

    let config = llm_config_from_parts(
        &params.api_base,
        &params.api_key,
        &params.model,
    );
    let agent = ForensicsAgent::new(config);
    let plan = agent
        .analyze(&node_summaries, &relation_summaries)
        .await?;

    serialize_json(&plan)
}

fn llm_config_from_parts(api_base: &str, api_key: &str, model: &str) -> LlmConfig {
    let api_base = api_base.trim().trim_end_matches('/').to_string();
    let model = model.trim().to_string();

    LlmConfig {
        configured: !api_base.is_empty() && !model.is_empty(),
        api_base,
        api_key: api_key.trim().to_string(),
        model,
    }
}

fn filter_nodes(
    nodes: Vec<tauri_app_lib::db::entities::node::Model>,
    selected_node_ids: Option<&HashSet<String>>,
) -> Result<Vec<tauri_app_lib::db::entities::node::Model>, McpError> {
    match selected_node_ids {
        Some(selected_node_ids) => {
            let filtered = nodes
                .into_iter()
                .filter(|node| selected_node_ids.contains(&node.id))
                .collect::<Vec<_>>();

            let found_ids = filtered
                .iter()
                .map(|node| node.id.clone())
                .collect::<HashSet<_>>();
            let missing_ids = selected_node_ids
                .iter()
                .filter(|node_id| !found_ids.contains(*node_id))
                .cloned()
                .collect::<Vec<_>>();

            if !missing_ids.is_empty() {
                return Err(McpError::NotFound(format!(
                    "node_ids not found in investigation {}: {}",
                    missing_ids.join(", "),
                    missing_ids.len()
                )));
            }

            Ok(filtered)
        }
        None => Ok(nodes),
    }
}

fn filter_relations(
    relations: Vec<tauri_app_lib::db::entities::relation::Model>,
    selected_node_ids: Option<&HashSet<String>>,
) -> Vec<tauri_app_lib::db::entities::relation::Model> {
    match selected_node_ids {
        Some(selected_node_ids) => relations
            .into_iter()
            .filter(|relation| {
                selected_node_ids.contains(&relation.source_node_id)
                    && selected_node_ids.contains(&relation.target_node_id)
            })
            .collect(),
        None => relations,
    }
}

fn node_summary(node: &tauri_app_lib::db::entities::node::Model) -> String {
    format!(
        "{}: {} — {}",
        node.node_type, node.label, node.description
    )
}

fn relation_summary(
    relation: &tauri_app_lib::db::entities::relation::Model,
    node_map: &HashMap<String, tauri_app_lib::db::entities::node::Model>,
) -> Option<String> {
    let source = node_map.get(&relation.source_node_id)?;
    let target = node_map.get(&relation.target_node_id)?;

    Some(format!(
        "{} -[{}]-> {}",
        source.label, relation.relation_type, target.label
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use tauri_app_lib::db::entities::{node, relation};

    use super::{filter_relations, llm_config_from_parts, node_summary, relation_summary};

    #[test]
    fn builds_llm_config_from_params() {
        let config = llm_config_from_parts(" https://example.com/ ", " secret ", " model-x ");

        assert_eq!(config.api_base, "https://example.com");
        assert_eq!(config.api_key, "secret");
        assert_eq!(config.model, "model-x");
        assert!(config.configured);
    }

    #[test]
    fn marks_llm_config_unconfigured_when_required_fields_missing() {
        let config = llm_config_from_parts("", "secret", "   ");

        assert!(!config.configured);
    }

    #[test]
    fn formats_node_summary() {
        let node = node::Model {
            id: "n1".to_string(),
            investigation_id: "inv-1".to_string(),
            node_type: "domain".to_string(),
            label: "example.com".to_string(),
            description: "C2 domain".to_string(),
            confidence: 0.9,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
            created_at: "2026-01-01 00:00:00".to_string(),
            updated_at: "2026-01-01 00:00:00".to_string(),
        };

        assert_eq!(node_summary(&node), "domain: example.com — C2 domain");
    }

    #[test]
    fn formats_relation_summary_from_node_labels() {
        let source = node::Model {
            id: "src".to_string(),
            investigation_id: "inv-1".to_string(),
            node_type: "ip_address".to_string(),
            label: "10.0.0.1".to_string(),
            description: String::new(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
            created_at: "2026-01-01 00:00:00".to_string(),
            updated_at: "2026-01-01 00:00:00".to_string(),
        };
        let target = node::Model {
            id: "dst".to_string(),
            investigation_id: "inv-1".to_string(),
            node_type: "domain".to_string(),
            label: "example.com".to_string(),
            description: String::new(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
            created_at: "2026-01-01 00:00:00".to_string(),
            updated_at: "2026-01-01 00:00:00".to_string(),
        };
        let relation = relation::Model {
            id: "r1".to_string(),
            investigation_id: "inv-1".to_string(),
            relation_type: "resolves_to".to_string(),
            source_node_id: "src".to_string(),
            target_node_id: "dst".to_string(),
            label: String::new(),
            confidence: 0.8,
            first_seen: None,
            last_seen: None,
            properties: "{}".to_string(),
            created_at: "2026-01-01 00:00:00".to_string(),
        };

        let node_map = HashMap::from([
            (source.id.clone(), source),
            (target.id.clone(), target),
        ]);

        assert_eq!(
            relation_summary(&relation, &node_map).as_deref(),
            Some("10.0.0.1 -[resolves_to]-> example.com")
        );
    }

    #[test]
    fn filters_relations_to_selected_nodes() {
        let selected = HashSet::from(["src".to_string(), "dst".to_string()]);
        let kept = relation::Model {
            id: "r1".to_string(),
            investigation_id: "inv-1".to_string(),
            relation_type: "uses".to_string(),
            source_node_id: "src".to_string(),
            target_node_id: "dst".to_string(),
            label: String::new(),
            confidence: 0.7,
            first_seen: None,
            last_seen: None,
            properties: "{}".to_string(),
            created_at: "2026-01-01 00:00:00".to_string(),
        };
        let dropped = relation::Model {
            id: "r2".to_string(),
            investigation_id: "inv-1".to_string(),
            relation_type: "uses".to_string(),
            source_node_id: "src".to_string(),
            target_node_id: "other".to_string(),
            label: String::new(),
            confidence: 0.7,
            first_seen: None,
            last_seen: None,
            properties: "{}".to_string(),
            created_at: "2026-01-01 00:00:00".to_string(),
        };

        let filtered = filter_relations(vec![kept.clone(), dropped], Some(&selected));

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, kept.id);
    }
}
