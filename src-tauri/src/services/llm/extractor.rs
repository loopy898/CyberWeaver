//! Entity and relation extractor — calls LLM, parses structured JSON output.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::client::LlmClient;
use super::prompts;
use crate::error::AppError;
use crate::models::domain::{
    AssetProps, DomainProps, FileHashProps, IpAddressProps, MalwareProps, NodeType, ProcessProps,
    RelationType, ThreatActorProps, TtpProps, TypeSpecificProps,
};

#[derive(Debug, Deserialize)]
struct RawExtractionResult {
    entities: Vec<RawEntity>,
}

#[derive(Debug, Deserialize)]
struct RawEntity {
    node_type: String,
    label: String,
    description: Option<String>,
    confidence: Option<f32>,
    properties: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct RawRelationResult {
    relations: Vec<RawRelation>,
}

#[derive(Debug, Deserialize)]
struct RawRelation {
    source_index: usize,
    target_index: usize,
    relation_type: String,
    label: Option<String>,
    confidence: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub node_type: NodeType,
    pub label: String,
    pub description: String,
    pub confidence: f32,
    pub properties: TypeSpecificProps,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelation {
    pub source_index: usize,
    pub target_index: usize,
    pub relation_type: RelationType,
    pub label: String,
    pub confidence: f32,
}

pub fn clean_json_response(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(inner) = trimmed
        .strip_prefix("```json")
        .and_then(|value| value.strip_suffix("```"))
    {
        return inner.trim().to_string();
    }
    if let Some(inner) = trimmed
        .strip_prefix("```")
        .and_then(|value| value.strip_suffix("```"))
    {
        return inner.trim().to_string();
    }
    trimmed.to_string()
}

pub(crate) fn parse_node_type(value: &str) -> Result<NodeType, AppError> {
    match value {
        "IpAddress" | "ip_address" => Ok(NodeType::IpAddress),
        "Domain" | "domain" => Ok(NodeType::Domain),
        "FileHash" | "file_hash" => Ok(NodeType::FileHash),
        "Process" | "process" => Ok(NodeType::Process),
        "Malware" | "malware" => Ok(NodeType::Malware),
        "Ttp" | "ttp" => Ok(NodeType::Ttp),
        "ThreatActor" | "threat_actor" => Ok(NodeType::ThreatActor),
        "Asset" | "asset" => Ok(NodeType::Asset),
        _ => Err(AppError::LlmService(format!("unknown node_type: {value}"))),
    }
}

pub(crate) fn parse_relation_type(value: &str) -> Result<RelationType, AppError> {
    match value {
        "ConnectsTo" | "connects_to" => Ok(RelationType::ConnectsTo),
        "ResolvesTo" | "resolves_to" => Ok(RelationType::ResolvesTo),
        "Creates" | "creates" => Ok(RelationType::Creates),
        "BelongsTo" | "belongs_to" => Ok(RelationType::BelongsTo),
        "Uses" | "uses" => Ok(RelationType::Uses),
        "Targets" | "targets" => Ok(RelationType::Targets),
        "Contains" | "contains" => Ok(RelationType::Contains),
        _ => Err(AppError::LlmService(format!(
            "unknown relation_type: {value}"
        ))),
    }
}

fn default_props_for(node_type: &NodeType) -> TypeSpecificProps {
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

fn parse_type_specific_props(
    node_type: &NodeType,
    properties: Option<Value>,
) -> Result<TypeSpecificProps, AppError> {
    let Some(properties) = properties else {
        return Ok(default_props_for(node_type));
    };

    match node_type {
        NodeType::IpAddress => serde_json::from_value::<IpAddressProps>(properties)
            .map(TypeSpecificProps::IpAddress)
            .map_err(|error| {
                AppError::LlmService(format!("invalid IpAddress properties: {error}"))
            }),
        NodeType::Domain => serde_json::from_value::<DomainProps>(properties)
            .map(TypeSpecificProps::Domain)
            .map_err(|error| AppError::LlmService(format!("invalid Domain properties: {error}"))),
        NodeType::FileHash => serde_json::from_value::<FileHashProps>(properties)
            .map(TypeSpecificProps::FileHash)
            .map_err(|error| AppError::LlmService(format!("invalid FileHash properties: {error}"))),
        NodeType::Process => serde_json::from_value::<ProcessProps>(properties)
            .map(TypeSpecificProps::Process)
            .map_err(|error| AppError::LlmService(format!("invalid Process properties: {error}"))),
        NodeType::Malware => serde_json::from_value::<MalwareProps>(properties)
            .map(TypeSpecificProps::Malware)
            .map_err(|error| AppError::LlmService(format!("invalid Malware properties: {error}"))),
        NodeType::Ttp => serde_json::from_value::<TtpProps>(properties)
            .map(TypeSpecificProps::Ttp)
            .map_err(|error| AppError::LlmService(format!("invalid Ttp properties: {error}"))),
        NodeType::ThreatActor => serde_json::from_value::<ThreatActorProps>(properties)
            .map(TypeSpecificProps::ThreatActor)
            .map_err(|error| {
                AppError::LlmService(format!("invalid ThreatActor properties: {error}"))
            }),
        NodeType::Asset => serde_json::from_value::<AssetProps>(properties)
            .map(TypeSpecificProps::Asset)
            .map_err(|error| AppError::LlmService(format!("invalid Asset properties: {error}"))),
    }
}

pub async fn extract_entities(
    client: &LlmClient,
    text: &str,
) -> Result<Vec<ExtractedEntity>, AppError> {
    let response = client.chat(prompts::ENTITY_EXTRACTION_SYSTEM, text).await?;
    let cleaned = clean_json_response(&response);
    let result: RawExtractionResult = serde_json::from_str(&cleaned)
        .map_err(|error| AppError::LlmService(format!("parse entities: {error}")))?;

    result
        .entities
        .into_iter()
        .map(|raw| {
            let node_type = parse_node_type(&raw.node_type)?;
            Ok(ExtractedEntity {
                node_type: node_type.clone(),
                label: raw.label,
                description: raw.description.unwrap_or_default(),
                confidence: raw.confidence.unwrap_or(0.5).clamp(0.0, 1.0),
                properties: parse_type_specific_props(&node_type, raw.properties)?,
            })
        })
        .collect()
}

pub async fn extract_relations(
    client: &LlmClient,
    entities: &[ExtractedEntity],
    text: &str,
) -> Result<Vec<ExtractedRelation>, AppError> {
    let entity_list: Vec<String> = entities
        .iter()
        .enumerate()
        .map(|(index, entity)| format!("[{index}] {:?}: {}", entity.node_type, entity.label))
        .collect();

    let user_message = format!(
        "Original text:\n{text}\n\nEntities:\n{}\n\nExtract supported relations using the indexes above.",
        entity_list.join("\n")
    );
    let response = client
        .chat(prompts::RELATION_EXTRACTION_SYSTEM, &user_message)
        .await?;
    let cleaned = clean_json_response(&response);
    let result: RawRelationResult = serde_json::from_str(&cleaned)
        .map_err(|error| AppError::LlmService(format!("parse relations: {error}")))?;

    result
        .relations
        .into_iter()
        .map(|raw| {
            if raw.source_index >= entities.len() || raw.target_index >= entities.len() {
                return Err(AppError::LlmService(format!(
                    "relation index out of range: source_index={}, target_index={}, entity_count={}",
                    raw.source_index,
                    raw.target_index,
                    entities.len()
                )));
            }

            Ok(ExtractedRelation {
                source_index: raw.source_index,
                target_index: raw.target_index,
                relation_type: parse_relation_type(&raw.relation_type)?,
                label: raw.label.unwrap_or_default(),
                confidence: raw.confidence.unwrap_or(0.5).clamp(0.0, 1.0),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::clean_json_response;

    #[test]
    fn strips_json_code_fence() {
        let raw = "```json\n{\"entities\":[]}\n```";
        assert_eq!(clean_json_response(raw), "{\"entities\":[]}");
    }

    #[test]
    fn strips_plain_code_fence() {
        let raw = "```\n{\"relations\":[]}\n```";
        assert_eq!(clean_json_response(raw), "{\"relations\":[]}");
    }
}
