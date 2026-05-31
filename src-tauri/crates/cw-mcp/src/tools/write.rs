use rmcp::{handler::server::wrapper::Parameters, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri_app_lib::db::entities::node;
use tauri_app_lib::db::repositories::node_repo::{CreateNodeData, NodeRepo, UpdateNodeData};
use tauri_app_lib::db::repositories::relation_repo::{CreateRelationData, RelationRepo};
use tauri_app_lib::error::AppError;
use tauri_app_lib::models::domain::{NodeType, RelationType};
use uuid::Uuid;

use crate::{error::McpError, server::CyberWeaverMcp};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AddNodeParams {
    pub investigation_id: String,
    pub node_type: String,
    pub label: String,
    pub description: Option<String>,
    pub properties: Option<String>,
    pub confidence: Option<f32>,
    pub pos_x: Option<f64>,
    pub pos_y: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AddRelationParams {
    pub investigation_id: String,
    pub relation_type: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub label: Option<String>,
    pub confidence: Option<f32>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct UpdateNodeParams {
    pub node_id: String,
    pub label: Option<String>,
    pub description: Option<String>,
    pub properties: Option<String>,
    pub confidence: Option<f32>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DeleteNodeParams {
    pub node_id: String,
}

#[tool_router(router = write_tool_router, vis = "pub(crate)")]
impl CyberWeaverMcp {
    #[rmcp::tool(description = "Add a node to an investigation.")]
    async fn add_node(
        &self,
        Parameters(params): Parameters<AddNodeParams>,
    ) -> Result<String, rmcp::ErrorData> {
        add_node_impl(self, params).await.map_err(Into::into)
    }

    #[rmcp::tool(description = "Add a relation between two nodes.")]
    async fn add_relation(
        &self,
        Parameters(params): Parameters<AddRelationParams>,
    ) -> Result<String, rmcp::ErrorData> {
        add_relation_impl(self, params).await.map_err(Into::into)
    }

    #[rmcp::tool(description = "Update mutable node fields.")]
    async fn update_node(
        &self,
        Parameters(params): Parameters<UpdateNodeParams>,
    ) -> Result<String, rmcp::ErrorData> {
        update_node_impl(self, params).await.map_err(Into::into)
    }

    #[rmcp::tool(description = "Delete a node and all connected relations.")]
    async fn delete_node(
        &self,
        Parameters(params): Parameters<DeleteNodeParams>,
    ) -> Result<String, rmcp::ErrorData> {
        delete_node_impl(self, params).await.map_err(Into::into)
    }
}

async fn add_node_impl(server: &CyberWeaverMcp, params: AddNodeParams) -> Result<String, McpError> {
    let node_type = parse_node_type(&params.node_type)?;
    let repo = NodeRepo::new(server.db());
    let created = repo
        .create(CreateNodeData {
            fixed_id: Uuid::new_v4().to_string(),
            investigation_id: params.investigation_id,
            node_type: enum_to_snake_case(&node_type)?,
            label: params.label,
            description: params.description.unwrap_or_default(),
            confidence: params.confidence.unwrap_or(1.0),
            properties: params.properties.unwrap_or_else(|| "{}".to_string()),
            pos_x: params.pos_x.unwrap_or(0.0),
            pos_y: params.pos_y.unwrap_or(0.0),
        })
        .await?;
    serialize_json(&created)
}

async fn add_relation_impl(
    server: &CyberWeaverMcp,
    params: AddRelationParams,
) -> Result<String, McpError> {
    let relation_type = parse_relation_type(&params.relation_type)?;
    let node_repo = NodeRepo::new(server.db());
    let relation_repo = RelationRepo::new(server.db());

    ensure_node_exists(&node_repo, &params.source_node_id, "source_node_id").await?;
    ensure_node_exists(&node_repo, &params.target_node_id, "target_node_id").await?;

    let created = relation_repo
        .create(CreateRelationData {
            investigation_id: params.investigation_id,
            relation_type: enum_to_snake_case(&relation_type)?,
            source_node_id: params.source_node_id,
            target_node_id: params.target_node_id,
            label: params.label.unwrap_or_default(),
            confidence: params.confidence.unwrap_or(1.0),
            first_seen: None,
            last_seen: None,
            properties: "{}".to_string(),
        })
        .await?;
    serialize_json(&created)
}

async fn update_node_impl(
    server: &CyberWeaverMcp,
    params: UpdateNodeParams,
) -> Result<String, McpError> {
    let repo = NodeRepo::new(server.db());
    let updated = repo
        .update(
            &params.node_id,
            UpdateNodeData {
                label: params.label,
                description: params.description,
                confidence: params.confidence,
                properties: params.properties,
                pos_x: None,
                pos_y: None,
            },
        )
        .await?;
    serialize_json(&updated)
}

async fn delete_node_impl(
    server: &CyberWeaverMcp,
    params: DeleteNodeParams,
) -> Result<String, McpError> {
    let node_repo = NodeRepo::new(server.db());
    let relation_repo = RelationRepo::new(server.db());

    let existing = node_repo.find_by_id(&params.node_id).await?;
    if existing.is_none() {
        return Err(McpError::NotFound(format!(
            "node not found: {}",
            params.node_id
        )));
    }

    relation_repo.delete_by_node(&params.node_id).await?;
    node_repo.delete(&params.node_id).await?;

    serialize_json(&json!({
        "deleted": true,
        "node_id": params.node_id,
    }))
}

pub async fn ensure_node_exists(
    repo: &NodeRepo<'_>,
    node_id: &str,
    field_name: &str,
) -> Result<node::Model, McpError> {
    repo.find_by_id(node_id)
        .await?
        .ok_or_else(|| McpError::NotFound(format!("{field_name} not found: {node_id}")))
}

pub fn parse_node_type(value: &str) -> Result<NodeType, McpError> {
    match value {
        "ip_address" => Ok(NodeType::IpAddress),
        "domain" => Ok(NodeType::Domain),
        "file_hash" => Ok(NodeType::FileHash),
        "process" => Ok(NodeType::Process),
        "malware" => Ok(NodeType::Malware),
        "ttp" => Ok(NodeType::Ttp),
        "threat_actor" => Ok(NodeType::ThreatActor),
        "asset" => Ok(NodeType::Asset),
        _ => Err(McpError::App(AppError::InvalidInput(format!(
            "unsupported node_type: {value}"
        )))),
    }
}

pub fn parse_relation_type(value: &str) -> Result<RelationType, McpError> {
    match value {
        "connects_to" => Ok(RelationType::ConnectsTo),
        "resolves_to" => Ok(RelationType::ResolvesTo),
        "creates" => Ok(RelationType::Creates),
        "belongs_to" => Ok(RelationType::BelongsTo),
        "uses" => Ok(RelationType::Uses),
        "targets" => Ok(RelationType::Targets),
        "contains" => Ok(RelationType::Contains),
        _ => Err(McpError::App(AppError::InvalidInput(format!(
            "unsupported relation_type: {value}"
        )))),
    }
}

pub fn enum_to_snake_case<T: Serialize>(value: &T) -> Result<String, McpError> {
    serde_json::to_value(value)
        .map_err(|error| McpError::App(AppError::Serialization(error)))?
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            McpError::App(AppError::Internal(
                "expected enum to serialize as string".to_string(),
            ))
        })
}

pub fn serialize_json<T: Serialize>(value: &T) -> Result<String, McpError> {
    serde_json::to_string(value).map_err(|error| McpError::App(AppError::Serialization(error)))
}
