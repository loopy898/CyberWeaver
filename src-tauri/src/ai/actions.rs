//! Agent actions — concrete operations the AI agent can perform.

use serde::{Deserialize, Serialize};

use crate::models::domain::{NodeType, RelationType};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", content = "params")]
pub enum AgentAction {
    AddNode {
        node_type: NodeType,
        label: String,
        description: String,
        confidence: f32,
        pos_x: f64,
        pos_y: f64,
    },
    AddRelation {
        source_node_id: String,
        target_node_id: String,
        relation_type: RelationType,
        label: String,
        confidence: f32,
    },
    QueryExternal {
        query_type: String,
        query_value: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPlan {
    pub reasoning: String,
    pub actions: Vec<AgentAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionApproval {
    pub action_index: usize,
    pub approved: bool,
    pub modifications: Option<String>,
}
