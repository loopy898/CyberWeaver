use rmcp::{handler::server::wrapper::Parameters, schemars::JsonSchema, tool_router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri_app_lib::{
    db::{
        entities::{node, relation},
        repositories::{
            node_repo::{CreateNodeData, NodeRepo},
            relation_repo::{CreateRelationData, RelationRepo},
        },
    },
    error::AppError,
    models::{
        canvas_format::to_json_canvas,
        domain::{NodeData, RelationData},
        stix::to_stix_bundle,
    },
    services::{
        import::{
            canvas_importer::import_canvas_json, stix_importer::import_stix_json,
        },
        report::generator::{generate_html_report, ReportConfig},
    },
};

use crate::{error::McpError, server::CyberWeaverMcp};

#[derive(Debug, Deserialize, JsonSchema)]
struct ImportStixParams {
    investigation_id: String,
    stix_json: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ExportStixParams {
    investigation_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ImportJsonCanvasParams {
    investigation_id: String,
    canvas_json: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ExportJsonCanvasParams {
    investigation_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GenerateReportParams {
    investigation_id: String,
    title: Option<String>,
    author: Option<String>,
    organization: Option<String>,
}

#[derive(Debug, Serialize)]
struct ImportResult {
    nodes_imported: usize,
    relations_imported: usize,
    errors: Vec<String>,
}

#[tool_router(router = import_export_tool_router, vis = "pub(crate)")]
impl CyberWeaverMcp {
    #[rmcp::tool(description = "Import STIX JSON into an investigation.")]
    async fn import_stix(
        &self,
        Parameters(params): Parameters<ImportStixParams>,
    ) -> Result<String, rmcp::ErrorData> {
        import_stix_impl(self, params).await.map_err(Into::into)
    }

    #[rmcp::tool(description = "Export an investigation as STIX JSON.")]
    async fn export_stix(
        &self,
        Parameters(params): Parameters<ExportStixParams>,
    ) -> Result<String, rmcp::ErrorData> {
        export_stix_impl(self, params).await.map_err(Into::into)
    }

    #[rmcp::tool(description = "Import JSON Canvas data into an investigation.")]
    async fn import_json_canvas(
        &self,
        Parameters(params): Parameters<ImportJsonCanvasParams>,
    ) -> Result<String, rmcp::ErrorData> {
        import_json_canvas_impl(self, params).await.map_err(Into::into)
    }

    #[rmcp::tool(description = "Export an investigation as JSON Canvas.")]
    async fn export_json_canvas(
        &self,
        Parameters(params): Parameters<ExportJsonCanvasParams>,
    ) -> Result<String, rmcp::ErrorData> {
        export_json_canvas_impl(self, params).await.map_err(Into::into)
    }

    #[rmcp::tool(description = "Generate an HTML report for an investigation.")]
    async fn generate_report(
        &self,
        Parameters(params): Parameters<GenerateReportParams>,
    ) -> Result<String, rmcp::ErrorData> {
        generate_report_impl(self, params).await.map_err(Into::into)
    }
}

async fn import_stix_impl(
    server: &CyberWeaverMcp,
    params: ImportStixParams,
) -> Result<String, McpError> {
    let result = import_graph_data(
        server,
        &params.investigation_id,
        &params.stix_json,
        import_stix_json,
    )
    .await?;
    serialize_json(&result)
}

async fn export_stix_impl(
    server: &CyberWeaverMcp,
    params: ExportStixParams,
) -> Result<String, McpError> {
    let (nodes, relations) = load_graph_data(server, &params.investigation_id).await?;
    serde_json::to_string_pretty(&to_stix_bundle(&nodes, &relations))
        .map_err(|error| McpError::App(AppError::Serialization(error)))
}

async fn import_json_canvas_impl(
    server: &CyberWeaverMcp,
    params: ImportJsonCanvasParams,
) -> Result<String, McpError> {
    let result = import_graph_data(
        server,
        &params.investigation_id,
        &params.canvas_json,
        import_canvas_json,
    )
    .await?;
    serialize_json(&result)
}

async fn export_json_canvas_impl(
    server: &CyberWeaverMcp,
    params: ExportJsonCanvasParams,
) -> Result<String, McpError> {
    let (nodes, relations) = load_graph_data(server, &params.investigation_id).await?;
    serde_json::to_string_pretty(&to_json_canvas(&nodes, &relations))
        .map_err(|error| McpError::App(AppError::Serialization(error)))
}

async fn generate_report_impl(
    server: &CyberWeaverMcp,
    params: GenerateReportParams,
) -> Result<String, McpError> {
    let (nodes, relations) = load_graph_data(server, &params.investigation_id).await?;
    Ok(generate_html_report(
        &nodes,
        &relations,
        &ReportConfig {
            title: params.title.unwrap_or_default(),
            author: params.author.unwrap_or_default(),
            organization: params.organization.unwrap_or_default(),
            include_ioc_list: true,
            include_graph_summary: true,
        },
    ))
}

type ImportFn = fn(&str) -> Result<(Vec<NodeData>, Vec<RelationData>), AppError>;

async fn import_graph_data(
    server: &CyberWeaverMcp,
    investigation_id: &str,
    json: &str,
    import_fn: ImportFn,
) -> Result<ImportResult, McpError> {
    let (nodes, relations) = import_fn(json)?;
    let node_repo = NodeRepo::new(server.db());
    let relation_repo = RelationRepo::new(server.db());

    for node_data in &nodes {
        let create_node = to_create_node_data(investigation_id, node_data)?;
        node_repo.upsert_batch(vec![create_node]).await?;
    }

    for relation_data in &relations {
        let create_relation = to_create_relation_data(investigation_id, relation_data)?;
        relation_repo.create(create_relation).await?;
    }

    Ok(ImportResult {
        nodes_imported: nodes.len(),
        relations_imported: relations.len(),
        errors: Vec::new(),
    })
}

async fn load_graph_data(
    server: &CyberWeaverMcp,
    investigation_id: &str,
) -> Result<(Vec<NodeData>, Vec<RelationData>), McpError> {
    let node_models = NodeRepo::new(server.db())
        .find_by_investigation(investigation_id)
        .await?;
    let relation_models = RelationRepo::new(server.db())
        .find_by_investigation(investigation_id)
        .await?;

    let nodes = node_models
        .into_iter()
        .map(model_to_node_data)
        .collect::<Result<Vec<_>, _>>()?;
    let relations = relation_models
        .into_iter()
        .map(model_to_relation_data)
        .collect::<Result<Vec<_>, _>>()?;

    Ok((nodes, relations))
}

fn to_create_node_data(
    investigation_id: &str,
    node: &NodeData,
) -> Result<CreateNodeData, McpError> {
    Ok(CreateNodeData {
        fixed_id: node.id.clone(),
        investigation_id: investigation_id.to_string(),
        node_type: serde_json::to_value(&node.node_type)
            .map_err(|error| McpError::App(AppError::Serialization(error)))?
            .as_str()
            .ok_or_else(|| {
                McpError::App(AppError::Internal(
                    "expected node_type to serialize as string".to_string(),
                ))
            })?
            .to_string(),
        label: node.label.clone(),
        description: node.description.clone(),
        confidence: node.confidence,
        properties: serde_json::to_string(&node.properties)
            .map_err(|error| McpError::App(AppError::Serialization(error)))?,
        pos_x: node.pos_x,
        pos_y: node.pos_y,
    })
}

fn to_create_relation_data(
    investigation_id: &str,
    relation: &RelationData,
) -> Result<CreateRelationData, McpError> {
    Ok(CreateRelationData {
        investigation_id: investigation_id.to_string(),
        relation_type: serde_json::to_value(&relation.relation_type)
            .map_err(|error| McpError::App(AppError::Serialization(error)))?
            .as_str()
            .ok_or_else(|| {
                McpError::App(AppError::Internal(
                    "expected relation_type to serialize as string".to_string(),
                ))
            })?
            .to_string(),
        source_node_id: relation.source_node_id.clone(),
        target_node_id: relation.target_node_id.clone(),
        label: relation.label.clone(),
        confidence: relation.confidence,
        first_seen: relation.first_seen.clone(),
        last_seen: relation.last_seen.clone(),
        properties: "{}".to_string(),
    })
}

fn model_to_node_data(model: node::Model) -> Result<NodeData, McpError> {
    let mut value = serde_json::to_value(model)
        .map_err(|error| McpError::App(AppError::Serialization(error)))?;

    let properties = value
        .get("properties")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            McpError::App(AppError::Internal(
                "expected node properties to serialize as string".to_string(),
            ))
        })?;
    let properties = serde_json::from_str::<Value>(properties)
        .map_err(|error| McpError::App(AppError::Serialization(error)))?;

    let object = value.as_object_mut().ok_or_else(|| {
        McpError::App(AppError::Internal(
            "expected serialized node model to be a JSON object".to_string(),
        ))
    })?;
    object.insert("properties".to_string(), properties);

    serde_json::from_value(value).map_err(|error| McpError::App(AppError::Serialization(error)))
}

fn model_to_relation_data(model: relation::Model) -> Result<RelationData, McpError> {
    let value = serde_json::to_value(model)
        .map_err(|error| McpError::App(AppError::Serialization(error)))?;
    serde_json::from_value(value).map_err(|error| McpError::App(AppError::Serialization(error)))
}

fn serialize_json<T: Serialize>(value: &T) -> Result<String, McpError> {
    serde_json::to_string(value).map_err(|error| McpError::App(AppError::Serialization(error)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri_app_lib::models::domain::{
        AssetProps, NodeType, RelationType, TypeSpecificProps,
    };

    #[test]
    fn model_to_node_data_round_trips_serialized_model() {
        let model = node::Model {
            id: "node-1".to_string(),
            investigation_id: "inv-1".to_string(),
            node_type: "asset".to_string(),
            label: "Host A".to_string(),
            description: "desc".to_string(),
            confidence: 0.8,
            properties: serde_json::to_string(&TypeSpecificProps::Asset(AssetProps {
                hostname: "Host A".to_string(),
                os: None,
                ip_addresses: Vec::new(),
                owner: None,
                criticality: None,
            }))
            .expect("properties should serialize"),
            pos_x: 1.0,
            pos_y: 2.0,
            created_at: "2026-01-01 00:00:00".to_string(),
            updated_at: "2026-01-01 00:00:01".to_string(),
        };

        let result = model_to_node_data(model);

        assert!(result.is_ok());
        let node = result.expect("model conversion should succeed");
        assert_eq!(node.id, "node-1");
        assert_eq!(node.node_type, NodeType::Asset);
        match node.properties {
            TypeSpecificProps::Asset(props) => assert_eq!(props.hostname, "Host A"),
            other => panic!("unexpected properties: {other:?}"),
        }
    }

    #[test]
    fn model_to_relation_data_round_trips_serialized_model() {
        let model = relation::Model {
            id: "rel-1".to_string(),
            investigation_id: "inv-1".to_string(),
            relation_type: "connects_to".to_string(),
            source_node_id: "node-1".to_string(),
            target_node_id: "node-2".to_string(),
            label: "edge".to_string(),
            confidence: 1.0,
            first_seen: None,
            last_seen: None,
            properties: "{}".to_string(),
            created_at: "2026-01-01 00:00:00".to_string(),
        };

        let result = model_to_relation_data(model);

        assert!(result.is_ok());
        let relation = result.expect("model conversion should succeed");
        assert_eq!(relation.id, "rel-1");
        assert_eq!(relation.relation_type, RelationType::ConnectsTo);
    }
}
