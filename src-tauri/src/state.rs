use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

use crate::plugins::registry::ToolRegistry;

pub struct AppState {
    pub db: Arc<DatabaseConnection>,

    pub ws_broadcast: broadcast::Sender<String>,

    pub tool_registry: Arc<ToolRegistry>,

    pub llm_config: RwLock<LlmConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub api_base: String,
    pub api_key: String,
    pub model: String,
    pub configured: bool,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            api_base: String::new(),
            api_key: String::new(),
            model: "gpt-4o".to_string(),
            configured: false,
        }
    }
}

impl AppState {
    pub fn new(db: DatabaseConnection, tool_registry: ToolRegistry) -> Self {
        let (tx, _) = broadcast::channel::<String>(64);
        Self {
            db: Arc::new(db),
            ws_broadcast: tx,
            tool_registry: Arc::new(tool_registry),
            llm_config: RwLock::new(LlmConfig::default()),
        }
    }
}
