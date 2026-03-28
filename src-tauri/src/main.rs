#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Json, State,
    },
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use pyo3::{prelude::*, types::{PyList, PyModule}};
use sea_orm::{
    ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, QueryResult, Statement,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::{broadcast, OnceCell, RwLock};
use uuid::Uuid;

static STATE: OnceCell<Arc<RuntimeState>> = OnceCell::const_new();
const DB_URL: &str = "sqlite://cyberweaver.db?mode=rwc";
const SHAPE_PREFIX: &str = "shape:";
const VIZ_PREFIX: &str = "viz:";
const THOUGHT_NODE_ID: &str = "agent-thought";
const WS_URL: &str = "ws://127.0.0.1:3000/ws";

#[derive(Clone)]
struct RuntimeState {
    db: DatabaseConnection,
    db_uses_legacy_type_column: bool,
    graph: Arc<RwLock<GraphManager>>,
    tx: broadcast::Sender<String>,
    plugins: Arc<RwLock<PluginRegistry>>,
    plugins_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct NodeRecord {
    id: String,
    created_at: String,
    x: f64,
    y: f64,
    node_type: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct EdgeRecord {
    id: String,
    source_id: String,
    target_id: String,
    relation: String,
    properties: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ComponentRecord {
    id: String,
    entity_id: String,
    component_type: String,
    fast_payload: Value,
    custom_payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct GraphSnapshot {
    nodes: Vec<NodeRecord>,
    edges: Vec<EdgeRecord>,
    components: Vec<ComponentRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ForensicsFinding {
    node_id: String,
    title: String,
    relation: String,
    evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ForensicsSummary {
    node_count: usize,
    edge_count: usize,
    component_count: usize,
    finding_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ForensicsReport {
    generated_at: String,
    summary: ForensicsSummary,
    findings: Vec<ForensicsFinding>,
    markdown: String,
}

#[derive(Debug, Default, Clone)]
struct GraphManager {
    nodes: HashMap<String, NodeRecord>,
    edges: HashMap<String, EdgeRecord>,
    components: HashMap<String, ComponentRecord>,
}

impl GraphManager {
    fn upsert_node(&mut self, node: NodeRecord) {
        self.nodes.insert(node.id.clone(), node);
    }

    fn upsert_edge(&mut self, edge: EdgeRecord) {
        self.edges.insert(edge.id.clone(), edge);
    }

    fn upsert_component(&mut self, component: ComponentRecord) {
        self.components.insert(component.id.clone(), component);
    }

    fn remove_node(&mut self, id: &str) {
        self.nodes.remove(id);
        self.edges
            .retain(|_, edge| edge.source_id != id && edge.target_id != id);
        self.components.retain(|_, component| component.entity_id != id);
    }

    fn snapshot(&self) -> GraphSnapshot {
        let mut nodes = self.nodes.values().cloned().collect::<Vec<_>>();
        let mut edges = self.edges.values().cloned().collect::<Vec<_>>();
        let mut components = self.components.values().cloned().collect::<Vec<_>>();
        nodes.sort_by(|a, b| a.id.cmp(&b.id));
        edges.sort_by(|a, b| a.id.cmp(&b.id));
        components.sort_by(|a, b| a.id.cmp(&b.id));
        GraphSnapshot {
            nodes,
            edges,
            components,
        }
    }
}

#[derive(Debug, Deserialize)]
struct NodePayload {
    pub id: String,
    pub r#type: String,
    pub x: f64,
    pub y: f64,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct EdgePayload {
    pub id: Option<String>,
    pub source_id: String,
    pub target_id: String,
    pub relation: String,
    pub properties: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ToolExecution {
    tool: String,
    params: Value,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum WsCommand {
    #[serde(rename = "debug_create_node")]
    DebugCreateNode,
    #[serde(rename = "tool_execution")]
    ToolExecution {
        tool: String,
        params: Value,
    },
}

#[derive(Debug, Serialize)]
struct GraphUpdateNode {
    id: String,
    node_type: String,
    x: f64,
    y: f64,
    content: String,
}

#[derive(Debug, Serialize)]
struct GraphUpdateEdge {
    id: String,
    source_id: String,
    target_id: String,
    relation: String,
}

#[derive(Debug, Serialize)]
struct GraphDelta {
    added_nodes: Vec<GraphUpdateNode>,
    updated_nodes: Vec<GraphUpdateNode>,
    added_edges: Vec<GraphUpdateEdge>,
    updated_edges: Vec<GraphUpdateEdge>,
}

#[derive(Debug, Serialize)]
struct GraphUpdateEvent {
    r#type: &'static str,
    delta: GraphDelta,
}

#[derive(Debug, Serialize)]
struct ToolResultEvent {
    r#type: &'static str,
    tool: String,
    ok: bool,
    message: String,
}

#[derive(Debug, Serialize)]
struct AgentTokenEvent {
    r#type: &'static str,
    token: String,
}

#[derive(Debug, Clone)]
struct RegisteredTool {
    name: String,
    module_name: String,
    function_name: String,
}

#[derive(Debug, Default, Clone)]
struct PluginRegistry {
    tools: HashMap<String, RegisteredTool>,
}

impl PluginRegistry {
    fn load_from_dir(plugins_dir: &Path) -> Self {
        let mut registry = Self::default();
        if !plugins_dir.exists() {
            return registry;
        }

        let read_dir = match std::fs::read_dir(plugins_dir) {
            Ok(read_dir) => read_dir,
            Err(_) => return registry,
        };

        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("py") {
                continue;
            }
            let file_name = match path.file_name().and_then(|name| name.to_str()) {
                Some(file_name) => file_name,
                None => continue,
            };
            if file_name == "sdk.py" {
                continue;
            }

            let content = match std::fs::read_to_string(&path) {
                Ok(content) => content,
                Err(_) => continue,
            };

            let module_name = match python_safe_module_name(&path) {
                Some(module_name) => module_name,
                None => continue,
            };

            for name in parse_registered_tool_names(&content) {
                registry.tools.insert(
                    name.clone(),
                    RegisteredTool {
                        name: name.clone(),
                        module_name: module_name.clone(),
                        function_name: name,
                    },
                );
            }
        }

        registry
    }

    fn get(&self, name: &str) -> Option<&RegisteredTool> {
        self.tools.get(name)
    }
}

fn parse_registered_tool_names(content: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("@register_tool") {
            continue;
        }
        let start = match trimmed.find('"') {
            Some(start) => start + 1,
            None => continue,
        };
        let rest = &trimmed[start..];
        let end = match rest.find('"') {
            Some(end) => end,
            None => continue,
        };
        names.push(rest[..end].to_string());
    }
    names
}

fn normalize_node_id(id: &str) -> String {
    id.trim()
        .trim_start_matches(SHAPE_PREFIX)
        .trim_start_matches(VIZ_PREFIX)
        .to_string()
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn python_safe_module_name(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    if stem.chars().all(|char| char.is_ascii_alphanumeric() || char == '_') {
        Some(stem.to_string())
    } else {
        None
    }
}

fn graph_update_message(
    added_nodes: Vec<NodeRecord>,
    updated_nodes: Vec<NodeRecord>,
    added_edges: Vec<EdgeRecord>,
    updated_edges: Vec<EdgeRecord>,
) -> Option<String> {
    let message = GraphUpdateEvent {
        r#type: "graph_update",
        delta: GraphDelta {
            added_nodes: added_nodes
                .into_iter()
                .map(|node| GraphUpdateNode {
                    id: node.id,
                    node_type: node.node_type,
                    x: node.x,
                    y: node.y,
                    content: node.content,
                })
                .collect(),
            updated_nodes: updated_nodes
                .into_iter()
                .map(|node| GraphUpdateNode {
                    id: node.id,
                    node_type: node.node_type,
                    x: node.x,
                    y: node.y,
                    content: node.content,
                })
                .collect(),
            added_edges: added_edges
                .into_iter()
                .map(|edge| GraphUpdateEdge {
                    id: edge.id,
                    source_id: edge.source_id,
                    target_id: edge.target_id,
                    relation: edge.relation,
                })
                .collect(),
            updated_edges: updated_edges
                .into_iter()
                .map(|edge| GraphUpdateEdge {
                    id: edge.id,
                    source_id: edge.source_id,
                    target_id: edge.target_id,
                    relation: edge.relation,
                })
                .collect(),
        },
    };
    serde_json::to_string(&message).ok()
}

fn tool_result_message(tool: String, ok: bool, message: String) -> Option<String> {
    serde_json::to_string(&ToolResultEvent {
        r#type: "tool_result",
        tool,
        ok,
        message,
    })
    .ok()
}

fn agent_token_message(token: String) -> Option<String> {
    serde_json::to_string(&AgentTokenEvent {
        r#type: "agent_token",
        token,
    })
    .ok()
}

async fn init_db() -> Result<DatabaseConnection, sea_orm::DbErr> {
    init_db_with_url(DB_URL).await
}

async fn init_db_with_url(url: &str) -> Result<DatabaseConnection, sea_orm::DbErr> {
    let db = Database::connect(url).await?;

    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "PRAGMA journal_mode = WAL;".to_string(),
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "PRAGMA synchronous = NORMAL;".to_string(),
    ))
    .await?;

    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE TABLE IF NOT EXISTS nodes (
            id TEXT PRIMARY KEY,
            created_at TEXT NOT NULL,
            x REAL NOT NULL,
            y REAL NOT NULL,
            node_type TEXT NOT NULL,
            content TEXT NOT NULL DEFAULT ''
        );"
        .to_string(),
    ))
    .await?;

    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE TABLE IF NOT EXISTS edges (
            id TEXT PRIMARY KEY,
            source_id TEXT NOT NULL,
            target_id TEXT NOT NULL,
            relation TEXT NOT NULL,
            properties TEXT NOT NULL DEFAULT '{}'
        );"
        .to_string(),
    ))
    .await?;

    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "CREATE TABLE IF NOT EXISTS components (
            id TEXT PRIMARY KEY,
            entity_id TEXT NOT NULL,
            component_type TEXT NOT NULL,
            fast_payload TEXT NOT NULL DEFAULT '{}',
            custom_payload TEXT NOT NULL DEFAULT '{}'
        );"
        .to_string(),
    ))
    .await?;

    migrate_nodes_table(&db).await?;
    migrate_edges_table(&db).await?;
    migrate_components_table(&db).await?;

    Ok(db)
}

async fn has_column(
    connection: &DatabaseConnection,
    table: &str,
    column: &str,
) -> Result<bool, sea_orm::DbErr> {
    let rows = connection
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("PRAGMA table_info({table});"),
        ))
        .await?;

    Ok(rows.into_iter().any(|row| {
        row.try_get::<String>("", "name")
            .map(|name| name == column)
            .unwrap_or(false)
    }))
}

async fn migrate_nodes_table(connection: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    if !has_column(connection, "nodes", "created_at").await? {
        connection
            .execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE nodes ADD COLUMN created_at TEXT NOT NULL DEFAULT '1970-01-01T00:00:00Z';".to_string(),
            ))
            .await?;
    }

    if !has_column(connection, "nodes", "node_type").await? {
        connection
            .execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE nodes ADD COLUMN node_type TEXT NOT NULL DEFAULT 'note';".to_string(),
            ))
            .await?;
        connection
            .execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "UPDATE nodes SET node_type = type WHERE type IS NOT NULL;".to_string(),
            ))
            .await?;
    }

    let has_legacy_type = has_column(connection, "nodes", "type").await?;
    if has_legacy_type {
        connection
            .execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "UPDATE nodes SET node_type = COALESCE(NULLIF(node_type, ''), type);".to_string(),
            ))
            .await?;
    }

    Ok(())
}

async fn migrate_edges_table(connection: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    if !has_column(connection, "edges", "properties").await? {
        connection
            .execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE edges ADD COLUMN properties TEXT NOT NULL DEFAULT '{}';".to_string(),
            ))
            .await?;
    }
    Ok(())
}

async fn migrate_components_table(connection: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    if !has_column(connection, "components", "fast_payload").await? {
        connection
            .execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE components ADD COLUMN fast_payload TEXT NOT NULL DEFAULT '{}';".to_string(),
            ))
            .await?;
    }
    if !has_column(connection, "components", "custom_payload").await? {
        connection
            .execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "ALTER TABLE components ADD COLUMN custom_payload TEXT NOT NULL DEFAULT '{}';".to_string(),
            ))
            .await?;
    }
    Ok(())
}

async fn nodes_has_legacy_type_column(connection: &DatabaseConnection) -> Result<bool, String> {
    has_column(connection, "nodes", "type")
        .await
        .map_err(|error| error.to_string())
}

async fn persist_node(
    connection: &DatabaseConnection,
    node: &NodeRecord,
    use_legacy_type_column: bool,
) -> Result<(), String> {
    if use_legacy_type_column {
        connection
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                "INSERT OR REPLACE INTO nodes (id, created_at, x, y, node_type, type, content) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                vec![
                    node.id.clone().into(),
                    node.created_at.clone().into(),
                    node.x.into(),
                    node.y.into(),
                    node.node_type.clone().into(),
                    node.node_type.clone().into(),
                    node.content.clone().into(),
                ],
            ))
            .await
            .map_err(|error| error.to_string())?;
    } else {
        connection
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                "INSERT OR REPLACE INTO nodes (id, created_at, x, y, node_type, content) VALUES ($1, $2, $3, $4, $5, $6)",
                vec![
                    node.id.clone().into(),
                    node.created_at.clone().into(),
                    node.x.into(),
                    node.y.into(),
                    node.node_type.clone().into(),
                    node.content.clone().into(),
                ],
            ))
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

async fn persist_edge(connection: &DatabaseConnection, edge: &EdgeRecord) -> Result<(), String> {
    connection
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "INSERT OR REPLACE INTO edges (id, source_id, target_id, relation, properties) VALUES ($1, $2, $3, $4, $5)",
            vec![
                edge.id.clone().into(),
                edge.source_id.clone().into(),
                edge.target_id.clone().into(),
                edge.relation.clone().into(),
                edge.properties.to_string().into(),
            ],
        ))
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

async fn persist_component(
    connection: &DatabaseConnection,
    component: &ComponentRecord,
) -> Result<(), String> {
    connection
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "INSERT OR REPLACE INTO components (id, entity_id, component_type, fast_payload, custom_payload) VALUES ($1, $2, $3, $4, $5)",
            vec![
                component.id.clone().into(),
                component.entity_id.clone().into(),
                component.component_type.clone().into(),
                component.fast_payload.to_string().into(),
                component.custom_payload.to_string().into(),
            ],
        ))
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

async fn delete_node_persisted(connection: &DatabaseConnection, id: &str) -> Result<(), String> {
    let canonical_id = normalize_node_id(id);
    connection
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "DELETE FROM nodes WHERE id = $1",
            vec![canonical_id.clone().into()],
        ))
        .await
        .map_err(|error| error.to_string())?;
    connection
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "DELETE FROM edges WHERE source_id = $1 OR target_id = $1",
            vec![canonical_id.clone().into()],
        ))
        .await
        .map_err(|error| error.to_string())?;
    connection
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "DELETE FROM components WHERE entity_id = $1",
            vec![canonical_id.into()],
        ))
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn parse_json_row(row: &QueryResult, key: &str) -> Value {
    let raw = row.try_get::<String>("", key).unwrap_or_else(|_| "{}".to_string());
    serde_json::from_str(&raw).unwrap_or_else(|_| json!({}))
}

fn node_from_row(row: QueryResult) -> NodeRecord {
    NodeRecord {
        id: normalize_node_id(&row.try_get::<String>("", "id").unwrap_or_default()),
        created_at: row
            .try_get::<String>("", "created_at")
            .unwrap_or_else(|_| now_rfc3339()),
        x: row.try_get("", "x").unwrap_or(0.0),
        y: row.try_get("", "y").unwrap_or(0.0),
        node_type: row
            .try_get::<String>("", "node_type")
            .unwrap_or_else(|_| "note".to_string()),
        content: row.try_get::<String>("", "content").unwrap_or_default(),
    }
}

fn edge_from_row(row: QueryResult) -> EdgeRecord {
    EdgeRecord {
        id: row.try_get::<String>("", "id").unwrap_or_default(),
        source_id: normalize_node_id(&row.try_get::<String>("", "source_id").unwrap_or_default()),
        target_id: normalize_node_id(&row.try_get::<String>("", "target_id").unwrap_or_default()),
        relation: row
            .try_get::<String>("", "relation")
            .unwrap_or_else(|_| "related_to".to_string()),
        properties: parse_json_row(&row, "properties"),
    }
}

fn component_from_row(row: QueryResult) -> ComponentRecord {
    ComponentRecord {
        id: row.try_get::<String>("", "id").unwrap_or_default(),
        entity_id: normalize_node_id(&row.try_get::<String>("", "entity_id").unwrap_or_default()),
        component_type: row
            .try_get::<String>("", "component_type")
            .unwrap_or_else(|_| "custom".to_string()),
        fast_payload: parse_json_row(&row, "fast_payload"),
        custom_payload: parse_json_row(&row, "custom_payload"),
    }
}

async fn load_graph_from_db(connection: &DatabaseConnection) -> Result<GraphManager, String> {
    let node_rows = connection
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT id, created_at, x, y, node_type, content FROM nodes".to_string(),
        ))
        .await
        .map_err(|error| error.to_string())?;

    let edge_rows = connection
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT id, source_id, target_id, relation, properties FROM edges".to_string(),
        ))
        .await
        .map_err(|error| error.to_string())?;

    let component_rows = connection
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT id, entity_id, component_type, fast_payload, custom_payload FROM components"
                .to_string(),
        ))
        .await
        .map_err(|error| error.to_string())?;

    let mut manager = GraphManager::default();
    for row in node_rows {
        manager.upsert_node(node_from_row(row));
    }
    for row in edge_rows {
        manager.upsert_edge(edge_from_row(row));
    }
    for row in component_rows {
        manager.upsert_component(component_from_row(row));
    }

    Ok(manager)
}

fn current_graph_context(snapshot: &GraphSnapshot) -> Value {
    json!({
        "summary": {
            "node_count": snapshot.nodes.len(),
            "edge_count": snapshot.edges.len(),
            "component_count": snapshot.components.len()
        },
        "nodes": snapshot.nodes.iter().map(|node| json!({
            "id": node.id,
            "type": node.node_type,
            "content": node.content,
        })).collect::<Vec<_>>(),
        "edges": snapshot.edges.iter().map(|edge| json!({
            "id": edge.id,
            "source_id": edge.source_id,
            "target_id": edge.target_id,
            "relation": edge.relation,
        })).collect::<Vec<_>>()
    })
}

fn build_forensics_report(snapshot: &GraphSnapshot) -> ForensicsReport {
    let mut findings = Vec::new();

    for edge in &snapshot.edges {
        if edge.relation != "scan_result" {
            continue;
        }

        let target_node = snapshot.nodes.iter().find(|node| node.id == edge.target_id);
        let evidence = target_node
            .map(|node| node.content.clone())
            .filter(|content| !content.trim().is_empty())
            .unwrap_or_else(|| {
                let ports = edge
                    .properties
                    .get("ports")
                    .and_then(|ports| ports.as_array())
                    .map(|ports| {
                        ports
                            .iter()
                            .filter_map(|port| port.as_i64())
                            .map(|port| port.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_else(|| "无端口细节".to_string());
                format!("{} 扫描结果: {}", edge.source_id, ports)
            });

        findings.push(ForensicsFinding {
            node_id: edge.target_id.clone(),
            title: "扫描发现".to_string(),
            relation: edge.relation.clone(),
            evidence,
        });
    }

    let summary = ForensicsSummary {
        node_count: snapshot.nodes.len(),
        edge_count: snapshot.edges.len(),
        component_count: snapshot.components.len(),
        finding_count: findings.len(),
    };

    let generated_at = now_rfc3339();
    let markdown = {
        let mut lines = vec![
            "# CyberWeaver 自动化取证报告".to_string(),
            String::new(),
            format!("- 生成时间: {}", generated_at),
            format!("- 节点数: {}", summary.node_count),
            format!("- 边数: {}", summary.edge_count),
            format!("- 组件数: {}", summary.component_count),
            format!("- 发现数: {}", summary.finding_count),
            String::new(),
            "## 关键发现".to_string(),
        ];

        if findings.is_empty() {
            lines.push(String::new());
            lines.push("- 当前图谱中尚未归纳出明确取证发现。".to_string());
        } else {
            for finding in &findings {
                lines.push(String::new());
                lines.push(format!(
                    "- [{}] {}: {}",
                    finding.node_id, finding.title, finding.evidence
                ));
            }
        }

        lines.join("\n")
    };

    ForensicsReport {
        generated_at,
        summary,
        findings,
        markdown,
    }
}

#[derive(Debug, Deserialize)]
struct PluginOutput {
    message: Option<String>,
    added_nodes: Option<Vec<NodePayload>>,
    added_edges: Option<Vec<EdgePayload>>,
    tokens: Option<Vec<String>>,
}

async fn execute_plugin(
    tool: &RegisteredTool,
    plugins_dir: &Path,
    params: Value,
    context: Value,
) -> Result<PluginOutput, String> {
    let tool = tool.clone();
    let plugins_dir = plugins_dir.to_path_buf();
    tokio::task::spawn_blocking(move || {
        Python::with_gil(|py| -> Result<PluginOutput, String> {
            let sys = PyModule::import_bound(py, "sys").map_err(|error| error.to_string())?;
            let sys_path = sys.getattr("path").map_err(|error| error.to_string())?;
            let path_list = sys_path
                .downcast::<PyList>()
                .map_err(|error| error.to_string())?;
            let plugins_dir_string = plugins_dir.to_string_lossy().to_string();
            path_list
                .insert(0, plugins_dir_string)
                .map_err(|error| error.to_string())?;

            let module = PyModule::import_bound(py, tool.module_name.as_str())
                .map_err(|error| format!("failed to import plugin module {}: {error}", tool.module_name))?;
            let function = module
                .getattr(tool.function_name.as_str())
                .map_err(|error| format!("missing function {} in module {}: {error}", tool.function_name, tool.module_name))?;

            let params_py = pyo3::types::PyString::new_bound(py, &params.to_string());
            let context_py = pyo3::types::PyString::new_bound(py, &context.to_string());
            let json_module = PyModule::import_bound(py, "json").map_err(|error| error.to_string())?;
            let loads = json_module.getattr("loads").map_err(|error| error.to_string())?;
            let params_obj = loads
                .call1((params_py,))
                .map_err(|error| format!("failed to decode params json in Python: {error}"))?;
            let context_obj = loads
                .call1((context_py,))
                .map_err(|error| format!("failed to decode context json in Python: {error}"))?;

            let result = function
                .call1((params_obj, context_obj))
                .map_err(|error| format!("python tool call failed: {error}"))?;

            let dumps = json_module.getattr("dumps").map_err(|error| error.to_string())?;
            let serialized = dumps
                .call1((result,))
                .map_err(|error| format!("failed to encode python output: {error}"))?;
            let text = serialized
                .extract::<String>()
                .map_err(|error| format!("failed to extract python output string: {error}"))?;
            serde_json::from_str::<PluginOutput>(&text)
                .map_err(|error| format!("invalid plugin output: {error}; output={text}"))
        })
    })
    .await
    .map_err(|error| format!("plugin execution join error: {error}"))?
}

async fn upsert_display_component(
    connection: &DatabaseConnection,
    graph: &mut GraphManager,
    node: &NodeRecord,
) -> Result<(), String> {
    let component = ComponentRecord {
        id: format!("display:{}", node.id),
        entity_id: node.id.clone(),
        component_type: "display_meta".to_string(),
        fast_payload: json!({
            "node_type": node.node_type,
            "content": node.content,
        }),
        custom_payload: json!({}),
    };
    persist_component(connection, &component).await?;
    graph.upsert_component(component);
    Ok(())
}

fn node_record_from_payload(payload: NodePayload) -> NodeRecord {
    NodeRecord {
        id: normalize_node_id(&payload.id),
        created_at: now_rfc3339(),
        x: payload.x,
        y: payload.y,
        node_type: payload.r#type,
        content: payload.content,
    }
}

fn edge_record_from_payload(payload: EdgePayload) -> EdgeRecord {
    EdgeRecord {
        id: payload
            .id
            .unwrap_or_else(|| format!("edge-{}", Uuid::new_v4().simple())),
        source_id: normalize_node_id(&payload.source_id),
        target_id: normalize_node_id(&payload.target_id),
        relation: payload.relation,
        properties: payload.properties.unwrap_or_else(|| json!({})),
    }
}

fn handle_ws_command(raw: &str) -> Result<WsCommand, String> {
    serde_json::from_str(raw).map_err(|error| error.to_string())
}

async fn apply_tool_execution(state: Arc<RuntimeState>, execution: ToolExecution) -> Result<(), String> {
    let snapshot = {
        let graph = state.graph.read().await;
        graph.snapshot()
    };
    let context = current_graph_context(&snapshot);

    let tool = {
        let plugins = state.plugins.read().await;
        plugins
            .get(&execution.tool)
            .cloned()
            .ok_or_else(|| format!("unknown tool: {}", execution.tool))?
    };

    let output = execute_plugin(&tool, &state.plugins_dir, execution.params, context).await?;

    let mut added_nodes = Vec::new();
    let mut added_edges = Vec::new();

    if let Some(nodes) = output.added_nodes {
        let mut graph = state.graph.write().await;
        for payload in nodes {
            let node = node_record_from_payload(payload);
            persist_node(&state.db, &node, state.db_uses_legacy_type_column).await?;
            upsert_display_component(&state.db, &mut graph, &node).await?;
            added_nodes.push(node.clone());
            graph.upsert_node(node);
        }
    }

    if let Some(edges) = output.added_edges {
        let mut graph = state.graph.write().await;
        for payload in edges {
            let edge = edge_record_from_payload(payload);
            persist_edge(&state.db, &edge).await?;
            added_edges.push(edge.clone());
            graph.upsert_edge(edge);
        }
    }

    if let Some(message) = output.message {
        if let Some(event) = tool_result_message(tool.name.clone(), true, message) {
            let _ = state.tx.send(event);
        }
    }

    if let Some(tokens) = output.tokens {
        for token in tokens {
            if let Some(event) = agent_token_message(token.clone()) {
                let _ = state.tx.send(event);
            }
        }
    }

    if let Some(event) = graph_update_message(added_nodes, Vec::new(), added_edges, Vec::new()) {
        let _ = state.tx.send(event);
    }

    Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<RuntimeState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn get_graph(State(state): State<Arc<RuntimeState>>) -> Json<GraphSnapshot> {
    let graph = state.graph.read().await;
    Json(graph.snapshot())
}

async fn get_report(State(state): State<Arc<RuntimeState>>) -> Json<ForensicsReport> {
    let graph = state.graph.read().await;
    Json(build_forensics_report(&graph.snapshot()))
}

async fn post_node(
    State(state): State<Arc<RuntimeState>>,
    Json(payload): Json<NodePayload>,
) -> impl IntoResponse {
    let node = node_record_from_payload(payload);
    let mut graph = state.graph.write().await;
    graph.upsert_node(node.clone());
    if let Err(error) = persist_node(&state.db, &node, state.db_uses_legacy_type_column).await {
        return Json(json!({ "ok": false, "error": error }));
    }
    if let Err(error) = upsert_display_component(&state.db, &mut graph, &node).await {
        return Json(json!({ "ok": false, "error": error }));
    }

    if let Some(message) = graph_update_message(vec![node], Vec::new(), Vec::new(), Vec::new()) {
        let _ = state.tx.send(message);
    }
    Json(json!({ "ok": true }))
}

async fn post_edge(
    State(state): State<Arc<RuntimeState>>,
    Json(payload): Json<EdgePayload>,
) -> impl IntoResponse {
    let edge = edge_record_from_payload(payload);
    let mut graph = state.graph.write().await;
    graph.upsert_edge(edge.clone());
    if let Err(error) = persist_edge(&state.db, &edge).await {
        return Json(json!({ "ok": false, "error": error }));
    }
    if let Some(message) = graph_update_message(Vec::new(), Vec::new(), vec![edge], Vec::new()) {
        let _ = state.tx.send(message);
    }
    Json(json!({ "ok": true }))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<RuntimeState>) {
    let mut rx = state.tx.subscribe();
    loop {
        tokio::select! {
            inbound = socket.recv() => match inbound {
                Some(Ok(Message::Text(text))) => {
                    match handle_ws_command(&text) {
                        Ok(WsCommand::DebugCreateNode) => {
                            let node = NodeRecord {
                                id: "ws-demo-1".to_string(),
                                created_at: now_rfc3339(),
                                node_type: "note".to_string(),
                                x: 320.0,
                                y: 180.0,
                                content: "来自 Rust WebSocket 的测试节点".to_string(),
                            };

                            let mut graph = state.graph.write().await;
                            graph.upsert_node(node.clone());
                            let _ = persist_node(&state.db, &node, state.db_uses_legacy_type_column).await;
                            let _ = upsert_display_component(&state.db, &mut graph, &node).await;
                            if let Some(message) = graph_update_message(vec![node], Vec::new(), Vec::new(), Vec::new()) {
                                let _ = state.tx.send(message);
                            }
                        }
                        Ok(WsCommand::ToolExecution { tool, params }) => {
                            let execution = ToolExecution { tool: tool.clone(), params };
                            let run_state = state.clone();
                            tokio::spawn(async move {
                                if let Err(error) = apply_tool_execution(run_state.clone(), execution.clone()).await {
                                    if let Some(event) = tool_result_message(execution.tool, false, error) {
                                        let _ = run_state.tx.send(event);
                                    }
                                }
                            });
                        }
                        Err(error) => {
                            if let Some(event) = tool_result_message("protocol".to_string(), false, format!("invalid command: {error}")) {
                                let _ = state.tx.send(event);
                            }
                        }
                    }
                }
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(_)) => {}
                Some(Err(error)) => {
                    eprintln!("websocket receive error: {error}");
                    break;
                }
            },
            outbound = rx.recv() => match outbound {
                Ok(payload) => {
                    if socket.send(Message::Text(payload.into())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    }
}

#[tauri::command]
async fn save_node(node: NodePayload) -> Result<(), String> {
    let runtime = STATE
        .get()
        .ok_or_else(|| "Runtime state not initialized".to_string())?
        .clone();
    let record = node_record_from_payload(node);
    {
        let mut graph = runtime.graph.write().await;
        graph.upsert_node(record.clone());
        upsert_display_component(&runtime.db, &mut graph, &record).await?;
    }
    persist_node(&runtime.db, &record, runtime.db_uses_legacy_type_column).await?;
    if let Some(message) = graph_update_message(vec![record], Vec::new(), Vec::new(), Vec::new()) {
        let _ = runtime.tx.send(message);
    }
    Ok(())
}

#[tauri::command]
async fn delete_node(id: String) -> Result<(), String> {
    let runtime = STATE
        .get()
        .ok_or_else(|| "Runtime state not initialized".to_string())?
        .clone();
    let normalized = normalize_node_id(&id);
    {
        let mut graph = runtime.graph.write().await;
        graph.remove_node(&normalized);
    }
    delete_node_persisted(&runtime.db, &normalized).await?;
    Ok(())
}

#[derive(Debug, Serialize)]
struct LegacyNodeModel {
    pub id: String,
    pub r#type: String,
    pub x: f64,
    pub y: f64,
    pub content: String,
}

#[tauri::command]
async fn get_nodes() -> Result<Vec<LegacyNodeModel>, String> {
    let runtime = STATE
        .get()
        .ok_or_else(|| "Runtime state not initialized".to_string())?
        .clone();
    let graph = runtime.graph.read().await;
    let snapshot = graph.snapshot();
    Ok(snapshot
        .nodes
        .into_iter()
        .filter(|node| node.id != THOUGHT_NODE_ID)
        .map(|node| LegacyNodeModel {
            id: node.id,
            r#type: node.node_type,
            x: node.x,
            y: node.y,
            content: node.content,
        })
        .collect())
}

#[tauri::command]
fn get_ws_url() -> String {
    WS_URL.to_string()
}

fn build_router(state: Arc<RuntimeState>) -> Router {
    Router::new()
        .route("/ping", get(|| async { "pong" }))
        .route("/graph", get(get_graph))
        .route("/report", get(get_report))
        .route("/node", post(post_node))
        .route("/edge", post(post_edge))
        .route("/ws", get(ws_handler))
        .with_state(state)
}

#[tokio::main]
async fn main() {
    let db = init_db().await.expect("failed to initialize database");
    let loaded_graph = load_graph_from_db(&db).await.unwrap_or_default();
    let db_uses_legacy_type_column = nodes_has_legacy_type_column(&db).await.unwrap_or(false);
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let plugins_dir = cwd.join("plugins");
    let plugins = PluginRegistry::load_from_dir(&plugins_dir);
    let (tx, _) = broadcast::channel::<String>(256);

    let state = Arc::new(RuntimeState {
        db,
        db_uses_legacy_type_column,
        graph: Arc::new(RwLock::new(loaded_graph)),
        tx,
        plugins: Arc::new(RwLock::new(plugins)),
        plugins_dir,
    });

    let _ = STATE.set(state.clone());

    tokio::spawn(async move {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
        let app = build_router(state.clone());
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![save_node, delete_node, get_nodes, get_ws_url])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{
        agent_token_message, build_forensics_report, current_graph_context, graph_update_message, handle_ws_command,
        parse_registered_tool_names, tool_result_message, AgentTokenEvent,
        GraphManager, GraphSnapshot, NodePayload, NodeRecord, PluginRegistry, RuntimeState,
        ToolResultEvent, WsCommand, build_router, init_db_with_url, load_graph_from_db,
        nodes_has_legacy_type_column,
    };
    use axum::body::{to_bytes, Body};
    use axum::http::{Method, Request, StatusCode};
    use std::sync::Arc;
    use std::path::Path;
    use serde_json::json;
    use tokio::sync::{broadcast, RwLock};
    use tower::util::ServiceExt;
    use uuid::Uuid;

    fn sample_node(id: &str) -> NodeRecord {
        NodeRecord {
            id: id.to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            x: 1.0,
            y: 2.0,
            node_type: "note".to_string(),
            content: "hello".to_string(),
        }
    }

    async fn make_test_runtime_state() -> Arc<RuntimeState> {
        let temp_path = std::env::temp_dir().join(format!("cyberweaver-test-{}.db", Uuid::new_v4()));
        let db_url = format!("sqlite://{}?mode=rwc", temp_path.to_string_lossy().replace('\\', "/"));
        let db = init_db_with_url(&db_url).await.expect("db init should succeed");
        let graph = load_graph_from_db(&db).await.unwrap_or_default();
        let db_uses_legacy_type_column = nodes_has_legacy_type_column(&db).await.unwrap_or(false);
        let (tx, _) = broadcast::channel::<String>(64);
        Arc::new(RuntimeState {
            db,
            db_uses_legacy_type_column,
            graph: Arc::new(RwLock::new(graph)),
            tx,
            plugins: Arc::new(RwLock::new(PluginRegistry::load_from_dir(Path::new("../plugins")))),
            plugins_dir: Path::new("../plugins").to_path_buf(),
        })
    }

    async fn exercise_graph_http_round_trip(app: axum::Router) -> GraphSnapshot {
        let node_request = Request::builder()
            .method(Method::POST)
            .uri("/node")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "id": "n-http-1",
                    "type": "note",
                    "x": 12.0,
                    "y": 34.0,
                    "content": "http-node"
                })
                .to_string(),
            ))
            .expect("node request build");

        let node_response = app
            .clone()
            .oneshot(node_request)
            .await
            .expect("node response");
        assert_eq!(node_response.status(), StatusCode::OK);

        let edge_request = Request::builder()
            .method(Method::POST)
            .uri("/edge")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "source_id": "n-http-1",
                    "target_id": "n-http-2",
                    "relation": "linked_to",
                    "properties": {}
                })
                .to_string(),
            ))
            .expect("edge request build");

        let edge_response = app
            .clone()
            .oneshot(edge_request)
            .await
            .expect("edge response");
        assert_eq!(edge_response.status(), StatusCode::OK);

        let graph_request = Request::builder()
            .method(Method::GET)
            .uri("/graph")
            .body(Body::empty())
            .expect("graph request build");

        let graph_response = app
            .oneshot(graph_request)
            .await
            .expect("graph response");
        assert_eq!(graph_response.status(), StatusCode::OK);

        let body = to_bytes(graph_response.into_body(), usize::MAX)
            .await
            .expect("graph body bytes");
        serde_json::from_slice::<GraphSnapshot>(&body).expect("graph snapshot parse")
    }

    async fn exercise_websocket_tool_execution(runtime: Arc<RuntimeState>) -> Vec<String> {
        let mut rx = runtime.tx.subscribe();
        {
            let mut graph = runtime.graph.write().await;
            graph.upsert_node(sample_node("seed-host"));
        }
        let _ = super::persist_node(&runtime.db, &sample_node("seed-host"), runtime.db_uses_legacy_type_column).await;

        super::apply_tool_execution(
            runtime.clone(),
            super::ToolExecution {
                tool: "scan_port".to_string(),
                params: json!({ "target_id": "seed-host" }),
            },
        )
        .await
        .expect("tool execution should succeed");

        let mut messages = Vec::new();
        while let Ok(message) = rx.try_recv() {
            messages.push(message);
        }
        messages
    }

    #[test]
    fn graph_manager_upsert_and_snapshot() {
        let mut manager = GraphManager::default();
        manager.upsert_node(sample_node("n1"));
        manager.upsert_node(sample_node("n2"));
        let snapshot = manager.snapshot();
        assert_eq!(snapshot.nodes.len(), 2);
        assert_eq!(snapshot.nodes[0].id, "n1");
    }

    #[test]
    fn graph_manager_removes_related_edges_and_components() {
        let mut manager = GraphManager::default();
        manager.upsert_node(sample_node("n1"));
        manager.upsert_node(sample_node("n2"));
        manager.upsert_edge(super::EdgeRecord {
            id: "e1".to_string(),
            source_id: "n1".to_string(),
            target_id: "n2".to_string(),
            relation: "linked".to_string(),
            properties: json!({}),
        });
        manager.upsert_component(super::ComponentRecord {
            id: "c1".to_string(),
            entity_id: "n1".to_string(),
            component_type: "display_meta".to_string(),
            fast_payload: json!({}),
            custom_payload: json!({}),
        });
        manager.remove_node("n1");
        let snapshot = manager.snapshot();
        assert_eq!(snapshot.nodes.len(), 1);
        assert!(snapshot.edges.is_empty());
        assert!(snapshot.components.is_empty());
    }

    #[test]
    fn parse_debug_create_node_command() {
        let command = handle_ws_command(r#"{"type":"debug_create_node"}"#).unwrap();
        assert!(matches!(command, WsCommand::DebugCreateNode));
    }

    #[test]
    fn parse_tool_execution_command() {
        let command = handle_ws_command(
            r#"{"type":"tool_execution","tool":"scan_port","params":{"target_id":"n1"}}"#,
        )
        .unwrap();
        match command {
            WsCommand::ToolExecution { tool, params } => {
                assert_eq!(tool, "scan_port");
                assert_eq!(params["target_id"], "n1");
            }
            _ => panic!("expected tool execution command"),
        }
    }

    #[test]
    fn reject_unknown_ws_command() {
        assert!(handle_ws_command(r#"{"type":"unknown"}"#).is_err());
    }

    #[test]
    fn graph_update_contains_edges_and_nodes() {
        let node = sample_node("n1");
        let edge = super::EdgeRecord {
            id: "e1".to_string(),
            source_id: "n1".to_string(),
            target_id: "n2".to_string(),
            relation: "scan_result".to_string(),
            properties: json!({ "ports": [80] }),
        };
        let message = graph_update_message(vec![node], Vec::new(), vec![edge], Vec::new()).unwrap();
        assert!(message.contains("\"added_nodes\""));
        assert!(message.contains("\"added_edges\""));
    }

    #[test]
    fn parse_registered_tool_names_from_sdk_style() {
        let content = r#"
from sdk import register_tool

@register_tool("scan_port")
def scan_port(ctx):
    pass

@register_tool("collect_process")
def collect_process(ctx):
    pass
"#;

        let names = parse_registered_tool_names(content);
        assert_eq!(names, vec!["scan_port".to_string(), "collect_process".to_string()]);
    }

    #[test]
    fn python_safe_module_name_rejects_invalid_stem() {
        let valid = super::python_safe_module_name(Path::new("plugins/scan_port.py"));
        assert_eq!(valid, Some("scan_port".to_string()));

        let invalid = super::python_safe_module_name(Path::new("plugins/scan-port.py"));
        assert_eq!(invalid, None);
    }

    #[test]
    fn graph_context_includes_summary_counts() {
        let mut manager = GraphManager::default();
        manager.upsert_node(sample_node("n1"));
        manager.upsert_node(sample_node("n2"));
        let context = current_graph_context(&manager.snapshot());
        assert_eq!(context["summary"]["node_count"], 2);
    }

    #[test]
    fn tool_result_event_serialization_shape() {
        let json = serde_json::to_string(&ToolResultEvent {
            r#type: "tool_result",
            tool: "scan_port".to_string(),
            ok: true,
            message: "done".to_string(),
        })
        .unwrap();
        assert!(json.contains("\"type\":\"tool_result\""));
    }

    #[test]
    fn agent_token_event_serialization_shape() {
        let json = serde_json::to_string(&AgentTokenEvent {
            r#type: "agent_token",
            token: "thinking".to_string(),
        })
        .unwrap();
        assert!(json.contains("\"token\":\"thinking\""));
    }

    #[test]
    fn node_payload_conversion_preserves_coordinates() {
        let payload = NodePayload {
            id: "shape:test".to_string(),
            r#type: "note".to_string(),
            x: 33.0,
            y: 66.0,
            content: "abc".to_string(),
        };
        let node = super::node_record_from_payload(payload);
        assert_eq!(node.id, "test");
        assert_eq!(node.x, 33.0);
        assert_eq!(node.y, 66.0);
    }

    #[test]
    fn tool_protocol_messages_form_expected_smoke_contract() {
        let result = tool_result_message("scan_port".to_string(), true, "ok".to_string()).unwrap();
        let token = agent_token_message("thinking".to_string()).unwrap();

        assert!(result.contains("\"type\":\"tool_result\""));
        assert!(result.contains("\"tool\":\"scan_port\""));
        assert!(token.contains("\"type\":\"agent_token\""));
        assert!(token.contains("\"token\":\"thinking\""));
    }

    #[test]
    fn build_forensics_report_summarizes_scan_findings() {
        let mut manager = GraphManager::default();
        manager.upsert_node(NodeRecord {
            id: "seed-host".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            x: 12.0,
            y: 24.0,
            node_type: "note".to_string(),
            content: "Host: 192.168.1.10".to_string(),
        });
        manager.upsert_node(NodeRecord {
            id: "scan:seed-host".to_string(),
            created_at: "2026-01-01T00:01:00Z".to_string(),
            x: 220.0,
            y: 180.0,
            node_type: "note".to_string(),
            content: "seed-host 开放端口: 22, 80, 443".to_string(),
        });
        manager.upsert_edge(super::EdgeRecord {
            id: "e-scan".to_string(),
            source_id: "seed-host".to_string(),
            target_id: "scan:seed-host".to_string(),
            relation: "scan_result".to_string(),
            properties: json!({ "ports": [22, 80, 443] }),
        });

        let report = build_forensics_report(&manager.snapshot());

        assert_eq!(report.summary.node_count, 2);
        assert_eq!(report.summary.edge_count, 1);
        assert_eq!(report.summary.finding_count, 1);
        assert!(report.markdown.contains("自动化取证报告"));
        assert!(report.markdown.contains("seed-host 开放端口: 22, 80, 443"));
    }

    #[tokio::test]
    async fn graph_http_routes_round_trip_node_and_edge() {
        let runtime = make_test_runtime_state().await;
        let app = build_router(runtime.clone());

        let graph = exercise_graph_http_round_trip(app).await;

        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.nodes[0].id, "n-http-1");
        assert_eq!(graph.edges[0].relation, "linked_to");
    }

    #[tokio::test]
    async fn report_http_route_returns_summary_and_markdown() {
        let runtime = make_test_runtime_state().await;
        let app = build_router(runtime.clone());
        let _ = exercise_graph_http_round_trip(app.clone()).await;

        let report_request = Request::builder()
            .method(Method::GET)
            .uri("/report")
            .body(Body::empty())
            .expect("report request build");

        let report_response = app
            .oneshot(report_request)
            .await
            .expect("report response");
        assert_eq!(report_response.status(), StatusCode::OK);

        let body = to_bytes(report_response.into_body(), usize::MAX)
            .await
            .expect("report body bytes");
        let report = serde_json::from_slice::<serde_json::Value>(&body).expect("report parse");

        assert_eq!(report["summary"]["node_count"], 1);
        assert_eq!(report["summary"]["edge_count"], 1);
        assert!(report["markdown"]
            .as_str()
            .unwrap_or_default()
            .contains("CyberWeaver 自动化取证报告"));
    }

    #[tokio::test]
    async fn websocket_tool_execution_emits_graph_update_and_tool_result() {
        let runtime = make_test_runtime_state().await;
        let ws_messages = exercise_websocket_tool_execution(runtime).await;

        assert!(ws_messages.iter().any(|message| message.contains("\"type\":\"graph_update\"")));
        assert!(ws_messages.iter().any(|message| message.contains("\"type\":\"tool_result\"")));
    }

    #[test]
    fn plugin_registry_loads_timestamp_and_reverse_geocode_tools() {
        let registry = PluginRegistry::load_from_dir(Path::new("../plugins"));
        assert!(registry.get("timestamp_convert").is_some());
        assert!(registry.get("reverse_geocode").is_some());
    }

    #[tokio::test]
    async fn websocket_timestamp_convert_execution_emits_tool_result() {
        let runtime = make_test_runtime_state().await;
        let mut rx = runtime.tx.subscribe();

        super::apply_tool_execution(
            runtime,
            super::ToolExecution {
                tool: "timestamp_convert".to_string(),
                params: json!({ "value": "1711603200" }),
            },
        )
        .await
        .expect("timestamp_convert execution should succeed");

        let mut messages = Vec::new();
        while let Ok(message) = rx.try_recv() {
            messages.push(message);
        }
        assert!(messages.iter().any(|message| message.contains("\"type\":\"tool_result\"")));
        assert!(messages.iter().any(|message| message.contains("\"tool\":\"timestamp_convert\"")));
        assert!(messages.iter().any(|message| message.contains("\"type\":\"graph_update\"")));
    }

    #[tokio::test]
    async fn websocket_reverse_geocode_execution_emits_tool_result() {
        let runtime = make_test_runtime_state().await;
        let mut rx = runtime.tx.subscribe();

        super::apply_tool_execution(
            runtime,
            super::ToolExecution {
                tool: "reverse_geocode".to_string(),
                params: json!({
                    "latitude": 39.9042,
                    "longitude": 116.4074,
                    "mock_response": {
                        "display_name": "Chaoyang, Beijing, China",
                        "address": {
                            "country": "China",
                            "state": "Beijing",
                            "city": "Beijing",
                            "suburb": "Chaoyang"
                        }
                    }
                }),
            },
        )
        .await
        .expect("reverse_geocode execution should succeed");

        let mut messages = Vec::new();
        while let Ok(message) = rx.try_recv() {
            messages.push(message);
        }
        assert!(messages.iter().any(|message| message.contains("\"type\":\"tool_result\"")));
        assert!(messages.iter().any(|message| message.contains("\"tool\":\"reverse_geocode\"")));
        assert!(messages.iter().any(|message| message.contains("\"type\":\"graph_update\"")));
    }
}
