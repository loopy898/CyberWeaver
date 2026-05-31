//! Tauri commands for relation/edge operations.

use std::collections::HashSet;

use tauri::State;

use crate::db::entities::relation;
use crate::db::repositories::{CreateRelationData, RelationRepo};
use crate::models::domain::{RelationData, RelationType};
use crate::state::AppState;

fn parse_relation_type(value: &str) -> Option<RelationType> {
    serde_json::from_str(&format!("\"{value}\""))
        .ok()
        .or_else(|| match value {
            "ConnectsTo" => Some(RelationType::ConnectsTo),
            "ResolvesTo" => Some(RelationType::ResolvesTo),
            "Creates" => Some(RelationType::Creates),
            "BelongsTo" => Some(RelationType::BelongsTo),
            "Uses" => Some(RelationType::Uses),
            "Targets" => Some(RelationType::Targets),
            "Contains" => Some(RelationType::Contains),
            _ => None,
        })
}

/// Convert a SeaORM relation model to our domain `RelationData` transfer struct.
fn model_to_relation_data(r: relation::Model) -> RelationData {
    let relation_type = parse_relation_type(&r.relation_type).unwrap_or(RelationType::ConnectsTo);

    RelationData {
        id: r.id,
        relation_type,
        source_node_id: r.source_node_id,
        target_node_id: r.target_node_id,
        label: r.label,
        confidence: r.confidence,
        first_seen: r.first_seen,
        last_seen: r.last_seen,
        investigation_id: r.investigation_id,
    }
}

/// List all relations belonging to an investigation.
#[tauri::command]
pub async fn get_relations(
    investigation_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<RelationData>, String> {
    let repo = RelationRepo::new(&state.db);
    let relations = repo
        .find_by_investigation(&investigation_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(relations.into_iter().map(model_to_relation_data).collect())
}

/// Fetch both outgoing and incoming relations for a single node.
///
/// Duplicate relations (e.g. a self-loop) are returned only once.
#[tauri::command]
pub async fn get_node_relations(
    node_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<RelationData>, String> {
    let repo = RelationRepo::new(&state.db);
    let outgoing = repo
        .find_outgoing(&node_id)
        .await
        .map_err(|e| e.to_string())?;
    let incoming = repo
        .find_incoming(&node_id)
        .await
        .map_err(|e| e.to_string())?;

    let mut seen = HashSet::new();
    let mut results = Vec::new();
    for r in outgoing.into_iter().chain(incoming) {
        if seen.insert(r.id.clone()) {
            results.push(model_to_relation_data(r));
        }
    }
    Ok(results)
}

/// Create a new relation (edge) between two nodes.
#[tauri::command]
pub async fn create_relation(
    investigation_id: String,
    relation_type: String,
    source_node_id: String,
    target_node_id: String,
    label: String,
    confidence: f32,
    state: State<'_, AppState>,
) -> Result<RelationData, String> {
    // Validate relation_type by converting to our enum.
    let _rt: RelationType = serde_json::from_str(&format!("\"{}\"", relation_type))
        .map_err(|_| format!("invalid relation_type: {relation_type}"))?;

    let data = CreateRelationData {
        investigation_id,
        relation_type,
        source_node_id,
        target_node_id,
        label,
        confidence,
        first_seen: None,
        last_seen: None,
        properties: "{}".to_string(),
    };

    let repo = RelationRepo::new(&state.db);
    let relation = repo.create(data).await.map_err(|e| e.to_string())?;
    Ok(model_to_relation_data(relation))
}

/// Delete a single relation by id.
#[tauri::command]
pub async fn delete_relation(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let repo = RelationRepo::new(&state.db);
    repo.delete(&id).await.map_err(|e| e.to_string())?;
    Ok(())
}
