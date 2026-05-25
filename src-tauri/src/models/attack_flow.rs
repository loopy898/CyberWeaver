//! Attack Flow (OASIS) data model types for import/export.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::domain::{
    AssetProps, MalwareProps, NodeData, NodeType, ProcessProps, RelationData, RelationType,
    ThreatActorProps, TtpProps, TypeSpecificProps,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AfbBundle {
    pub schema_version: String,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub actions: Vec<AfbAction>,
    #[serde(default)]
    pub assets: Vec<AfbAsset>,
    #[serde(default)]
    pub relationships: Vec<AfbRelationship>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfbAction {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub action_type: String,
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfbAsset {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub asset_type: Option<String>,
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfbRelationship {
    pub id: String,
    pub source_ref: String,
    pub target_ref: String,
    #[serde(default)]
    pub relationship_type: String,
    #[serde(default)]
    pub description: Option<String>,
}

pub fn parse_attack_flow(
    bundle: AfbBundle,
) -> Result<(Vec<NodeData>, Vec<RelationData>), AppError> {
    if bundle.schema_version.trim().is_empty() {
        return Err(AppError::Import(
            "AFB schema_version is required".to_string(),
        ));
    }

    let mut nodes = Vec::new();
    let mut relations = Vec::new();

    for (index, action) in bundle.actions.into_iter().enumerate() {
        nodes.push(action_to_node(action, index));
    }

    let asset_offset = nodes.len();
    for (index, asset) in bundle.assets.into_iter().enumerate() {
        nodes.push(asset_to_node(asset, asset_offset + index));
    }

    for relationship in bundle.relationships {
        relations.push(relationship.into_relation());
    }

    Ok((nodes, relations))
}

pub fn to_attack_flow(nodes: &[NodeData], relations: &[RelationData]) -> AfbBundle {
    let mut actions = Vec::new();
    let mut assets = Vec::new();

    for node in nodes {
        match node.node_type {
            NodeType::Asset | NodeType::IpAddress | NodeType::Domain | NodeType::FileHash => {
                assets.push(node_to_asset(node));
            }
            _ => actions.push(node_to_action(node)),
        }
    }

    let relationships = relations.iter().map(node_relation_to_afb).collect();

    AfbBundle {
        schema_version: "1.0".to_string(),
        id: Uuid::new_v4().to_string(),
        name: "CyberWeaver Attack Flow".to_string(),
        description: None,
        actions,
        assets,
        relationships,
    }
}

fn action_to_node(action: AfbAction, index: usize) -> NodeData {
    let (pos_x, pos_y) = match (action.x, action.y) {
        (Some(x), Some(y)) => (x, y),
        _ => grid_position(index),
    };
    let node_type = map_action_type_to_node_type(&action.action_type);

    NodeData {
        id: if action.id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            action.id
        },
        node_type: node_type.clone(),
        label: action.name.clone(),
        description: action.description.unwrap_or_default(),
        confidence: 1.0,
        properties: default_action_props(&node_type, &action.name, &action.action_type),
        pos_x,
        pos_y,
        investigation_id: String::new(),
        created_at: None,
        updated_at: None,
    }
}

fn asset_to_node(asset: AfbAsset, index: usize) -> NodeData {
    let (pos_x, pos_y) = match (asset.x, asset.y) {
        (Some(x), Some(y)) => (x, y),
        _ => grid_position(index),
    };

    NodeData {
        id: if asset.id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            asset.id
        },
        node_type: NodeType::Asset,
        label: asset.name.clone(),
        description: asset.description.unwrap_or_default(),
        confidence: 1.0,
        properties: TypeSpecificProps::Asset(AssetProps {
            hostname: asset.name,
            os: None,
            ip_addresses: Vec::new(),
            owner: None,
            criticality: asset.asset_type,
        }),
        pos_x,
        pos_y,
        investigation_id: String::new(),
        created_at: None,
        updated_at: None,
    }
}

impl AfbRelationship {
    fn into_relation(self) -> RelationData {
        RelationData {
            id: if self.id.is_empty() {
                Uuid::new_v4().to_string()
            } else {
                self.id
            },
            relation_type: map_afb_relationship_type(&self.relationship_type),
            source_node_id: self.source_ref,
            target_node_id: self.target_ref,
            label: self.relationship_type,
            confidence: 1.0,
            first_seen: None,
            last_seen: None,
            investigation_id: String::new(),
        }
    }
}

fn node_to_action(node: &NodeData) -> AfbAction {
    AfbAction {
        id: node.id.clone(),
        name: node.label.clone(),
        description: Some(node.description.clone()).filter(|value| !value.is_empty()),
        action_type: map_node_type_to_action_type(&node.node_type).to_string(),
        x: Some(node.pos_x),
        y: Some(node.pos_y),
    }
}

fn node_to_asset(node: &NodeData) -> AfbAsset {
    let asset_type = match &node.properties {
        TypeSpecificProps::Asset(props) => props.criticality.clone(),
        _ => Some(node.node_type.display_name().to_string()),
    };

    AfbAsset {
        id: node.id.clone(),
        name: node.label.clone(),
        description: Some(node.description.clone()).filter(|value| !value.is_empty()),
        asset_type,
        x: Some(node.pos_x),
        y: Some(node.pos_y),
    }
}

fn node_relation_to_afb(relation: &RelationData) -> AfbRelationship {
    AfbRelationship {
        id: relation.id.clone(),
        source_ref: relation.source_node_id.clone(),
        target_ref: relation.target_node_id.clone(),
        relationship_type: if relation.label.trim().is_empty() {
            map_relation_type_to_afb(&relation.relation_type).to_string()
        } else {
            relation.label.clone()
        },
        description: None,
    }
}

fn map_action_type_to_node_type(action_type: &str) -> NodeType {
    match action_type {
        "malware" | "payload" => NodeType::Malware,
        "threat-actor" | "operator" | "identity" => NodeType::ThreatActor,
        "technique" | "ttp" | "attack-pattern" => NodeType::Ttp,
        "process" | "execution" => NodeType::Process,
        _ => NodeType::Ttp,
    }
}

fn map_node_type_to_action_type(node_type: &NodeType) -> &'static str {
    match node_type {
        NodeType::Malware => "malware",
        NodeType::ThreatActor => "threat-actor",
        NodeType::Ttp => "technique",
        NodeType::Process => "process",
        NodeType::Asset => "asset",
        NodeType::IpAddress => "asset",
        NodeType::Domain => "asset",
        NodeType::FileHash => "asset",
    }
}

fn default_action_props(node_type: &NodeType, label: &str, action_type: &str) -> TypeSpecificProps {
    match node_type {
        NodeType::Malware => TypeSpecificProps::Malware(MalwareProps {
            family_name: label.to_string(),
            aliases: Vec::new(),
            malware_type: Some(action_type.to_string()).filter(|value| !value.is_empty()),
            first_seen: None,
        }),
        NodeType::ThreatActor => TypeSpecificProps::ThreatActor(ThreatActorProps {
            name: label.to_string(),
            aliases: Vec::new(),
            motivation: None,
            sophistication: None,
            targets: Vec::new(),
        }),
        NodeType::Process => TypeSpecificProps::Process(ProcessProps {
            process_name: label.to_string(),
            pid: None,
            command_line: None,
            parent_process: None,
            user: None,
        }),
        _ => TypeSpecificProps::Ttp(TtpProps {
            mitre_id: label.to_string(),
            tactic: Some(action_type.to_string()).filter(|value| !value.is_empty()),
            platform: Vec::new(),
            data_source: Vec::new(),
        }),
    }
}

fn map_afb_relationship_type(relationship_type: &str) -> RelationType {
    match relationship_type {
        "uses" => RelationType::Uses,
        "targets" => RelationType::Targets,
        "creates" => RelationType::Creates,
        "contains" => RelationType::Contains,
        "belongs-to" => RelationType::BelongsTo,
        "resolves-to" => RelationType::ResolvesTo,
        _ => RelationType::ConnectsTo,
    }
}

fn map_relation_type_to_afb(relation_type: &RelationType) -> &'static str {
    match relation_type {
        RelationType::Uses => "uses",
        RelationType::Targets => "targets",
        RelationType::Creates => "creates",
        RelationType::Contains => "contains",
        RelationType::BelongsTo => "belongs-to",
        RelationType::ResolvesTo => "resolves-to",
        RelationType::ConnectsTo => "connects-to",
    }
}

fn grid_position(index: usize) -> (f64, f64) {
    let column = (index % 4) as f64;
    let row = (index / 4) as f64;
    (column * 240.0, row * 160.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_flow_roundtrip_parse_export_parse_preserves_shape() {
        let bundle = AfbBundle {
            schema_version: "1.0".to_string(),
            id: "bundle-1".to_string(),
            name: "Test Flow".to_string(),
            description: Some("Roundtrip".to_string()),
            actions: vec![
                AfbAction {
                    id: "action-1".to_string(),
                    name: "Initial Access".to_string(),
                    description: Some("Phishing".to_string()),
                    action_type: "technique".to_string(),
                    x: Some(10.0),
                    y: Some(20.0),
                },
                AfbAction {
                    id: "action-2".to_string(),
                    name: "Payload".to_string(),
                    description: Some("Drop malware".to_string()),
                    action_type: "malware".to_string(),
                    x: Some(30.0),
                    y: Some(40.0),
                },
            ],
            assets: vec![AfbAsset {
                id: "asset-1".to_string(),
                name: "Workstation".to_string(),
                description: Some("Victim host".to_string()),
                asset_type: Some("critical".to_string()),
                x: Some(50.0),
                y: Some(60.0),
            }],
            relationships: vec![
                AfbRelationship {
                    id: "rel-1".to_string(),
                    source_ref: "action-1".to_string(),
                    target_ref: "action-2".to_string(),
                    relationship_type: "uses".to_string(),
                    description: Some("operator uses malware".to_string()),
                },
                AfbRelationship {
                    id: "rel-2".to_string(),
                    source_ref: "action-2".to_string(),
                    target_ref: "asset-1".to_string(),
                    relationship_type: "targets".to_string(),
                    description: Some("malware targets host".to_string()),
                },
            ],
        };

        let (nodes, relations) = parse_attack_flow(bundle).expect("initial parse should succeed");
        let exported = to_attack_flow(&nodes, &relations);
        let (roundtrip_nodes, roundtrip_relations) =
            parse_attack_flow(exported).expect("roundtrip parse should succeed");

        assert_eq!(roundtrip_nodes.len(), nodes.len());
        assert_eq!(roundtrip_relations.len(), relations.len());

        let original_ids: std::collections::HashSet<_> =
            nodes.iter().map(|node| node.id.as_str()).collect();
        let roundtrip_ids: std::collections::HashSet<_> = roundtrip_nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect();
        assert_eq!(roundtrip_ids, original_ids);

        let original_edges: std::collections::HashSet<_> = relations
            .iter()
            .map(|relation| {
                (
                    relation.id.as_str(),
                    relation.source_node_id.as_str(),
                    relation.target_node_id.as_str(),
                    &relation.relation_type,
                )
            })
            .collect();
        let roundtrip_edges: std::collections::HashSet<_> = roundtrip_relations
            .iter()
            .map(|relation| {
                (
                    relation.id.as_str(),
                    relation.source_node_id.as_str(),
                    relation.target_node_id.as_str(),
                    &relation.relation_type,
                )
            })
            .collect();
        assert_eq!(roundtrip_edges, original_edges);
    }
}
