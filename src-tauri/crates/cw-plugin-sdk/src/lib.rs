pub mod export_macro;
pub mod types;

pub use types::*;

pub trait InvestigationTool: Send + Sync {
    fn manifest(&self) -> ToolManifest;
    fn execute(&self, input: ToolInput) -> Result<ToolOutput, String>;
}
