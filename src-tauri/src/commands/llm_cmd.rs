//! Tauri commands for LLM-powered operations (entity extraction, etc.).

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::ai::actions::{ActionApproval, AgentAction, AgentPlan};
use crate::ai::agent::ForensicsAgent;
use crate::services::llm::client::LlmClient;
use crate::services::llm::extractor::{
    extract_entities, extract_relations, ExtractedEntity, ExtractedRelation,
};
use crate::services::llm::suggester::{suggest_connections, Suggestion};
use crate::state::{AppState, LlmConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfigStatus {
    pub configured: bool,
    pub model: String,
    pub api_base: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigureLlmRequest {
    pub api_base: String,
    pub api_key: String,
    pub model: String,
}

fn config_status(config: &LlmConfig) -> LlmConfigStatus {
    LlmConfigStatus {
        configured: config.configured,
        model: config.model.clone(),
        api_base: config.api_base.clone(),
    }
}

fn configured_llm(state: &State<'_, AppState>) -> Result<LlmClient, String> {
    let config = state
        .llm_config
        .read()
        .map_err(|_| "failed to read LLM config".to_string())?
        .clone();

    if !config.configured {
        return Err("LLM is not configured".to_string());
    }

    Ok(LlmClient::new(config))
}

#[tauri::command]
pub async fn configure_llm(
    config: ConfigureLlmRequest,
    state: State<'_, AppState>,
) -> Result<LlmConfigStatus, String> {
    let new_config = LlmConfig {
        api_base: config.api_base.trim().trim_end_matches('/').to_string(),
        api_key: config.api_key.trim().to_string(),
        model: config.model.trim().to_string(),
        configured: !config.api_base.trim().is_empty() && !config.model.trim().is_empty(),
    };

    let mut stored = state
        .llm_config
        .write()
        .map_err(|_| "failed to update LLM config".to_string())?;
    *stored = new_config.clone();

    Ok(config_status(&new_config))
}

#[tauri::command]
pub fn get_llm_config(state: State<'_, AppState>) -> Result<LlmConfigStatus, String> {
    let config = state
        .llm_config
        .read()
        .map_err(|_| "failed to read LLM config".to_string())?;
    Ok(config_status(&config))
}

#[tauri::command]
pub async fn extract_from_text(
    text: String,
    state: State<'_, AppState>,
) -> Result<Vec<ExtractedEntity>, String> {
    let client = configured_llm(&state)?;
    extract_entities(&client, &text).await.map_err(Into::into)
}

#[tauri::command]
pub async fn extract_relations_from_text(
    entities: Vec<ExtractedEntity>,
    text: String,
    state: State<'_, AppState>,
) -> Result<Vec<ExtractedRelation>, String> {
    let client = configured_llm(&state)?;
    extract_relations(&client, &entities, &text)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn suggest_next_steps(
    node_info: String,
    state: State<'_, AppState>,
) -> Result<Vec<Suggestion>, String> {
    let client = configured_llm(&state)?;
    suggest_connections(&client, &node_info)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn agent_analyze(
    node_summaries: Vec<String>,
    relation_summaries: Vec<String>,
    state: State<'_, AppState>,
) -> Result<AgentPlan, String> {
    let config = state
        .llm_config
        .read()
        .map_err(|_| "failed to read LLM config".to_string())?
        .clone();

    if !config.configured {
        return Err("LLM is not configured".to_string());
    }

    let agent = ForensicsAgent::new(config, state.tool_registry.all_manifests());
    agent
        .analyze(&node_summaries, &relation_summaries)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub fn agent_apply_approvals(
    plan: AgentPlan,
    approvals: Vec<ActionApproval>,
) -> Result<Vec<AgentAction>, String> {
    ForensicsAgent::apply_approvals(&plan, &approvals).map_err(Into::into)
}
