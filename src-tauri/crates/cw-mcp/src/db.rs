use std::path::Path;

use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Statement};

use crate::error::McpError;

pub async fn open_db(db_path: impl AsRef<Path>) -> Result<DatabaseConnection, McpError> {
    let db_url = format!("sqlite:{}?mode=rwc", db_path.as_ref().display());
    let db = Database::connect(&db_url).await?;

    db.execute(Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        "PRAGMA journal_mode = WAL;".to_string(),
    ))
    .await?;

    Ok(db)
}
