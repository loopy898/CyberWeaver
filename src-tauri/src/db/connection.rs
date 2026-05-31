use sea_orm::{ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement};
use std::path::Path;

const DB_URL: &str = "sqlite://cyberweaver.db?mode=rwc";

/// 初始化数据库连接，并设置 SQLite 性能 pragma
pub async fn init_db() -> Result<DatabaseConnection, sea_orm::DbErr> {
    let db = Database::connect(DB_URL).await?;

    let pragmas = [
        "PRAGMA journal_mode = WAL;",
        "PRAGMA synchronous = NORMAL;",
        "PRAGMA foreign_keys = ON;",
        "PRAGMA busy_timeout = 5000;",
        "PRAGMA cache_size = -20000;",
        "PRAGMA temp_store = MEMORY;",
    ];

    for pragma in &pragmas {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            pragma.to_string(),
        ))
        .await?;
    }

    Ok(db)
}

/// 获取数据库文件路径（用于备份/重置等操作）
pub fn db_file_path() -> &'static Path {
    Path::new("cyberweaver.db")
}
