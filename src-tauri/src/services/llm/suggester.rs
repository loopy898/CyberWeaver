//! Graph suggester — uses LLM to suggest next investigative steps.

use super::client::LlmClient;
use super::extractor::{clean_json_response, parse_node_type, parse_relation_type};
use super::prompts;
use crate::error::AppError;
use crate::models::domain::{NodeType, RelationType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct RawSuggestionResult {
    suggestions: Vec<RawSuggestion>,
}

#[derive(Debug, Deserialize)]
struct RawSuggestion {
    action: String,
    description: String,
    entity_type: Option<String>,
    relation_type: Option<String>,
    confidence: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub action: SuggestionAction,
    pub description: String,
    pub entity_type: Option<NodeType>,
    pub relation_type: Option<RelationType>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionAction {
    AddNode,
    AddRelation,
    QueryExternal,
    Investigate,
}

pub async fn suggest_connections(
    client: &LlmClient,
    node_info: &str,
) -> Result<Vec<Suggestion>, AppError> {
    let user_message = format!(
        "Investigation graph nodes:\n{}\n\nSuggest next investigation steps.",
        node_info
    );
    let response = client
        .chat(prompts::SUGGESTION_SYSTEM, &user_message)
        .await?;
    let cleaned = clean_json_response(&response);
    let result: RawSuggestionResult = serde_json::from_str(&cleaned)
        .map_err(|e| AppError::LlmService(format!("Parse suggestions: {e}")))?;

    result
        .suggestions
        .into_iter()
        .map(|suggestion| {
            let action = match suggestion.action.as_str() {
                "add_node" => SuggestionAction::AddNode,
                "add_relation" => SuggestionAction::AddRelation,
                "query_external" => SuggestionAction::QueryExternal,
                "investigate" => SuggestionAction::Investigate,
                other => {
                    return Err(AppError::LlmService(format!(
                        "Unknown suggestion action: {other}"
                    )))
                }
            };

            let entity_type = match suggestion.entity_type {
                Some(value) => Some(parse_node_type(&value)?),
                None => None,
            };
            let relation_type = match suggestion.relation_type {
                Some(value) => Some(parse_relation_type(&value)?),
                None => None,
            };

            Ok(Suggestion {
                action,
                description: suggestion.description,
                entity_type,
                relation_type,
                confidence: suggestion.confidence.unwrap_or(0.5).clamp(0.0, 1.0),
            })
        })
        .collect()
}
