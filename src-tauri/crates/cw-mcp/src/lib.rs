pub mod db;
pub mod error;
pub mod server;
pub mod tools;

use std::{path::Path, sync::Arc};

use rmcp::{ServiceExt, transport::stdio};
use sea_orm::DatabaseConnection;

use crate::{db::open_db, error::McpError, server::CyberWeaverMcp};

pub async fn run(db_path: impl AsRef<Path>) -> Result<(), McpError> {
    let db = open_db(db_path).await?;
    let db = Arc::new(db);
    let server = CyberWeaverMcp::new(db);
    let service = server.serve(stdio()).await?;
    let _ = service.waiting().await?;
    Ok(())
}

pub type SharedDatabase = Arc<DatabaseConnection>;
