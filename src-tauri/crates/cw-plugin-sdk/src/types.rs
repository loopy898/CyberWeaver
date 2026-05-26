use serde::{Deserialize, Serialize};

pub const SDK_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifest {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub parameters: Vec<ToolParameter>,
    pub input_types: Vec<String>,
    pub output_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub parameter_type: ParameterType,
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Integer,
    Float,
    Boolean,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInput {
    pub node_id: Option<String>,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub new_nodes: Vec<DiscoveredNode>,
    pub new_relations: Vec<DiscoveredRelation>,
    pub enriched_properties: serde_json::Value,
    pub text_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredNode {
    pub node_type: String,
    pub label: String,
    pub description: String,
    pub properties: serde_json::Value,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredRelation {
    pub source_label: String,
    pub target_label: String,
    pub relation_type: String,
    pub label: String,
    pub confidence: f32,
}
