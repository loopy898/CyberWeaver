use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::ServerInfo,
    tool_handler,
};
use sea_orm::DatabaseConnection;

use crate::SharedDatabase;

#[tool_handler(router = self.tool_router)]
impl ServerHandler for CyberWeaverMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::default()
    }
}

#[derive(Debug, Clone)]
pub struct CyberWeaverMcp {
    db: SharedDatabase,
    tool_router: ToolRouter<Self>,
}

impl CyberWeaverMcp {
    pub fn new(db: SharedDatabase) -> Self {
        Self {
            db,
            tool_router: Self::read_tool_router()
                + Self::write_tool_router()
                + Self::ai_tool_router()
                + Self::import_export_tool_router(),
        }
    }

    pub fn db(&self) -> &DatabaseConnection {
        self.db.as_ref()
    }
}
