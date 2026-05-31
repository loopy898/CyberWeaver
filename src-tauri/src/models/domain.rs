//! Core domain models — Node, Relation, and their type-specific properties.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Node type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    IpAddress,
    Domain,
    FileHash,
    Process,
    Malware,
    Ttp,
    ThreatActor,
    Asset,
}

impl NodeType {
    pub fn display_name(&self) -> &'static str {
        match self {
            NodeType::IpAddress => "IP 地址",
            NodeType::Domain => "域名",
            NodeType::FileHash => "文件哈希",
            NodeType::Process => "进程",
            NodeType::Malware => "恶意软件",
            NodeType::Ttp => "攻击技术",
            NodeType::ThreatActor => "威胁组织",
            NodeType::Asset => "资产",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            NodeType::IpAddress => "ip",
            NodeType::Domain => "domain",
            NodeType::FileHash => "file-hash",
            NodeType::Process => "process",
            NodeType::Malware => "malware",
            NodeType::Ttp => "ttp",
            NodeType::ThreatActor => "threat-actor",
            NodeType::Asset => "asset",
        }
    }
}

// ---------------------------------------------------------------------------
// Relation type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    ConnectsTo,
    ResolvesTo,
    Creates,
    BelongsTo,
    Uses,
    Targets,
    Contains,
}

impl RelationType {
    pub fn display_name(&self) -> &'static str {
        match self {
            RelationType::ConnectsTo => "网络连接",
            RelationType::ResolvesTo => "DNS 解析",
            RelationType::Creates => "创建",
            RelationType::BelongsTo => "归属于",
            RelationType::Uses => "使用技术",
            RelationType::Targets => "攻击目标",
            RelationType::Contains => "包含",
        }
    }

    pub fn is_directed(&self) -> bool {
        matches!(
            self,
            RelationType::ConnectsTo
                | RelationType::ResolvesTo
                | RelationType::Creates
                | RelationType::BelongsTo
                | RelationType::Uses
                | RelationType::Targets
                | RelationType::Contains
        )
    }
}

// ---------------------------------------------------------------------------
// Type-specific property structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IpAddressProps {
    pub address: String,
    pub version: Option<String>,
    pub geo_location: Option<String>,
    pub asn: Option<String>,
    pub isp: Option<String>,
    pub reputation: Option<Reputation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DomainProps {
    pub domain: String,
    pub registrar: Option<String>,
    pub creation_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HashAlgorithm {
    #[default]
    MD5,
    SHA1,
    SHA256,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileHashProps {
    pub hash_value: String,
    pub algorithm: HashAlgorithm,
    pub file_name: Option<String>,
    pub file_size: Option<u64>,
    pub file_type: Option<String>,
    pub malware_classification: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessProps {
    pub process_name: String,
    pub pid: Option<u32>,
    pub command_line: Option<String>,
    pub parent_process: Option<String>,
    pub user: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MalwareProps {
    pub family_name: String,
    pub aliases: Vec<String>,
    pub malware_type: Option<String>,
    pub first_seen: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TtpProps {
    pub mitre_id: String,
    pub tactic: Option<String>,
    pub platform: Vec<String>,
    pub data_source: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThreatActorProps {
    pub name: String,
    pub aliases: Vec<String>,
    pub motivation: Option<String>,
    pub sophistication: Option<String>,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AssetProps {
    pub hostname: String,
    pub os: Option<String>,
    pub ip_addresses: Vec<String>,
    pub owner: Option<String>,
    pub criticality: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Reputation {
    Clean,
    Suspicious,
    Malicious,
    Unknown,
}

// ---------------------------------------------------------------------------
// Tagged union of type-specific properties
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum TypeSpecificProps {
    IpAddress(IpAddressProps),
    Domain(DomainProps),
    FileHash(FileHashProps),
    Process(ProcessProps),
    Malware(MalwareProps),
    Ttp(TtpProps),
    ThreatActor(ThreatActorProps),
    Asset(AssetProps),
}

// ---------------------------------------------------------------------------
// Transfer structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeData {
    pub id: String,
    pub node_type: NodeType,
    pub label: String,
    pub description: String,
    pub confidence: f32,
    pub properties: TypeSpecificProps,
    pub pos_x: f64,
    pub pos_y: f64,
    pub investigation_id: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationData {
    pub id: String,
    pub relation_type: RelationType,
    pub source_node_id: String,
    pub target_node_id: String,
    pub label: String,
    pub confidence: f32,
    pub first_seen: Option<String>,
    pub last_seen: Option<String>,
    pub investigation_id: String,
}
