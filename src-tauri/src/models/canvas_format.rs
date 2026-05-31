//! JSON Canvas format ↔ domain model mapping.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::domain::{
    AssetProps, DomainProps, FileHashProps, HashAlgorithm, IpAddressProps, MalwareProps, NodeData,
    NodeType, ProcessProps, RelationData, RelationType, ThreatActorProps, TtpProps,
    TypeSpecificProps,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JsonCanvas {
    #[serde(default)]
    pub nodes: Vec<CanvasNode>,
    #[serde(default)]
    pub edges: Vec<CanvasEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default)]
    pub width: f64,
    #[serde(default)]
    pub height: f64,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasEdge {
    pub id: String,
    #[serde(alias = "fromNode", alias = "from_node")]
    pub from_node: String,
    #[serde(alias = "toNode", alias = "to_node")]
    pub to_node: String,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CanvasTextPayload {
    node_type: NodeType,
    label: String,
}

pub fn parse_json_canvas(
    canvas: JsonCanvas,
) -> Result<(Vec<NodeData>, Vec<RelationData>), AppError> {
    let mut nodes = Vec::new();
    let mut relations = Vec::new();

    for node in canvas.nodes {
        if node.node_type != "text" {
            continue;
        }

        let text = node.text.unwrap_or_default();
        let (parsed_type, parsed_label) = parse_canvas_text_payload(&text);

        nodes.push(NodeData {
            id: if node.id.is_empty() {
                Uuid::new_v4().to_string()
            } else {
                node.id
            },
            node_type: parsed_type.clone(),
            label: parsed_label.clone(),
            description: String::new(),
            confidence: 1.0,
            properties: default_props_for_node_type(parsed_type, &parsed_label),
            pos_x: node.x,
            pos_y: node.y,
            investigation_id: String::new(),
            created_at: None,
            updated_at: None,
        });
    }

    for edge in canvas.edges {
        relations.push(RelationData {
            id: if edge.id.is_empty() {
                Uuid::new_v4().to_string()
            } else {
                edge.id
            },
            relation_type: RelationType::ConnectsTo,
            source_node_id: edge.from_node,
            target_node_id: edge.to_node,
            label: edge.label.unwrap_or_default(),
            confidence: 1.0,
            first_seen: None,
            last_seen: None,
            investigation_id: String::new(),
        });
    }

    Ok((nodes, relations))
}

pub fn to_json_canvas(nodes: &[NodeData], relations: &[RelationData]) -> JsonCanvas {
    let nodes = nodes
        .iter()
        .map(|node| {
            let text = serde_json::to_string(&CanvasTextPayload {
                node_type: node.node_type.clone(),
                label: node.label.clone(),
            })
            .unwrap_or_else(|_| node.label.clone());

            CanvasNode {
                id: node.id.clone(),
                node_type: "text".to_string(),
                x: node.pos_x,
                y: node.pos_y,
                width: 220.0,
                height: 72.0,
                color: None,
                text: Some(text),
                file: None,
            }
        })
        .collect();

    let edges = relations
        .iter()
        .map(|relation| CanvasEdge {
            id: relation.id.clone(),
            from_node: relation.source_node_id.clone(),
            to_node: relation.target_node_id.clone(),
            label: Some(relation.label.clone()),
        })
        .collect();

    JsonCanvas { nodes, edges }
}

fn parse_canvas_text_payload(text: &str) -> (NodeType, String) {
    if let Ok(payload) = serde_json::from_str::<CanvasTextPayload>(text) {
        return (payload.node_type, payload.label);
    }

    (NodeType::Asset, text.to_string())
}

fn default_props_for_node_type(node_type: NodeType, label: &str) -> TypeSpecificProps {
    match node_type {
        NodeType::IpAddress => TypeSpecificProps::IpAddress(IpAddressProps {
            address: label.to_string(),
            version: None,
            geo_location: None,
            asn: None,
            isp: None,
            reputation: None,
        }),
        NodeType::Domain => TypeSpecificProps::Domain(DomainProps {
            domain: label.to_string(),
            registrar: None,
            creation_date: None,
        }),
        NodeType::FileHash => TypeSpecificProps::FileHash(FileHashProps {
            hash_value: label.to_string(),
            algorithm: HashAlgorithm::MD5,
            file_name: None,
            file_size: None,
            file_type: None,
            malware_classification: None,
        }),
        NodeType::Process => TypeSpecificProps::Process(ProcessProps {
            process_name: label.to_string(),
            pid: None,
            command_line: None,
            parent_process: None,
            user: None,
        }),
        NodeType::Malware => TypeSpecificProps::Malware(MalwareProps {
            family_name: label.to_string(),
            aliases: Vec::new(),
            malware_type: None,
            first_seen: None,
        }),
        NodeType::Ttp => TypeSpecificProps::Ttp(TtpProps {
            mitre_id: label.to_string(),
            tactic: None,
            platform: Vec::new(),
            data_source: Vec::new(),
        }),
        NodeType::ThreatActor => TypeSpecificProps::ThreatActor(ThreatActorProps {
            name: label.to_string(),
            aliases: Vec::new(),
            motivation: None,
            sophistication: None,
            targets: Vec::new(),
        }),
        NodeType::Asset => TypeSpecificProps::Asset(AssetProps {
            hostname: label.to_string(),
            os: None,
            ip_addresses: Vec::new(),
            owner: None,
            criticality: None,
        }),
    }
}
