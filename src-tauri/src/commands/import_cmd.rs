//! Tauri commands for importing external data (STIX, Canvas, AFB).

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::repositories::{CreateNodeData, CreateRelationData, NodeRepo, RelationRepo};
use crate::error::AppError;
use crate::models::domain::{NodeData, RelationData};
use crate::services::import::afb_importer::import_afb_json;
use crate::services::import::canvas_importer::import_canvas_json;
use crate::services::import::stix_importer::import_stix_json;
use crate::state::AppState;

type ImportFn = fn(&str) -> Result<(Vec<NodeData>, Vec<RelationData>), AppError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub nodes_imported: usize,
    pub relations_imported: usize,
    pub errors: Vec<String>,
}

fn enum_to_db_string<T: Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_value(value)?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| AppError::Internal("expected enum to serialize as string".to_string()))
}

fn to_create_node_data(investigation_id: &str, node: NodeData) -> Result<CreateNodeData, AppError> {
    Ok(CreateNodeData {
        fixed_id: node.id,
        investigation_id: investigation_id.to_string(),
        node_type: enum_to_db_string(&node.node_type)?,
        label: node.label,
        description: node.description,
        confidence: node.confidence,
        properties: serde_json::to_string(&node.properties)?,
        pos_x: node.pos_x,
        pos_y: node.pos_y,
    })
}

fn to_create_relation_data(
    investigation_id: &str,
    relation: RelationData,
) -> Result<CreateRelationData, AppError> {
    Ok(CreateRelationData {
        investigation_id: investigation_id.to_string(),
        relation_type: enum_to_db_string(&relation.relation_type)?,
        source_node_id: relation.source_node_id,
        target_node_id: relation.target_node_id,
        label: relation.label,
        confidence: relation.confidence,
        first_seen: relation.first_seen,
        last_seen: relation.last_seen,
        properties: "{}".to_string(),
    })
}

async fn import_graph_data(
    state: State<'_, AppState>,
    investigation_id: String,
    json: String,
    import_fn: ImportFn,
) -> Result<ImportResult, AppError> {
    let (nodes, relations) = import_fn(&json)?;
    let node_repo = NodeRepo::new(&state.db);
    let relation_repo = RelationRepo::new(&state.db);

    let mut result = ImportResult {
        nodes_imported: 0,
        relations_imported: 0,
        errors: Vec::new(),
    };

    for node in nodes {
        let node_id = node.id.clone();
        let data = match to_create_node_data(&investigation_id, node) {
            Ok(data) => data,
            Err(err) => {
                result
                    .errors
                    .push(format!("Failed to convert node {node_id}: {err}"));
                continue;
            }
        };

        // Preserve imported IDs so imported relations reference real node records.
        match node_repo.upsert_batch(vec![data]).await {
            Ok(_) => result.nodes_imported += 1,
            Err(err) => result
                .errors
                .push(format!("Failed to insert node {node_id}: {err}")),
        }
    }

    for relation in relations {
        let source_node_id = relation.source_node_id.clone();
        let target_node_id = relation.target_node_id.clone();
        let data = match to_create_relation_data(&investigation_id, relation) {
            Ok(data) => data,
            Err(err) => {
                result.errors.push(format!(
                    "Failed to convert relation {source_node_id} -> {target_node_id}: {err}"
                ));
                continue;
            }
        };

        match relation_repo.create(data).await {
            Ok(_) => result.relations_imported += 1,
            Err(err) => result.errors.push(format!(
                "Failed to insert relation {source_node_id} -> {target_node_id}: {err}"
            )),
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn import_json_canvas(
    state: State<'_, AppState>,
    investigation_id: String,
    json: String,
) -> Result<ImportResult, AppError> {
    import_graph_data(state, investigation_id, json, import_canvas_json).await
}

#[tauri::command]
pub async fn import_stix(
    state: State<'_, AppState>,
    investigation_id: String,
    json: String,
) -> Result<ImportResult, AppError> {
    import_graph_data(state, investigation_id, json, import_stix_json).await
}

#[tauri::command]
pub async fn import_attack_flow(
    state: State<'_, AppState>,
    investigation_id: String,
    json: String,
) -> Result<ImportResult, AppError> {
    import_graph_data(state, investigation_id, json, import_afb_json).await
}
