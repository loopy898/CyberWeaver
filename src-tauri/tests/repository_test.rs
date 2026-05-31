use sea_orm::{ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement};
use tauri_app_lib::db::repositories::{
    node_repo::chrono_now, CreateNodeData, CreateRelationData, NodeRepo, RelationRepo,
    UpdateNodeData,
};

/// Create a SQLite database backed by a unique temporary file.
/// Avoids SQLite `:memory:` issues with connection pooling in SeaORM.
async fn setup_test_db() -> DatabaseConnection {
    let tmp_path = std::env::temp_dir().join(format!("test_cw_{}.db", uuid::Uuid::new_v4()));
    let url = format!("sqlite://{}?mode=rwc", tmp_path.display());
    let db = Database::connect(&url)
        .await
        .expect("Failed to create test database");

    let stmts = [
        "CREATE TABLE IF NOT EXISTS investigations (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL DEFAULT '',
            description TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
        "CREATE TABLE IF NOT EXISTS nodes (
            id TEXT PRIMARY KEY,
            investigation_id TEXT NOT NULL,
            node_type TEXT NOT NULL,
            label TEXT NOT NULL DEFAULT '',
            description TEXT NOT NULL DEFAULT '',
            confidence REAL NOT NULL DEFAULT 1.0,
            properties TEXT NOT NULL DEFAULT '{}',
            pos_x REAL NOT NULL DEFAULT 0,
            pos_y REAL NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
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
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    ];

    for stmt in &stmts {
        db.execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            stmt.to_string(),
        ))
        .await
        .expect("Failed to create test table");
    }

    db
}

/// Insert a minimal investigation row so that nodes refer to a real investigation_id.
async fn create_test_investigation(db: &DatabaseConnection) -> String {
    let now = chrono_now();
    let id = uuid::Uuid::new_v4().to_string();
    let sql = format!(
        "INSERT INTO investigations (id, name, description, created_at, updated_at)
         VALUES ('{id}', 'Test Investigation', '', '{now}', '{now}')"
    );
    db.execute(Statement::from_string(DatabaseBackend::Sqlite, sql))
        .await
        .expect("Failed to create test investigation");
    id
}

// ---------------------------------------------------------------------------
// Node CRUD tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_create_node() {
    let db = setup_test_db().await;
    let investigation_id = create_test_investigation(&db).await;
    let repo = NodeRepo::new(&db);

    let fixed_id = uuid::Uuid::new_v4().to_string();
    let created = repo
        .create(CreateNodeData {
            fixed_id: fixed_id.clone(),
            investigation_id: investigation_id.clone(),
            node_type: "ip_address".to_string(),
            label: "192.168.1.100".to_string(),
            description: "Suspicious IP".to_string(),
            confidence: 0.9,
            properties: "{}".to_string(),
            pos_x: 100.0,
            pos_y: 200.0,
        })
        .await
        .expect("Failed to create node");
    assert_eq!(created.label, "192.168.1.100");
    assert_eq!(created.node_type, "ip_address");
    assert!((created.confidence - 0.9f32).abs() < f32::EPSILON);
    assert!((created.pos_x - 100.0).abs() < f64::EPSILON);
    assert!((created.pos_y - 200.0).abs() < f64::EPSILON);
    assert_eq!(created.description, "Suspicious IP");
    // create() generates its own id (ignores fixed_id)
    assert!(!created.id.is_empty());
    assert_eq!(created.id, fixed_id); // repo uses the caller-supplied fixed_id
}

#[tokio::test]
async fn test_find_node_by_id() {
    let db = setup_test_db().await;
    let investigation_id = create_test_investigation(&db).await;
    let repo = NodeRepo::new(&db);

    let created = repo
        .create(CreateNodeData {
            fixed_id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation_id.clone(),
            node_type: "domain".to_string(),
            label: "evil.com".to_string(),
            description: "Malicious domain".to_string(),
            confidence: 0.8,
            properties: "{}".to_string(),
            pos_x: 50.0,
            pos_y: 50.0,
        })
        .await
        .expect("Failed to create node");

    let found = repo
        .find_by_id(&created.id)
        .await
        .expect("Failed to find node");
    assert!(found.is_some());
    assert_eq!(found.unwrap().label, "evil.com");
}

#[tokio::test]
async fn test_find_node_by_id_not_found() {
    let db = setup_test_db().await;
    let repo = NodeRepo::new(&db);

    let result = repo
        .find_by_id("nonexistent-id")
        .await
        .expect("Query should succeed");
    assert!(result.is_none());
}

#[tokio::test]
async fn test_find_nodes_by_investigation() {
    let db = setup_test_db().await;
    let investigation_id = create_test_investigation(&db).await;
    let repo = NodeRepo::new(&db);

    // Create two nodes in the same investigation
    for (label, ntype) in [("node-a", "ip_address"), ("node-b", "domain")] {
        repo.create(CreateNodeData {
            fixed_id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation_id.clone(),
            node_type: ntype.to_string(),
            label: label.to_string(),
            description: String::new(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
        })
        .await
        .expect("Failed to create node");
    }

    let nodes = repo
        .find_by_investigation(&investigation_id)
        .await
        .expect("Failed to find nodes");
    assert_eq!(nodes.len(), 2);
}

#[tokio::test]
async fn test_find_nodes_by_type() {
    let db = setup_test_db().await;
    let investigation_id = create_test_investigation(&db).await;
    let repo = NodeRepo::new(&db);

    repo.create(CreateNodeData {
        fixed_id: uuid::Uuid::new_v4().to_string(),
        investigation_id: investigation_id.clone(),
        node_type: "ip_address".to_string(),
        label: "10.0.0.1".to_string(),
        description: String::new(),
        confidence: 1.0,
        properties: "{}".to_string(),
        pos_x: 0.0,
        pos_y: 0.0,
    })
    .await
    .expect("Failed to create node");

    let ip_nodes = repo
        .find_by_type(&investigation_id, "ip_address")
        .await
        .expect("Failed to find by type");
    assert_eq!(ip_nodes.len(), 1);

    // Filtering by a type that has no rows returns empty list
    let domain_nodes = repo
        .find_by_type(&investigation_id, "domain")
        .await
        .expect("Failed to find by type");
    assert_eq!(domain_nodes.len(), 0);
}

#[tokio::test]
async fn test_update_node_full() {
    let db = setup_test_db().await;
    let investigation_id = create_test_investigation(&db).await;
    let repo = NodeRepo::new(&db);

    let created = repo
        .create(CreateNodeData {
            fixed_id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation_id.clone(),
            node_type: "ip_address".to_string(),
            label: "original".to_string(),
            description: "original desc".to_string(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
        })
        .await
        .expect("Failed to create node");

    let update = UpdateNodeData {
        label: Some("updated".to_string()),
        description: None,
        confidence: Some(0.5),
        properties: None,
        pos_x: Some(300.0),
        pos_y: Some(400.0),
    };

    let updated = repo
        .update(&created.id, update)
        .await
        .expect("Failed to update node");

    assert_eq!(updated.label, "updated");
    assert!((updated.confidence - 0.5f32).abs() < f32::EPSILON);
    assert!((updated.pos_x - 300.0).abs() < f64::EPSILON);
    assert!((updated.pos_y - 400.0).abs() < f64::EPSILON);
    // Field not included in the update remains unchanged
    assert_eq!(updated.description, "original desc");
}

#[tokio::test]
async fn test_update_node_partial() {
    let db = setup_test_db().await;
    let investigation_id = create_test_investigation(&db).await;
    let repo = NodeRepo::new(&db);

    let created = repo
        .create(CreateNodeData {
            fixed_id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation_id.clone(),
            node_type: "domain".to_string(),
            label: "old-label".to_string(),
            description: "old-desc".to_string(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 100.0,
            pos_y: 100.0,
        })
        .await
        .expect("Failed to create node");

    // Only update the label; everything else stays the same
    let update = UpdateNodeData {
        label: Some("new-label".to_string()),
        description: None,
        confidence: None,
        properties: None,
        pos_x: None,
        pos_y: None,
    };

    let updated = repo
        .update(&created.id, update)
        .await
        .expect("Failed to update node");

    assert_eq!(updated.label, "new-label");
    assert_eq!(updated.description, "old-desc");
    assert!((updated.confidence - 1.0f32).abs() < f32::EPSILON);
    assert!((updated.pos_x - 100.0).abs() < f64::EPSILON);
    assert!((updated.pos_y - 100.0).abs() < f64::EPSILON);
}

#[tokio::test]
async fn test_update_node_not_found() {
    let db = setup_test_db().await;
    let repo = NodeRepo::new(&db);

    let update = UpdateNodeData {
        label: Some("nope".to_string()),
        description: None,
        confidence: None,
        properties: None,
        pos_x: None,
        pos_y: None,
    };

    let result = repo.update("nonexistent-id", update).await;
    assert!(result.is_err());
    match result {
        Err(tauri_app_lib::error::AppError::NotFound(_)) => { /* expected */ }
        other => panic!("Expected NotFound, got {:?}", other),
    }
}

#[tokio::test]
async fn test_delete_node() {
    let db = setup_test_db().await;
    let investigation_id = create_test_investigation(&db).await;
    let repo = NodeRepo::new(&db);

    let created = repo
        .create(CreateNodeData {
            fixed_id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation_id.clone(),
            node_type: "ip_address".to_string(),
            label: "to-delete".to_string(),
            description: String::new(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
        })
        .await
        .expect("Failed to create node");

    repo.delete(&created.id)
        .await
        .expect("Failed to delete node");

    let after = repo
        .find_by_id(&created.id)
        .await
        .expect("Query should succeed");
    assert!(after.is_none());
}

#[tokio::test]
async fn test_upsert_batch_creates_nodes() {
    let db = setup_test_db().await;
    let investigation_id = create_test_investigation(&db).await;
    let repo = NodeRepo::new(&db);

    let node_id = uuid::Uuid::new_v4().to_string();
    let nodes = vec![CreateNodeData {
        fixed_id: node_id.clone(),
        investigation_id: investigation_id.clone(),
        node_type: "ip_address".to_string(),
        label: "upsert-test".to_string(),
        description: String::new(),
        confidence: 1.0,
        properties: "{}".to_string(),
        pos_x: 42.0,
        pos_y: 99.0,
    }];

    let results = repo.upsert_batch(nodes).await.expect("Failed to upsert");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, node_id); // upsert uses fixed_id
    assert_eq!(results[0].label, "upsert-test");
    assert!((results[0].pos_x - 42.0).abs() < f64::EPSILON);
    assert!((results[0].pos_y - 99.0).abs() < f64::EPSILON);
}

#[tokio::test]
async fn test_upsert_batch_replaces_existing() {
    let db = setup_test_db().await;
    let investigation_id = create_test_investigation(&db).await;
    let repo = NodeRepo::new(&db);

    let node_id = uuid::Uuid::new_v4().to_string();

    // First upsert — creates
    let nodes_a = vec![CreateNodeData {
        fixed_id: node_id.clone(),
        investigation_id: investigation_id.clone(),
        node_type: "ip_address".to_string(),
        label: "version-1".to_string(),
        description: String::new(),
        confidence: 1.0,
        properties: "{}".to_string(),
        pos_x: 0.0,
        pos_y: 0.0,
    }];
    repo.upsert_batch(nodes_a)
        .await
        .expect("First upsert failed");

    // Second upsert with the same fixed_id — replaces
    let nodes_b = vec![CreateNodeData {
        fixed_id: node_id.clone(),
        investigation_id: investigation_id.clone(),
        node_type: "domain".to_string(),
        label: "version-2".to_string(),
        description: String::new(),
        confidence: 0.5,
        properties: "{}".to_string(),
        pos_x: 500.0,
        pos_y: 600.0,
    }];
    let results = repo
        .upsert_batch(nodes_b)
        .await
        .expect("Second upsert failed");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, node_id);
    assert_eq!(results[0].label, "version-2");
    assert_eq!(results[0].node_type, "domain");
    assert!((results[0].confidence - 0.5f32).abs() < f32::EPSILON);
}

// ---------------------------------------------------------------------------
// Relation tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_create_relation() {
    let db = setup_test_db().await;
    let investigation_id = create_test_investigation(&db).await;
    let node_repo = NodeRepo::new(&db);

    let src = node_repo
        .create(CreateNodeData {
            fixed_id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation_id.clone(),
            node_type: "ip_address".to_string(),
            label: "10.0.0.1".to_string(),
            description: String::new(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
        })
        .await
        .unwrap();

    let tgt = node_repo
        .create(CreateNodeData {
            fixed_id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation_id.clone(),
            node_type: "domain".to_string(),
            label: "evil.com".to_string(),
            description: String::new(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
        })
        .await
        .unwrap();

    let rel_repo = RelationRepo::new(&db);
    let rel = rel_repo
        .create(CreateRelationData {
            investigation_id: investigation_id.clone(),
            relation_type: "connects_to".to_string(),
            source_node_id: src.id.clone(),
            target_node_id: tgt.id.clone(),
            label: "HTTP connection".to_string(),
            confidence: 0.8,
            first_seen: None,
            last_seen: None,
            properties: "{}".to_string(),
        })
        .await
        .expect("Failed to create relation");

    assert_eq!(rel.relation_type, "connects_to");
    assert_eq!(rel.source_node_id, src.id);
    assert_eq!(rel.target_node_id, tgt.id);
    assert_eq!(rel.label, "HTTP connection");
    assert!((rel.confidence - 0.8f32).abs() < f32::EPSILON);
}

#[tokio::test]
async fn test_find_outgoing_relations() {
    let db = setup_test_db().await;
    let investigation_id = create_test_investigation(&db).await;
    let node_repo = NodeRepo::new(&db);

    let src = node_repo
        .create(CreateNodeData {
            fixed_id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation_id.clone(),
            node_type: "ip_address".to_string(),
            label: "src".to_string(),
            description: String::new(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
        })
        .await
        .unwrap();

    let tgt = node_repo
        .create(CreateNodeData {
            fixed_id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation_id.clone(),
            node_type: "domain".to_string(),
            label: "tgt".to_string(),
            description: String::new(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
        })
        .await
        .unwrap();

    let rel_repo = RelationRepo::new(&db);
    rel_repo
        .create(CreateRelationData {
            investigation_id: investigation_id.clone(),
            relation_type: "connects_to".to_string(),
            source_node_id: src.id.clone(),
            target_node_id: tgt.id.clone(),
            label: "edge".to_string(),
            confidence: 1.0,
            first_seen: None,
            last_seen: None,
            properties: "{}".to_string(),
        })
        .await
        .unwrap();

    let outgoing = rel_repo
        .find_outgoing(&src.id)
        .await
        .expect("Failed to find outgoing");
    assert_eq!(outgoing.len(), 1);
    assert_eq!(outgoing[0].target_node_id, tgt.id);

    // Source node itself should have no incoming
    let incoming_src = rel_repo
        .find_incoming(&src.id)
        .await
        .expect("Failed to find incoming");
    assert!(incoming_src.is_empty());
}

#[tokio::test]
async fn test_find_incoming_relations() {
    let db = setup_test_db().await;
    let investigation_id = create_test_investigation(&db).await;
    let node_repo = NodeRepo::new(&db);

    let src = node_repo
        .create(CreateNodeData {
            fixed_id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation_id.clone(),
            node_type: "ip_address".to_string(),
            label: "src".to_string(),
            description: String::new(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
        })
        .await
        .unwrap();

    let tgt = node_repo
        .create(CreateNodeData {
            fixed_id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation_id.clone(),
            node_type: "domain".to_string(),
            label: "tgt".to_string(),
            description: String::new(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
        })
        .await
        .unwrap();

    let rel_repo = RelationRepo::new(&db);
    rel_repo
        .create(CreateRelationData {
            investigation_id: investigation_id.clone(),
            relation_type: "resolves_to".to_string(),
            source_node_id: src.id.clone(),
            target_node_id: tgt.id.clone(),
            label: "dns".to_string(),
            confidence: 1.0,
            first_seen: None,
            last_seen: None,
            properties: "{}".to_string(),
        })
        .await
        .unwrap();

    let incoming = rel_repo
        .find_incoming(&tgt.id)
        .await
        .expect("Failed to find incoming");
    assert_eq!(incoming.len(), 1);
    assert_eq!(incoming[0].source_node_id, src.id);
}

#[tokio::test]
async fn test_find_relations_by_investigation() {
    let db = setup_test_db().await;
    let investigation_id = create_test_investigation(&db).await;
    let node_repo = NodeRepo::new(&db);

    let a = node_repo
        .create(CreateNodeData {
            fixed_id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation_id.clone(),
            node_type: "ip_address".to_string(),
            label: "a".to_string(),
            description: String::new(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
        })
        .await
        .unwrap();

    let b = node_repo
        .create(CreateNodeData {
            fixed_id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation_id.clone(),
            node_type: "ip_address".to_string(),
            label: "b".to_string(),
            description: String::new(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
        })
        .await
        .unwrap();

    let rel_repo = RelationRepo::new(&db);
    rel_repo
        .create(CreateRelationData {
            investigation_id: investigation_id.clone(),
            relation_type: "connects_to".to_string(),
            source_node_id: a.id.clone(),
            target_node_id: b.id.clone(),
            label: "e1".to_string(),
            confidence: 1.0,
            first_seen: None,
            last_seen: None,
            properties: "{}".to_string(),
        })
        .await
        .unwrap();
    rel_repo
        .create(CreateRelationData {
            investigation_id: investigation_id.clone(),
            relation_type: "connects_to".to_string(),
            source_node_id: b.id.clone(),
            target_node_id: a.id.clone(),
            label: "e2".to_string(),
            confidence: 1.0,
            first_seen: None,
            last_seen: None,
            properties: "{}".to_string(),
        })
        .await
        .unwrap();

    let all = rel_repo
        .find_by_investigation(&investigation_id)
        .await
        .expect("Failed to find relations");
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_find_relations_between() {
    let db = setup_test_db().await;
    let investigation_id = create_test_investigation(&db).await;
    let node_repo = NodeRepo::new(&db);

    let a = node_repo
        .create(CreateNodeData {
            fixed_id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation_id.clone(),
            node_type: "ip_address".to_string(),
            label: "A".to_string(),
            description: String::new(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
        })
        .await
        .unwrap();

    let b = node_repo
        .create(CreateNodeData {
            fixed_id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation_id.clone(),
            node_type: "domain".to_string(),
            label: "B".to_string(),
            description: String::new(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
        })
        .await
        .unwrap();

    let c = node_repo
        .create(CreateNodeData {
            fixed_id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation_id.clone(),
            node_type: "domain".to_string(),
            label: "C".to_string(),
            description: String::new(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
        })
        .await
        .unwrap();

    let rel_repo = RelationRepo::new(&db);

    // A -> B (one edge)
    rel_repo
        .create(CreateRelationData {
            investigation_id: investigation_id.clone(),
            relation_type: "connects_to".to_string(),
            source_node_id: a.id.clone(),
            target_node_id: b.id.clone(),
            label: "A->B #1".to_string(),
            confidence: 1.0,
            first_seen: None,
            last_seen: None,
            properties: "{}".to_string(),
        })
        .await
        .unwrap();
    // A -> B (second edge)
    rel_repo
        .create(CreateRelationData {
            investigation_id: investigation_id.clone(),
            relation_type: "uses".to_string(),
            source_node_id: a.id.clone(),
            target_node_id: b.id.clone(),
            label: "A->B #2".to_string(),
            confidence: 1.0,
            first_seen: None,
            last_seen: None,
            properties: "{}".to_string(),
        })
        .await
        .unwrap();
    // A -> C (not returned when querying A-B)
    rel_repo
        .create(CreateRelationData {
            investigation_id: investigation_id.clone(),
            relation_type: "connects_to".to_string(),
            source_node_id: a.id.clone(),
            target_node_id: c.id.clone(),
            label: "A->C".to_string(),
            confidence: 1.0,
            first_seen: None,
            last_seen: None,
            properties: "{}".to_string(),
        })
        .await
        .unwrap();

    let between = rel_repo
        .find_between(&a.id, &b.id)
        .await
        .expect("Failed to find between");
    assert_eq!(between.len(), 2, "Should only return A->B edges");
}

#[tokio::test]
async fn test_delete_relation_by_id() {
    let db = setup_test_db().await;
    let investigation_id = create_test_investigation(&db).await;
    let node_repo = NodeRepo::new(&db);

    let a = node_repo
        .create(CreateNodeData {
            fixed_id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation_id.clone(),
            node_type: "ip_address".to_string(),
            label: "A".to_string(),
            description: String::new(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
        })
        .await
        .unwrap();

    let b = node_repo
        .create(CreateNodeData {
            fixed_id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation_id.clone(),
            node_type: "domain".to_string(),
            label: "B".to_string(),
            description: String::new(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
        })
        .await
        .unwrap();

    let rel_repo = RelationRepo::new(&db);
    let rel = rel_repo
        .create(CreateRelationData {
            investigation_id: investigation_id.clone(),
            relation_type: "connects_to".to_string(),
            source_node_id: a.id.clone(),
            target_node_id: b.id.clone(),
            label: "to-delete".to_string(),
            confidence: 1.0,
            first_seen: None,
            last_seen: None,
            properties: "{}".to_string(),
        })
        .await
        .unwrap();

    rel_repo.delete(&rel.id).await.expect("Failed to delete");

    let all = rel_repo
        .find_by_investigation(&investigation_id)
        .await
        .unwrap();
    assert_eq!(all.len(), 0);
    let outgoing = rel_repo.find_outgoing(&a.id).await.unwrap();
    assert_eq!(outgoing.len(), 0);
}

#[tokio::test]
async fn test_delete_relations_by_node() {
    let db = setup_test_db().await;
    let investigation_id = create_test_investigation(&db).await;
    let node_repo = NodeRepo::new(&db);

    let a = node_repo
        .create(CreateNodeData {
            fixed_id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation_id.clone(),
            node_type: "ip_address".to_string(),
            label: "A".to_string(),
            description: String::new(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
        })
        .await
        .unwrap();

    let b = node_repo
        .create(CreateNodeData {
            fixed_id: uuid::Uuid::new_v4().to_string(),
            investigation_id: investigation_id.clone(),
            node_type: "domain".to_string(),
            label: "B".to_string(),
            description: String::new(),
            confidence: 1.0,
            properties: "{}".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
        })
        .await
        .unwrap();

    let rel_repo = RelationRepo::new(&db);

    // a -> b
    rel_repo
        .create(CreateRelationData {
            investigation_id: investigation_id.clone(),
            relation_type: "connects_to".to_string(),
            source_node_id: a.id.clone(),
            target_node_id: b.id.clone(),
            label: "outgoing".to_string(),
            confidence: 1.0,
            first_seen: None,
            last_seen: None,
            properties: "{}".to_string(),
        })
        .await
        .unwrap();

    // b -> a (so a is also a target)
    rel_repo
        .create(CreateRelationData {
            investigation_id: investigation_id.clone(),
            relation_type: "connects_to".to_string(),
            source_node_id: b.id.clone(),
            target_node_id: a.id.clone(),
            label: "incoming".to_string(),
            confidence: 1.0,
            first_seen: None,
            last_seen: None,
            properties: "{}".to_string(),
        })
        .await
        .unwrap();

    // Delete both edges touching `a`
    rel_repo
        .delete_by_node(&a.id)
        .await
        .expect("Failed to delete by node");

    let all = rel_repo
        .find_by_investigation(&investigation_id)
        .await
        .unwrap();
    assert_eq!(all.len(), 0);
}
