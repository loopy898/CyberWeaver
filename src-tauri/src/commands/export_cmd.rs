//! Tauri commands for exporting investigation data.

use serde_json::Value;
use tauri::State;

use crate::db::entities::{node, relation};
use crate::db::repositories::{NodeRepo, RelationRepo};
use crate::error::AppError;
use crate::models::attack_flow::to_attack_flow;
use crate::models::canvas_format::to_json_canvas;
use crate::models::domain::{NodeData, NodeType, RelationData, RelationType, TypeSpecificProps};
use crate::models::stix::to_stix_bundle;
use crate::services::report::generator::{generate_html_report, ReportConfig};
use crate::state::AppState;

fn parse_node_type(value: &str) -> Result<NodeType, AppError> {
    serde_json::from_value(Value::String(value.to_string())).or_else(|_| match value {
        "IpAddress" => Ok(NodeType::IpAddress),
        "Domain" => Ok(NodeType::Domain),
        "FileHash" => Ok(NodeType::FileHash),
        "Process" => Ok(NodeType::Process),
        "Malware" => Ok(NodeType::Malware),
        "Ttp" => Ok(NodeType::Ttp),
        "ThreatActor" => Ok(NodeType::ThreatActor),
        "Asset" => Ok(NodeType::Asset),
        _ => Err(AppError::Import(format!("unsupported node_type: {value}"))),
    })
}

fn parse_relation_type(value: &str) -> Result<RelationType, AppError> {
    serde_json::from_value(Value::String(value.to_string())).or_else(|_| match value {
        "ConnectsTo" => Ok(RelationType::ConnectsTo),
        "ResolvesTo" => Ok(RelationType::ResolvesTo),
        "Creates" => Ok(RelationType::Creates),
        "BelongsTo" => Ok(RelationType::BelongsTo),
        "Uses" => Ok(RelationType::Uses),
        "Targets" => Ok(RelationType::Targets),
        "Contains" => Ok(RelationType::Contains),
        _ => Err(AppError::Import(format!(
            "unsupported relation_type: {value}"
        ))),
    })
}

fn model_to_node_data(model: node::Model) -> Result<NodeData, AppError> {
    let node_type = parse_node_type(&model.node_type)?;
    let properties: TypeSpecificProps = serde_json::from_str(&model.properties)?;

    Ok(NodeData {
        id: model.id,
        node_type,
        label: model.label,
        description: model.description,
        confidence: model.confidence,
        properties,
        pos_x: model.pos_x,
        pos_y: model.pos_y,
        investigation_id: model.investigation_id,
        created_at: Some(model.created_at),
        updated_at: Some(model.updated_at),
    })
}

fn model_to_relation_data(model: relation::Model) -> Result<RelationData, AppError> {
    let relation_type = parse_relation_type(&model.relation_type)?;

    Ok(RelationData {
        id: model.id,
        relation_type,
        source_node_id: model.source_node_id,
        target_node_id: model.target_node_id,
        label: model.label,
        confidence: model.confidence,
        first_seen: model.first_seen,
        last_seen: model.last_seen,
        investigation_id: model.investigation_id,
    })
}

async fn load_graph_data(
    state: &State<'_, AppState>,
    investigation_id: &str,
) -> Result<(Vec<NodeData>, Vec<RelationData>), AppError> {
    let node_models = NodeRepo::new(&state.db)
        .find_by_investigation(investigation_id)
        .await?;
    let relation_models = RelationRepo::new(&state.db)
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

#[tauri::command]
pub async fn export_json_canvas(
    state: State<'_, AppState>,
    investigation_id: String,
) -> Result<String, AppError> {
    let (nodes, relations) = load_graph_data(&state, &investigation_id).await?;
    Ok(serde_json::to_string_pretty(&to_json_canvas(
        &nodes, &relations,
    ))?)
}

#[tauri::command]
pub async fn export_stix(
    state: State<'_, AppState>,
    investigation_id: String,
) -> Result<String, AppError> {
    let (nodes, relations) = load_graph_data(&state, &investigation_id).await?;
    Ok(serde_json::to_string_pretty(&to_stix_bundle(
        &nodes, &relations,
    ))?)
}

#[tauri::command]
pub async fn export_attack_flow(
    state: State<'_, AppState>,
    investigation_id: String,
) -> Result<String, AppError> {
    let (nodes, relations) = load_graph_data(&state, &investigation_id).await?;
    Ok(serde_json::to_string_pretty(&to_attack_flow(
        &nodes, &relations,
    ))?)
}

#[tauri::command]
pub async fn export_report(
    state: State<'_, AppState>,
    investigation_id: String,
    config: ReportConfig,
) -> Result<String, AppError> {
    let (nodes, relations) = load_graph_data(&state, &investigation_id).await?;
    Ok(generate_html_report(&nodes, &relations, &config))
}
