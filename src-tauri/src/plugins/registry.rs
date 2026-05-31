use std::path::Path;
use std::sync::Arc;

use cw_plugin_sdk::{InvestigationTool, ToolInput, ToolManifest, ToolOutput};

use crate::plugins::loader::{discover_plugins, LoadedPlugin, PluginError};

pub struct ToolRegistry {
    builtin_tools: Vec<Arc<dyn InvestigationTool>>,
    dynamic_plugins: Vec<Arc<LoadedPlugin>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            builtin_tools: Vec::new(),
            dynamic_plugins: Vec::new(),
        }
    }

    pub fn register_builtin(&mut self, tool: Arc<dyn InvestigationTool>) {
        self.builtin_tools.push(tool);
    }

    pub fn load_plugins_from(&mut self, dir: &Path) {
        let plugins = discover_plugins(dir);
        self.dynamic_plugins
            .extend(plugins.into_iter().map(Arc::new));
    }

    pub fn all_manifests(&self) -> Vec<ToolManifest> {
        let mut manifests: Vec<ToolManifest> =
            self.builtin_tools.iter().map(|tool| tool.manifest()).collect();
        for plugin in &self.dynamic_plugins {
            manifests.push(plugin.manifest.clone());
        }
        manifests
    }

    pub fn tool_names(&self) -> Vec<String> {
        self.all_manifests()
            .iter()
            .map(|manifest| manifest.name.clone())
            .collect()
    }

    pub async fn execute(
        &self,
        tool_name: &str,
        input: ToolInput,
    ) -> Result<ToolOutput, PluginError> {
        for tool in &self.builtin_tools {
            if tool.manifest().name == tool_name {
                let tool = Arc::clone(tool);
                let result = tokio::task::spawn_blocking(move || tool.execute(input))
                    .await
                    .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
                return result.map_err(PluginError::ExecutionFailed);
            }
        }

        for plugin in &self.dynamic_plugins {
            if plugin.manifest.name == tool_name {
                let plugin = Arc::clone(plugin);
                let plugin_input = input.clone();
                let result = tokio::task::spawn_blocking(move || plugin.execute(&plugin_input))
                .await
                .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
                return result;
            }
        }

        Err(PluginError::ExecutionFailed(format!(
            "tool not found: {tool_name}"
        )))
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
