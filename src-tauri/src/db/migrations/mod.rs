use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};

pub async fn run_migrations(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    let statements = vec![
        "CREATE TABLE IF NOT EXISTS investigations (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
        "CREATE TABLE IF NOT EXISTS nodes (
            id TEXT PRIMARY KEY,
            investigation_id TEXT NOT NULL,
            node_type TEXT NOT NULL,
            label TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            confidence REAL NOT NULL DEFAULT 1.0,
            properties TEXT NOT NULL DEFAULT '{}',
            pos_x REAL NOT NULL DEFAULT 0,
            pos_y REAL NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (investigation_id) REFERENCES investigations(id) ON DELETE CASCADE
        );",
        "CREATE TABLE IF NOT EXISTS relations (
            id TEXT PRIMARY KEY,
            investigation_id TEXT NOT NULL,
            relation_type TEXT NOT NULL,
            source_node_id TEXT NOT NULL,
            target_node_id TEXT NOT NULL,
            label TEXT NOT NULL DEFAULT '',
            confidence REAL NOT NULL DEFAULT 1.0,
            first_seen TEXT,
            last_seen TEXT,
            properties TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (investigation_id) REFERENCES investigations(id) ON DELETE CASCADE,
            FOREIGN KEY (source_node_id) REFERENCES nodes(id) ON DELETE CASCADE,
            FOREIGN KEY (target_node_id) REFERENCES nodes(id) ON DELETE CASCADE
        );",
        "CREATE INDEX IF NOT EXISTS idx_nodes_investigation ON nodes(investigation_id);",
        "CREATE INDEX IF NOT EXISTS idx_nodes_type ON nodes(node_type);",
        "CREATE INDEX IF NOT EXISTS idx_relations_investigation ON relations(investigation_id);",
        "CREATE INDEX IF NOT EXISTS idx_relations_source ON relations(source_node_id);",
        "CREATE INDEX IF NOT EXISTS idx_relations_target ON relations(target_node_id);",
        "CREATE INDEX IF NOT EXISTS idx_relations_type ON relations(relation_type);",
    ];

    for stmt in statements {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            stmt.to_string(),
        ))
        .await?;
    }

    Ok(())
}
