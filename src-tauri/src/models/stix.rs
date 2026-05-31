//! STIX 2.1 bundle types and converters.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::domain::{
    AssetProps, DomainProps, FileHashProps, HashAlgorithm, IpAddressProps, MalwareProps, NodeData,
    NodeType, RelationData, RelationType, ThreatActorProps, TtpProps, TypeSpecificProps,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StixBundle {
    #[serde(rename = "type")]
    pub bundle_type: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub objects: Vec<Value>,
}

pub fn parse_stix_bundle(
    bundle: StixBundle,
) -> Result<(Vec<NodeData>, Vec<RelationData>), AppError> {
    if bundle.bundle_type != "bundle" {
        return Err(AppError::Import(format!(
            "expected STIX bundle type 'bundle', got '{}'",
            bundle.bundle_type
        )));
    }

    let mut nodes = Vec::new();
    let mut relations = Vec::new();
    let mut id_map = HashMap::new();
    let mut node_index = 0usize;

    for object in &bundle.objects {
        let object_type = object
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::Import("STIX object missing type".to_string()))?;

        if is_supported_node_type(object_type) {
            let object_map = object.as_object().ok_or_else(|| {
                AppError::Import("STIX node object must be a JSON object".to_string())
            })?;
            let original_id = required_str(object_map, "id")?.to_string();
            let node = parse_node_object(object_map, node_index)?;
            id_map.insert(original_id, node.id.clone());
            nodes.push(node);
            node_index += 1;
        }
    }

    for object in &bundle.objects {
        let object_type = object
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::Import("STIX object missing type".to_string()))?;

        match object_type {
            "relationship" => {
                let object_map = object.as_object().ok_or_else(|| {
                    AppError::Import("STIX relationship object must be a JSON object".to_string())
                })?;
                relations.push(parse_relationship_object(object_map, &id_map)?);
            }
            "observed-data" => {
                let object_map = object.as_object().ok_or_else(|| {
                    AppError::Import("STIX observed-data object must be a JSON object".to_string())
                })?;
                relations.extend(parse_observed_data_relationships(object_map, &id_map));
            }
            _ => {}
        }
    }

    Ok((nodes, relations))
}

pub fn to_stix_bundle(nodes: &[NodeData], relations: &[RelationData]) -> StixBundle {
    let mut id_map: HashMap<String, String> = HashMap::new();
    let mut objects = Vec::new();

    for node in nodes {
        if let Some((stix_id, object)) = node_to_stix_object(node) {
            id_map.insert(node.id.clone(), stix_id);
            objects.push(object);
        }
    }

    for relation in relations {
        let Some(source_ref) = id_map.get(relation.source_node_id.as_str()) else {
            continue;
        };
        let Some(target_ref) = id_map.get(relation.target_node_id.as_str()) else {
            continue;
        };

        let relationship_type = relation_type_name(&relation.relation_type);
        let relationship_id = deterministic_stix_id(
            "relationship",
            &format!(
                "{}:{}:{}:{}",
                relation.source_node_id, relation.target_node_id, relationship_type, relation.label
            ),
        );

        objects.push(json!({
            "type": "relationship",
            "spec_version": "2.1",
            "id": relationship_id,
            "created": format_stix_timestamp(relation.first_seen.as_deref()),
            "modified": format_stix_timestamp(
                relation.last_seen.as_deref().or(relation.first_seen.as_deref())
            ),
            "relationship_type": relationship_type,
            "source_ref": source_ref,
            "target_ref": target_ref,
        }));
    }

    let bundle_seed = objects
        .iter()
        .filter_map(|object| object.get("id").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("|");

    StixBundle {
        bundle_type: "bundle".to_string(),
        id: deterministic_stix_id("bundle", &bundle_seed),
        objects,
    }
}

fn is_supported_node_type(object_type: &str) -> bool {
    matches!(
        object_type,
        "indicator"
            | "malware"
            | "threat-actor"
            | "attack-pattern"
            | "infrastructure"
            | "identity"
            | "observed-data"
    )
}

fn node_to_stix_object(node: &NodeData) -> Option<(String, Value)> {
    let created = format_stix_timestamp(node.created_at.as_deref());
    let modified = format_stix_timestamp(node.updated_at.as_deref().or(node.created_at.as_deref()));

    match (&node.node_type, &node.properties) {
        (NodeType::IpAddress, TypeSpecificProps::IpAddress(props)) => {
            let field = if props.version.as_deref() == Some("ipv6") {
                "ipv6-addr:value"
            } else {
                "ipv4-addr:value"
            };
            let id = deterministic_stix_id("indicator", &node.id);
            Some((
                id.clone(),
                json!({
                    "type": "indicator",
                    "spec_version": "2.1",
                    "id": id,
                    "created": created,
                    "modified": modified,
                    "name": node.label,
                    "pattern": format!("[{field} = '{}']", props.address),
                }),
            ))
        }
        (NodeType::Domain, TypeSpecificProps::Domain(props)) => {
            let id = deterministic_stix_id("indicator", &node.id);
            Some((
                id.clone(),
                json!({
                    "type": "indicator",
                    "spec_version": "2.1",
                    "id": id,
                    "created": created,
                    "modified": modified,
                    "name": node.label,
                    "pattern": format!("[domain-name:value = '{}']", props.domain),
                }),
            ))
        }
        (NodeType::FileHash, TypeSpecificProps::FileHash(props)) => {
            let id = deterministic_stix_id("indicator", &node.id);
            Some((
                id.clone(),
                json!({
                    "type": "indicator",
                    "spec_version": "2.1",
                    "id": id,
                    "created": created,
                    "modified": modified,
                    "name": node.label,
                    "pattern": format!(
                        "[file:hashes.{} = '{}']",
                        hash_algorithm_name(&props.algorithm),
                        props.hash_value
                    ),
                }),
            ))
        }
        (NodeType::Malware, _) => {
            let id = deterministic_stix_id("malware", &node.id);
            Some((
                id.clone(),
                json!({
                    "type": "malware",
                    "spec_version": "2.1",
                    "id": id,
                    "created": created,
                    "modified": modified,
                    "name": node.label,
                }),
            ))
        }
        (NodeType::ThreatActor, _) => {
            let id = deterministic_stix_id("threat-actor", &node.id);
            Some((
                id.clone(),
                json!({
                    "type": "threat-actor",
                    "spec_version": "2.1",
                    "id": id,
                    "created": created,
                    "modified": modified,
                    "name": node.label,
                }),
            ))
        }
        (NodeType::Ttp, TypeSpecificProps::Ttp(props)) => {
            let id = deterministic_stix_id("attack-pattern", &node.id);
            Some((
                id.clone(),
                json!({
                    "type": "attack-pattern",
                    "spec_version": "2.1",
                    "id": id,
                    "created": created,
                    "modified": modified,
                    "name": node.label,
                    "external_references": [{
                        "source_name": "mitre",
                        "external_id": props.mitre_id,
                    }],
                }),
            ))
        }
        (NodeType::Asset, TypeSpecificProps::Asset(props)) => {
            let id = deterministic_stix_id("infrastructure", &node.id);
            Some((
                id.clone(),
                json!({
                    "type": "infrastructure",
                    "spec_version": "2.1",
                    "id": id,
                    "created": created,
                    "modified": modified,
                    "name": props.hostname,
                }),
            ))
        }
        (NodeType::Process, _) => None,
        _ => None,
    }
}

fn parse_node_object(object_map: &Map<String, Value>, index: usize) -> Result<NodeData, AppError> {
    let object_type = required_str(object_map, "type")?;
    let label = preferred_label(object_map);
    let description = optional_str(object_map, "description")
        .unwrap_or_default()
        .to_string();
    let confidence = object_map
        .get("confidence")
        .and_then(Value::as_f64)
        .map(|value| (value as f32 / 100.0).clamp(0.0, 1.0))
        .unwrap_or(1.0);
    let (pos_x, pos_y) = grid_position(index);

    let (node_type, properties) = match object_type {
        "indicator" => parse_indicator_props(object_map, &label),
        "malware" => parse_malware_props(object_map, &label),
        "threat-actor" | "identity" => parse_threat_actor_props(object_map, &label),
        "attack-pattern" => parse_ttp_props(object_map, &label),
        "infrastructure" | "observed-data" => parse_asset_props(object_map, &label),
        other => {
            return Err(AppError::Import(format!(
                "unsupported STIX object type for node conversion: {other}"
            )));
        }
    };

    Ok(NodeData {
        id: Uuid::new_v4().to_string(),
        node_type,
        label,
        description,
        confidence,
        properties,
        pos_x,
        pos_y,
        investigation_id: String::new(),
        created_at: optional_str(object_map, "created")
            .or_else(|| optional_str(object_map, "first_observed"))
            .map(str::to_string),
        updated_at: optional_str(object_map, "modified")
            .or_else(|| optional_str(object_map, "last_observed"))
            .map(str::to_string),
    })
}

fn parse_indicator_props(
    object_map: &Map<String, Value>,
    label: &str,
) -> (NodeType, TypeSpecificProps) {
    let pattern = optional_str(object_map, "pattern").unwrap_or_default();

    if let Some(value) = extract_indicator_literal(pattern, "ipv4-addr:value") {
        return (
            NodeType::IpAddress,
            TypeSpecificProps::IpAddress(IpAddressProps {
                address: value.to_string(),
                version: Some("ipv4".to_string()),
                geo_location: None,
                asn: None,
                isp: None,
                reputation: None,
            }),
        );
    }

    if let Some(value) = extract_indicator_literal(pattern, "ipv6-addr:value") {
        return (
            NodeType::IpAddress,
            TypeSpecificProps::IpAddress(IpAddressProps {
                address: value.to_string(),
                version: Some("ipv6".to_string()),
                geo_location: None,
                asn: None,
                isp: None,
                reputation: None,
            }),
        );
    }

    if let Some(value) = extract_indicator_literal(pattern, "domain-name:value") {
        return (
            NodeType::Domain,
            TypeSpecificProps::Domain(DomainProps {
                domain: value.to_string(),
                registrar: None,
                creation_date: optional_str(object_map, "valid_from").map(str::to_string),
            }),
        );
    }

    if let Some(value) = extract_file_hash(pattern) {
        return (
            NodeType::FileHash,
            TypeSpecificProps::FileHash(FileHashProps {
                hash_value: value.to_string(),
                algorithm: extract_hash_algorithm(pattern),
                file_name: Some(label.to_string()).filter(|name| !name.is_empty()),
                file_size: None,
                file_type: None,
                malware_classification: object_map
                    .get("indicator_types")
                    .and_then(Value::as_array)
                    .and_then(|items| items.first())
                    .and_then(Value::as_str)
                    .map(str::to_string),
            }),
        );
    }

    (
        NodeType::Asset,
        TypeSpecificProps::Asset(AssetProps {
            hostname: label.to_string(),
            os: None,
            ip_addresses: Vec::new(),
            owner: None,
            criticality: None,
        }),
    )
}

fn parse_malware_props(
    object_map: &Map<String, Value>,
    label: &str,
) -> (NodeType, TypeSpecificProps) {
    (
        NodeType::Malware,
        TypeSpecificProps::Malware(MalwareProps {
            family_name: label.to_string(),
            aliases: str_array(object_map, "aliases"),
            malware_type: object_map
                .get("malware_types")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(Value::as_str)
                .map(str::to_string),
            first_seen: optional_str(object_map, "first_seen").map(str::to_string),
        }),
    )
}

fn parse_threat_actor_props(
    object_map: &Map<String, Value>,
    label: &str,
) -> (NodeType, TypeSpecificProps) {
    (
        NodeType::ThreatActor,
        TypeSpecificProps::ThreatActor(ThreatActorProps {
            name: label.to_string(),
            aliases: str_array(object_map, "aliases"),
            motivation: object_map
                .get("primary_motivation")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| optional_str(object_map, "identity_class").map(str::to_string)),
            sophistication: optional_str(object_map, "sophistication").map(str::to_string),
            targets: str_array(object_map, "goals"),
        }),
    )
}

fn parse_ttp_props(object_map: &Map<String, Value>, label: &str) -> (NodeType, TypeSpecificProps) {
    (
        NodeType::Ttp,
        TypeSpecificProps::Ttp(TtpProps {
            mitre_id: first_external_id(object_map).unwrap_or_else(|| label.to_string()),
            tactic: kill_chain_phase(object_map),
            platform: str_array(object_map, "x_mitre_platforms"),
            data_source: str_array(object_map, "x_mitre_data_sources"),
        }),
    )
}

fn parse_asset_props(
    object_map: &Map<String, Value>,
    label: &str,
) -> (NodeType, TypeSpecificProps) {
    let hostname = if label.is_empty() {
        "Unnamed Asset".to_string()
    } else {
        label.to_string()
    };

    (
        NodeType::Asset,
        TypeSpecificProps::Asset(AssetProps {
            hostname,
            os: None,
            ip_addresses: Vec::new(),
            owner: optional_str(object_map, "created_by_ref").map(str::to_string),
            criticality: object_map
                .get("infrastructure_types")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    object_map
                        .get("labels")
                        .and_then(Value::as_array)
                        .and_then(|items| items.first())
                        .and_then(Value::as_str)
                        .map(str::to_string)
                }),
        }),
    )
}

fn parse_relationship_object(
    object_map: &Map<String, Value>,
    id_map: &HashMap<String, String>,
) -> Result<RelationData, AppError> {
    let source_ref = required_str(object_map, "source_ref")?;
    let target_ref = required_str(object_map, "target_ref")?;

    let source_node_id = id_map.get(source_ref).cloned().ok_or_else(|| {
        AppError::Import(format!("relationship source_ref not found: {source_ref}"))
    })?;
    let target_node_id = id_map.get(target_ref).cloned().ok_or_else(|| {
        AppError::Import(format!("relationship target_ref not found: {target_ref}"))
    })?;

    let relationship_type = required_str(object_map, "relationship_type")?;

    Ok(RelationData {
        id: Uuid::new_v4().to_string(),
        relation_type: map_relationship_type(relationship_type),
        source_node_id,
        target_node_id,
        label: relationship_type.to_string(),
        confidence: object_map
            .get("confidence")
            .and_then(Value::as_f64)
            .map(|value| (value as f32 / 100.0).clamp(0.0, 1.0))
            .unwrap_or(1.0),
        first_seen: optional_str(object_map, "start_time").map(str::to_string),
        last_seen: optional_str(object_map, "stop_time").map(str::to_string),
        investigation_id: String::new(),
    })
}

fn parse_observed_data_relationships(
    object_map: &Map<String, Value>,
    id_map: &HashMap<String, String>,
) -> Vec<RelationData> {
    let source_ref = match required_str(object_map, "id") {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let source_node_id = match id_map.get(source_ref) {
        Some(value) => value.clone(),
        None => return Vec::new(),
    };

    let mut relations = Vec::new();

    if let Some(object_refs) = object_map.get("object_refs").and_then(Value::as_array) {
        for target_ref in object_refs.iter().filter_map(Value::as_str) {
            if let Some(target_node_id) = id_map.get(target_ref) {
                relations.push(RelationData {
                    id: Uuid::new_v4().to_string(),
                    relation_type: RelationType::Contains,
                    source_node_id: source_node_id.clone(),
                    target_node_id: target_node_id.clone(),
                    label: "contains".to_string(),
                    confidence: 1.0,
                    first_seen: optional_str(object_map, "first_observed").map(str::to_string),
                    last_seen: optional_str(object_map, "last_observed").map(str::to_string),
                    investigation_id: String::new(),
                });
            }
        }
    }

    relations
}

fn required_str<'a>(map: &'a Map<String, Value>, key: &str) -> Result<&'a str, AppError> {
    map.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Import(format!("STIX object missing string field '{key}'")))
}

fn optional_str<'a>(map: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    map.get(key).and_then(Value::as_str)
}

fn str_array(map: &Map<String, Value>, key: &str) -> Vec<String> {
    map.get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn preferred_label(map: &Map<String, Value>) -> String {
    optional_str(map, "name")
        .or_else(|| optional_str(map, "value"))
        .unwrap_or("")
        .to_string()
}

fn extract_indicator_literal<'a>(pattern: &'a str, field_name: &str) -> Option<&'a str> {
    let marker = format!("{field_name} = '");
    let start = pattern.find(&marker)? + marker.len();
    let rest = &pattern[start..];
    let end = rest.find('\'')?;
    Some(&rest[..end])
}

fn extract_file_hash(pattern: &str) -> Option<&str> {
    let marker = "file:hashes.";
    let start = pattern.find(marker)? + marker.len();
    let rest = &pattern[start..];
    let equals_index = rest.find(" = '")?;
    let value_start = equals_index + " = '".len();
    let value_rest = &rest[value_start..];
    let value_end = value_rest.find('\'')?;
    Some(&value_rest[..value_end])
}

fn extract_hash_algorithm(pattern: &str) -> HashAlgorithm {
    if pattern.contains("SHA-256") {
        HashAlgorithm::SHA256
    } else if pattern.contains("SHA-1") {
        HashAlgorithm::SHA1
    } else {
        HashAlgorithm::MD5
    }
}

fn first_external_id(map: &Map<String, Value>) -> Option<String> {
    map.get("external_references")
        .and_then(Value::as_array)
        .and_then(|references| {
            references.iter().find_map(|reference| {
                reference
                    .get("external_id")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
        })
}

fn kill_chain_phase(map: &Map<String, Value>) -> Option<String> {
    map.get("kill_chain_phases")
        .and_then(Value::as_array)
        .and_then(|phases| phases.first())
        .and_then(|phase| phase.get("phase_name"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn map_relationship_type(name: &str) -> RelationType {
    match name {
        "resolves-to" => RelationType::ResolvesTo,
        "connects-to" | "communicates-with" => RelationType::ConnectsTo,
        "creates" | "drops" => RelationType::Creates,
        "belongs-to" | "attributed-to" => RelationType::BelongsTo,
        "uses" => RelationType::Uses,
        "targets" => RelationType::Targets,
        "contains" | "consists-of" => RelationType::Contains,
        _ => RelationType::ConnectsTo,
    }
}

fn relation_type_name(relation_type: &RelationType) -> &'static str {
    match relation_type {
        RelationType::ConnectsTo => "connects-to",
        RelationType::ResolvesTo => "resolves-to",
        RelationType::Creates => "creates",
        RelationType::BelongsTo => "belongs-to",
        RelationType::Uses => "uses",
        RelationType::Targets => "targets",
        RelationType::Contains => "contains",
    }
}

fn deterministic_stix_id(object_type: &str, seed: &str) -> String {
    format!(
        "{object_type}--{}",
        Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            format!("cyberweaver:{object_type}:{seed}").as_bytes(),
        )
    )
}

fn hash_algorithm_name(algorithm: &HashAlgorithm) -> &'static str {
    match algorithm {
        HashAlgorithm::MD5 => "MD5",
        HashAlgorithm::SHA1 => "SHA-1",
        HashAlgorithm::SHA256 => "SHA-256",
    }
}

fn format_stix_timestamp(value: Option<&str>) -> String {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) if value.ends_with('Z') => value.to_string(),
        Some(value) if value.len() == 10 => format!("{value}T00:00:00.000Z"),
        Some(value) if value.len() == 19 && value.as_bytes().get(10) == Some(&b' ') => {
            format!("{}Z", value.replace(' ', "T"))
        }
        Some(value) if value.len() == 19 && value.as_bytes().get(10) == Some(&b'T') => {
            format!("{value}Z")
        }
        Some(value) => format!("{value}T00:00:00.000Z"),
        None => "1970-01-01T00:00:00.000Z".to_string(),
    }
}

fn grid_position(index: usize) -> (f64, f64) {
    let column = (index % 4) as f64;
    let row = (index / 4) as f64;
    (column * 240.0, row * 160.0)
}

#[cfg(test)]
mod tests_export {
    use super::*;

    use crate::models::domain::{
        DomainProps, FileHashProps, ProcessProps, RelationType, ThreatActorProps, TtpProps,
    };

    fn sample_node(
        id: &str,
        node_type: NodeType,
        label: &str,
        properties: TypeSpecificProps,
    ) -> NodeData {
        NodeData {
            id: id.to_string(),
            node_type,
            label: label.to_string(),
            description: String::new(),
            confidence: 0.8,
            properties,
            pos_x: 0.0,
            pos_y: 0.0,
            investigation_id: "inv-1".to_string(),
            created_at: Some("2026-05-25 12:34:56".to_string()),
            updated_at: Some("2026-05-25 13:45:00".to_string()),
        }
    }

    #[test]
    fn exports_supported_domain_nodes_to_stix_bundle() {
        let nodes = vec![
            sample_node(
                "ip-1",
                NodeType::IpAddress,
                "198.51.100.10",
                TypeSpecificProps::IpAddress(IpAddressProps {
                    address: "198.51.100.10".to_string(),
                    version: Some("ipv4".to_string()),
                    geo_location: None,
                    asn: None,
                    isp: None,
                    reputation: None,
                }),
            ),
            sample_node(
                "domain-1",
                NodeType::Domain,
                "evil.example",
                TypeSpecificProps::Domain(DomainProps {
                    domain: "evil.example".to_string(),
                    registrar: None,
                    creation_date: None,
                }),
            ),
            sample_node(
                "hash-1",
                NodeType::FileHash,
                "loader.bin",
                TypeSpecificProps::FileHash(FileHashProps {
                    hash_value: "abcd1234".to_string(),
                    algorithm: HashAlgorithm::SHA256,
                    file_name: Some("loader.bin".to_string()),
                    file_size: None,
                    file_type: None,
                    malware_classification: None,
                }),
            ),
            sample_node(
                "malware-1",
                NodeType::Malware,
                "DarkLoader",
                TypeSpecificProps::Malware(MalwareProps {
                    family_name: "DarkLoader".to_string(),
                    aliases: vec!["DL".to_string()],
                    malware_type: Some("loader".to_string()),
                    first_seen: None,
                }),
            ),
            sample_node(
                "actor-1",
                NodeType::ThreatActor,
                "APT-X",
                TypeSpecificProps::ThreatActor(ThreatActorProps {
                    name: "APT-X".to_string(),
                    aliases: vec![],
                    motivation: None,
                    sophistication: None,
                    targets: vec![],
                }),
            ),
            sample_node(
                "ttp-1",
                NodeType::Ttp,
                "Spearphishing Attachment",
                TypeSpecificProps::Ttp(TtpProps {
                    mitre_id: "T1566.001".to_string(),
                    tactic: Some("initial-access".to_string()),
                    platform: vec![],
                    data_source: vec![],
                }),
            ),
            sample_node(
                "asset-1",
                NodeType::Asset,
                "ws-01",
                TypeSpecificProps::Asset(AssetProps {
                    hostname: "ws-01".to_string(),
                    os: Some("Windows".to_string()),
                    ip_addresses: vec![],
                    owner: None,
                    criticality: None,
                }),
            ),
            sample_node(
                "proc-1",
                NodeType::Process,
                "powershell.exe",
                TypeSpecificProps::Process(ProcessProps::default()),
            ),
        ];
        let relations = vec![RelationData {
            id: "rel-1".to_string(),
            relation_type: RelationType::Uses,
            source_node_id: "actor-1".to_string(),
            target_node_id: "ttp-1".to_string(),
            label: "uses".to_string(),
            confidence: 0.9,
            first_seen: Some("2026-05-25 13:00:00".to_string()),
            last_seen: Some("2026-05-25 14:00:00".to_string()),
            investigation_id: "inv-1".to_string(),
        }];

        let bundle = to_stix_bundle(&nodes, &relations);

        assert_eq!(bundle.bundle_type, "bundle");
        assert!(bundle.id.starts_with("bundle--"));
        assert_eq!(bundle.objects.len(), 8);

        let indicator = bundle
            .objects
            .iter()
            .find(|object| object["type"] == "indicator" && object["name"] == "198.51.100.10")
            .expect("expected IP indicator");
        assert_eq!(indicator["pattern"], "[ipv4-addr:value = '198.51.100.10']");
        assert_eq!(indicator["spec_version"], "2.1");
        assert_eq!(indicator["created"], "2026-05-25T12:34:56Z");

        let attack_pattern = bundle
            .objects
            .iter()
            .find(|object| object["type"] == "attack-pattern")
            .expect("expected attack-pattern");
        assert_eq!(
            attack_pattern["external_references"][0]["source_name"],
            "mitre"
        );
        assert_eq!(
            attack_pattern["external_references"][0]["external_id"],
            "T1566.001"
        );

        let relationship = bundle
            .objects
            .iter()
            .find(|object| object["type"] == "relationship")
            .expect("expected relationship");
        assert_eq!(relationship["relationship_type"], "uses");
        assert!(relationship["source_ref"]
            .as_str()
            .unwrap()
            .starts_with("threat-actor--"));
        assert!(relationship["target_ref"]
            .as_str()
            .unwrap()
            .starts_with("attack-pattern--"));
    }

    #[test]
    fn stix_export_is_deterministic_for_same_graph() {
        let nodes = vec![sample_node(
            "domain-1",
            NodeType::Domain,
            "evil.example",
            TypeSpecificProps::Domain(DomainProps {
                domain: "evil.example".to_string(),
                registrar: None,
                creation_date: None,
            }),
        )];
        let relations = Vec::new();

        let first: StixBundle = to_stix_bundle(&nodes, &relations);
        let second: StixBundle = to_stix_bundle(&nodes, &relations);

        assert_eq!(first.id, second.id);
        assert_eq!(first.objects, second.objects);
    }
}
