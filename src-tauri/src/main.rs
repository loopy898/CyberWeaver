#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use sea_orm::{
    ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, QueryResult, Statement,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::sync::{broadcast, OnceCell};

static DB: OnceCell<DatabaseConnection> = OnceCell::const_new();
const DB_URL: &str = "sqlite://cyberweaver.db?mode=rwc";
const SHAPE_PREFIX: &str = "shape:";

#[derive(Clone)]
struct WsState {
    tx: broadcast::Sender<String>,
}

fn normalize_node_id(id: &str) -> String {
    id.trim().trim_start_matches(SHAPE_PREFIX).to_string()
}

async fn init_db() -> Result<(), sea_orm::DbErr> {
    let db = Database::connect(DB_URL).await?;

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
            type TEXT NOT NULL,
            x REAL NOT NULL,
            y REAL NOT NULL,
            content TEXT NOT NULL DEFAULT ''
        );"
        .to_string(),
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "DELETE FROM nodes
         WHERE id LIKE 'shape:%'
           AND EXISTS (
             SELECT 1 FROM nodes AS canonical
             WHERE canonical.id = substr(nodes.id, 7)
           );"
        .to_string(),
    ))
    .await?;
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        "UPDATE nodes SET id = substr(id, 7) WHERE id LIKE 'shape:%';".to_string(),
    ))
    .await?;

    let _ = DB.set(db);
    Ok(())
}

#[derive(Debug, Deserialize)]
struct NodePayload {
    pub id: String,
    pub r#type: String,
    pub x: f64,
    pub y: f64,
    pub content: String,
}

#[derive(Debug, Serialize)]
struct NodeModel {
    pub id: String,
    pub r#type: String,
    pub x: f64,
    pub y: f64,
    pub content: String,
}

impl NodeModel {
    fn from_row(row: QueryResult) -> Self {
        Self {
            id: normalize_node_id(&row.try_get::<String>("", "id").unwrap_or_default()),
            r#type: row.try_get("", "type").unwrap_or_default(),
            x: row.try_get("", "x").unwrap_or(0.0),
            y: row.try_get("", "y").unwrap_or(0.0),
            content: row.try_get("", "content").unwrap_or_default(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum WsCommand {
    #[serde(rename = "debug_create_node")]
    DebugCreateNode,
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
struct GraphDelta {
    added_nodes: Vec<GraphUpdateNode>,
    updated_nodes: Vec<GraphUpdateNode>,
}

#[derive(Debug, Serialize)]
struct GraphUpdateEvent {
    r#type: &'static str,
    delta: GraphDelta,
}

fn handle_ws_command(raw: &str) -> Option<String> {
    let command: WsCommand = serde_json::from_str(raw).ok()?;
    match command {
        WsCommand::DebugCreateNode => serde_json::to_string(&GraphUpdateEvent {
            r#type: "graph_update",
            delta: GraphDelta {
                added_nodes: vec![GraphUpdateNode {
                    id: "ws-demo-1".to_string(),
                    node_type: "note".to_string(),
                    x: 320.0,
                    y: 180.0,
                    content: "来自 Rust WebSocket 的测试节点".to_string(),
                }],
                updated_nodes: vec![],
            },
        })
        .ok(),
    }
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<WsState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: WsState) {
    let mut rx = state.tx.subscribe();

    loop {
        tokio::select! {
            inbound = socket.recv() => match inbound {
                Some(Ok(Message::Text(text))) => {
                    if let Some(response) = handle_ws_command(&text) {
                        let _ = state.tx.send(response);
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
    let db = DB.get().ok_or("Database Not Ready")?;
    let node_id = normalize_node_id(&node.id);
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "INSERT OR REPLACE INTO nodes (id, type, x, y, content) VALUES ($1, $2, $3, $4, $5)",
        vec![
            node_id.into(),
            node.r#type.into(),
            node.x.into(),
            node.y.into(),
            node.content.into(),
        ],
    ))
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn delete_node(id: String) -> Result<(), String> {
    let db = DB.get().ok_or("Database Not Ready")?;
    let canonical_id = normalize_node_id(&id);
    let prefixed_id = format!("{SHAPE_PREFIX}{canonical_id}");
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "DELETE FROM nodes WHERE id = $1 OR id = $2",
        vec![canonical_id.into(), prefixed_id.into()],
    ))
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn get_nodes() -> Result<Vec<NodeModel>, String> {
    let db = DB.get().ok_or("Database Not Ready")?;
    let rows = db
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT id, type, x, y, content FROM nodes ORDER BY rowid ASC".to_string(),
        ))
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(NodeModel::from_row).collect())
}

#[tokio::main]
async fn main() {
    let (tx, _) = broadcast::channel::<String>(32);
    let ws_state = WsState { tx };

    tokio::spawn(async move {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
        let app = Router::new()
            .route("/ping", get(|| async { "pong" }))
            .route("/ws", get(ws_handler))
            .with_state(ws_state);
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });
    let _ = init_db().await;
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![save_node, delete_node, get_nodes])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{handle_ws_command, normalize_node_id};

    #[test]
    fn normalize_node_id_strips_shape_prefix() {
        assert_eq!(normalize_node_id("shape:abc"), "abc");
        assert_eq!(normalize_node_id("abc"), "abc");
        assert_eq!(normalize_node_id(" shape:def "), "def");
    }

    #[test]
    fn debug_create_node_command_emits_graph_update_message() {
        let message = handle_ws_command(r#"{"type":"debug_create_node"}"#).unwrap();
        assert!(message.contains(r#""type":"graph_update""#));
        assert!(message.contains(r#""node_type":"note""#));
    }

    #[test]
    fn unknown_ws_command_is_rejected() {
        assert!(handle_ws_command(r#"{"type":"unknown"}"#).is_none());
    }
}
