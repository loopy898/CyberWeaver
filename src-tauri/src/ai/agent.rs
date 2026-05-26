//! AI agent — observes graph state, proposes investigation actions.

use super::actions::{ActionApproval, AgentAction, AgentPlan};
use crate::error::AppError;
use crate::services::llm::client::LlmClient;
use crate::services::llm::extractor::clean_json_response;
use crate::state::LlmConfig;
use cw_plugin_sdk::{ParameterType, ToolManifest};
use serde::Deserialize;
use std::collections::HashSet;

const AGENT_PROMPT: &str = r#"You are an autonomous DFIR investigation agent.
Given the current investigation graph state, propose concrete next actions.

{TOOL_SECTION}

Return ONLY valid JSON. Do not include markdown fences or commentary.
{
  "reasoning": "explain your investigative reasoning in 2-3 sentences",
  "actions": [
    { "action": "AddNode", "params": { "node_type": "ip_address", "label": "...", "description": "...", "confidence": 0.8, "pos_x": 0.0, "pos_y": 0.0 } },
    { "action": "AddRelation", "params": { "source_node_id": "node-1", "target_node_id": "node-2", "relation_type": "connects_to", "label": "...", "confidence": 0.8 } },
    { "action": "QueryExternal", "params": { "query_type": "virustotal|whois|shodan", "query_value": "..." } },
    { "action": "UseTool", "params": { "tool_name": "virustotal_ip_lookup", "params": { "ip_address": "8.8.8.8" }, "auto_merge": true } }
  ]
}
Focus on high-impact actions. Be specific and data-driven. Use UseTool when an available investigation tool is a better fit than a generic external query."#;

#[derive(Debug, Deserialize)]
struct AgentPlanResponse {
    reasoning: String,
    actions: Vec<AgentAction>,
}

pub struct ForensicsAgent {
    client: LlmClient,
    tool_manifests: Vec<ToolManifest>,
}

impl ForensicsAgent {
    pub fn new(config: LlmConfig, tool_manifests: Vec<ToolManifest>) -> Self {
        Self {
            client: LlmClient::new(config),
            tool_manifests,
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
        let prompt = AGENT_PROMPT.replace(
            "{TOOL_SECTION}",
            &build_tool_section(&self.tool_manifests),
        );
        let response = self.client.chat(&prompt, &user_message).await?;
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

fn build_tool_section(tool_manifests: &[ToolManifest]) -> String {
    if tool_manifests.is_empty() {
        return "Available investigation tools:\n- None currently loaded.".to_string();
    }

    let entries = tool_manifests
        .iter()
        .map(|manifest| {
            let parameters = if manifest.parameters.is_empty() {
                "none".to_string()
            } else {
                manifest
                    .parameters
                    .iter()
                    .map(|parameter| {
                        let required = if parameter.required {
                            "required"
                        } else {
                            "optional"
                        };
                        format!(
                            "{} ({}, {}).",
                            parameter.name,
                            parameter_type_name(&parameter.parameter_type),
                            required
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            };

            format!(
                "- {}: {}.\n  Parameters: {} \n  Input types: [{}]. Output types: [{}].",
                manifest.name,
                manifest.description.trim_end_matches('.'),
                parameters,
                manifest.input_types.join(", "),
                manifest.output_types.join(", "),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("Available investigation tools:\n{entries}")
}

fn parameter_type_name(parameter_type: &ParameterType) -> &'static str {
    match parameter_type {
        ParameterType::String => "String",
        ParameterType::Integer => "Integer",
        ParameterType::Float => "Float",
        ParameterType::Boolean => "Boolean",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cw_plugin_sdk::{ToolManifest, ToolParameter};

    #[test]
    fn builds_tool_section_from_manifests() {
        let section = build_tool_section(&[ToolManifest {
            name: "virustotal_ip_lookup".to_string(),
            display_name: "VirusTotal IP".to_string(),
            description: "Query VirusTotal for an IP address.".to_string(),
            version: "1.0.0".to_string(),
            author: "tests".to_string(),
            parameters: vec![ToolParameter {
                name: "ip_address".to_string(),
                parameter_type: ParameterType::String,
                description: "IP address to query".to_string(),
                required: true,
                default_value: None,
            }],
            input_types: vec!["ip_address".to_string()],
            output_types: vec!["domain".to_string(), "malware".to_string()],
        }]);

        assert!(section.contains("Available investigation tools:"));
        assert!(section.contains("- virustotal_ip_lookup: Query VirusTotal for an IP address."));
        assert!(section.contains("Parameters: ip_address (String, required)."));
        assert!(section.contains("Input types: [ip_address]. Output types: [domain, malware]."));
    }

    #[test]
    fn approvals_can_override_with_use_tool_action() {
        let plan = AgentPlan {
            reasoning: "reasoning".to_string(),
            actions: vec![AgentAction::UseTool {
                tool_name: "whois_domain".to_string(),
                params: serde_json::json!({"domain": "example.com"}),
                auto_merge: true,
            }],
        };
        let approvals = vec![ActionApproval {
            action_index: 0,
            approved: true,
            modifications: Some(
                r#"{"action":"UseTool","params":{"tool_name":"virustotal_ip_lookup","params":{"ip_address":"8.8.8.8"},"auto_merge":true}}"#
                    .to_string(),
            ),
        }];

        let approved = ForensicsAgent::apply_approvals(&plan, &approvals)
            .expect("approval override should parse");

        assert!(matches!(
            &approved[0],
            AgentAction::UseTool {
                tool_name,
                auto_merge,
                ..
            } if tool_name == "virustotal_ip_lookup" && *auto_merge
        ));
    }
}
