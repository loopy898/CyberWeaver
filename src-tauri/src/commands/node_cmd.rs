//! Tauri commands for node CRUD operations.

use tauri::State;

use crate::db::entities::node;
use crate::db::repositories::{CreateNodeData, NodeRepo, UpdateNodeData};
use crate::models::domain::{NodeData, NodeType, TypeSpecificProps};
use crate::state::AppState;

fn parse_node_type(value: &str) -> Option<NodeType> {
    serde_json::from_str(&format!("\"{value}\""))
        .ok()
        .or_else(|| match value {
            "IpAddress" => Some(NodeType::IpAddress),
            "Domain" => Some(NodeType::Domain),
            "FileHash" => Some(NodeType::FileHash),
            "Process" => Some(NodeType::Process),
            "Malware" => Some(NodeType::Malware),
            "Ttp" => Some(NodeType::Ttp),
            "ThreatActor" => Some(NodeType::ThreatActor),
            "Asset" => Some(NodeType::Asset),
            _ => None,
        })
}

/// Convert a SeaORM node model to our domain `NodeData` transfer struct.
fn model_to_node_data(n: node::Model) -> NodeData {
    let node_type = parse_node_type(&n.node_type).unwrap_or(NodeType::Asset);
    let properties: TypeSpecificProps =
        serde_json::from_str(&n.properties).unwrap_or(TypeSpecificProps::Asset(Default::default()));

    NodeData {
        id: n.id,
        node_type,
        label: n.label,
        description: n.description,
        confidence: n.confidence,
        properties,
        pos_x: n.pos_x,
        pos_y: n.pos_y,
        investigation_id: n.investigation_id,
        created_at: Some(n.created_at),
        updated_at: Some(n.updated_at),
    }
}

/// Return the default `TypeSpecificProps` variant matching the given node type.
fn default_properties_for(node_type: &NodeType) -> TypeSpecificProps {
    match node_type {
        NodeType::IpAddress => TypeSpecificProps::IpAddress(Default::default()),
        NodeType::Domain => TypeSpecificProps::Domain(Default::default()),
        NodeType::FileHash => TypeSpecificProps::FileHash(Default::default()),
        NodeType::Process => TypeSpecificProps::Process(Default::default()),
        NodeType::Malware => TypeSpecificProps::Malware(Default::default()),
        NodeType::Ttp => TypeSpecificProps::Ttp(Default::default()),
        NodeType::ThreatActor => TypeSpecificProps::ThreatActor(Default::default()),
        NodeType::Asset => TypeSpecificProps::Asset(Default::default()),
    }
}

/// List all nodes belonging to an investigation.
#[tauri::command]
pub async fn get_nodes(
    investigation_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<NodeData>, String> {
    let repo = NodeRepo::new(&state.db);
    let nodes = repo
        .find_by_investigation(&investigation_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(nodes.into_iter().map(model_to_node_data).collect())
}

/// Fetch a single node by id.
#[tauri::command]
pub async fn get_node(id: String, state: State<'_, AppState>) -> Result<Option<NodeData>, String> {
    let repo = NodeRepo::new(&state.db);
    let node = repo.find_by_id(&id).await.map_err(|e| e.to_string())?;
    Ok(node.map(model_to_node_data))
}

/// Create a new node with sensible defaults and return it.
#[tauri::command]
pub async fn create_node(
    investigation_id: String,
    node_type: String,
    label: String,
    pos_x: f64,
    pos_y: f64,
    state: State<'_, AppState>,
) -> Result<NodeData, String> {
    // Validate node_type by converting to our enum.
    let nt: NodeType = serde_json::from_str(&format!("\"{}\"", node_type))
        .map_err(|_| format!("invalid node_type: {node_type}"))?;

    let properties = serde_json::to_string(&default_properties_for(&nt))
        .map_err(|e| format!("failed to serialize default properties: {e}"))?;

    let data = CreateNodeData {
        fixed_id: uuid::Uuid::new_v4().to_string(),
        investigation_id,
        node_type,
        label,
        description: String::new(),
        confidence: 1.0,
        properties,
        pos_x,
        pos_y,
    };

    let repo = NodeRepo::new(&state.db);
    let node = repo.create(data).await.map_err(|e| e.to_string())?;
    Ok(model_to_node_data(node))
}

/// Update the mutable fields of an existing node.
///
/// Only the supplied `Some(...)` fields are applied; `None` fields are
/// left unchanged.
#[tauri::command]
pub async fn update_node(
    id: String,
    label: Option<String>,
    description: Option<String>,
    confidence: Option<f32>,
    properties: Option<String>,
    pos_x: Option<f64>,
    pos_y: Option<f64>,
    state: State<'_, AppState>,
) -> Result<NodeData, String> {
    let data = UpdateNodeData {
        label,
        description,
        confidence,
        properties,
        pos_x,
        pos_y,
    };

    let repo = NodeRepo::new(&state.db);
    let node = repo.update(&id, data).await.map_err(|e| e.to_string())?;
    Ok(model_to_node_data(node))
}

/// Delete a node and all its associated relations (cascading).
#[tauri::command]
pub async fn delete_node(id: String, state: State<'_, AppState>) -> Result<(), String> {
    // Remove all relations pointing to/from this node first.
    let rel_repo = crate::db::repositories::RelationRepo::new(&state.db);
    rel_repo
        .delete_by_node(&id)
        .await
        .map_err(|e| e.to_string())?;

    let repo = NodeRepo::new(&state.db);
    repo.delete(&id).await.map_err(|e| e.to_string())?;
    Ok(())
}
