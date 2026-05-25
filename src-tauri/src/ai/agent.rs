//! AI agent — observes graph state, proposes investigation actions.

use super::actions::{ActionApproval, AgentAction, AgentPlan};
use crate::error::AppError;
use crate::services::llm::client::LlmClient;
use crate::services::llm::extractor::clean_json_response;
use crate::state::LlmConfig;
use serde::Deserialize;
use std::collections::HashSet;

const AGENT_PROMPT: &str = r#"You are an autonomous DFIR investigation agent.
Given the current investigation graph state, propose concrete next actions.

Return ONLY valid JSON. Do not include markdown fences or commentary.
{
  "reasoning": "explain your investigative reasoning in 2-3 sentences",
  "actions": [
    { "action": "AddNode", "params": { "node_type": "ip_address", "label": "...", "description": "...", "confidence": 0.8, "pos_x": 0.0, "pos_y": 0.0 } },
    { "action": "AddRelation", "params": { "source_node_id": "node-1", "target_node_id": "node-2", "relation_type": "connects_to", "label": "...", "confidence": 0.8 } },
    { "action": "QueryExternal", "params": { "query_type": "virustotal|whois|shodan", "query_value": "..." } }
  ]
}
Focus on high-impact actions. Be specific and data-driven."#;

#[derive(Debug, Deserialize)]
struct AgentPlanResponse {
    reasoning: String,
    actions: Vec<AgentAction>,
}

pub struct ForensicsAgent {
    client: LlmClient,
}

impl ForensicsAgent {
    pub fn new(config: LlmConfig) -> Self {
        Self {
            client: LlmClient::new(config),
        }
    }

    pub async fn analyze(
        &self,
        node_summaries: &[String],
        relation_summaries: &[String],
    ) -> Result<AgentPlan, AppError> {
        let state_json = serde_json::json!({
            "nodes": node_summaries,
            "relations": relation_summaries,
        });
        let serialized_state = serde_json::to_string_pretty(&state_json)
            .map_err(|error| AppError::LlmService(format!("Serialize agent state: {error}")))?;
        let user_message =
            format!("Current investigation graph:\n{serialized_state}\n\nPropose next actions.");
        let response = self.client.chat(AGENT_PROMPT, &user_message).await?;
        let cleaned = clean_json_response(&response);
        let plan: AgentPlanResponse = serde_json::from_str(&cleaned)
            .map_err(|e| AppError::LlmService(format!("Parse agent plan: {e}")))?;

        Ok(AgentPlan {
            reasoning: plan.reasoning,
            actions: plan.actions,
        })
    }

    pub fn apply_approvals(
        plan: &AgentPlan,
        approvals: &[ActionApproval],
    ) -> Result<Vec<AgentAction>, AppError> {
        let approved: HashSet<usize> = approvals
            .iter()
            .filter(|approval| approval.approved)
            .map(|approval| approval.action_index)
            .collect();

        plan.actions
            .iter()
            .enumerate()
            .filter(|(index, _)| approved.contains(index))
            .map(|(index, action)| {
                let override_action = approvals
                    .iter()
                    .find(|approval| approval.action_index == index && approval.approved)
                    .and_then(|approval| approval.modifications.as_deref())
                    .filter(|value| !value.trim().is_empty());

                match override_action {
                    Some(raw) => serde_json::from_str::<AgentAction>(raw).map_err(|error| {
                        AppError::InvalidInput(format!(
                            "invalid approval modification for action {index}: {error}"
                        ))
                    }),
                    None => Ok(action.clone()),
                }
            })
            .collect()
    }
}
