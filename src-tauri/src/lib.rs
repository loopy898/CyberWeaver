//! CyberWeaver — a digital forensics workbench built on Tauri v2.
//!
//! This library crate is the application core. It declares all modules,
//! initializes shared state (database, WebSocket, AI), and exposes a
//! single `run()` entry point called by `main.rs`.

pub mod ai;
pub mod commands;
pub mod db;
pub mod error;
pub mod graph;
pub mod models;
pub mod plugins;
pub mod services;
pub mod state;
pub mod ws;

use db::connection::init_db;
use db::migrations::run_migrations;
use error::AppError;
use plugins::registry::ToolRegistry;
use state::AppState;

/// Bootstrap the Tauri application, Axum server, and all background tasks.
///
/// Called once from `main.rs`. Initializes the database, runs migrations,
/// creates shared application state, and starts the WebSocket broadcast server.
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Scheme A: block on async DB initialization inside Tauri setup.
            // By this point Tauri has already started its tokio runtime.
            let db = tokio::runtime::Handle::current().block_on(async {
                let db = init_db()
                    .await
                    .map_err(|error| -> Box<dyn std::error::Error> { Box::new(error) })?;
                run_migrations(&db)
                    .await
                    .map_err(|error| -> Box<dyn std::error::Error> { Box::new(error) })?;
                Ok::<_, Box<dyn std::error::Error>>(db)
            })?;

            let mut tool_registry = ToolRegistry::new();
            for tool in cw_plugins_builtin::builtin_tools() {
                tool_registry.register_builtin(tool);
            }
            let app_state = AppState::new(db, tool_registry);

            // Spawn the Axum WebSocket broadcast server as a background task.
            let ws_tx = app_state.ws_broadcast.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = start_axum_server(ws_tx).await {
                    eprintln!("axum server exited with error: {error}");
                }
            });

            // Register shared state so Tauri commands can access it via State<AppState>.
            use tauri::Manager;
            app.manage(app_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::node_cmd::get_nodes,
            commands::node_cmd::get_node,
            commands::node_cmd::create_node,
            commands::node_cmd::update_node,
            commands::node_cmd::delete_node,
            commands::relation_cmd::get_relations,
            commands::relation_cmd::get_node_relations,
            commands::relation_cmd::create_relation,
            commands::relation_cmd::delete_relation,
            commands::traversal_cmd::expand_node,
            commands::traversal_cmd::find_path,
            commands::traversal_cmd::get_component,
            commands::traversal_cmd::get_graph_summary,
            commands::llm_cmd::configure_llm,
            commands::llm_cmd::get_llm_config,
            commands::llm_cmd::extract_from_text,
            commands::llm_cmd::extract_relations_from_text,
            commands::llm_cmd::suggest_next_steps,
            commands::llm_cmd::agent_analyze,
            commands::llm_cmd::agent_apply_approvals,
            commands::import_cmd::import_json_canvas,
            commands::import_cmd::import_stix,
            commands::import_cmd::import_attack_flow,
            commands::export_cmd::export_json_canvas,
            commands::export_cmd::export_stix,
            commands::export_cmd::export_attack_flow,
            commands::export_cmd::export_report,
        ])
        .run(tauri::generate_context!())
}

// ---------------------------------------------------------------------------
// Axum WebSocket server (background task)
// ---------------------------------------------------------------------------

async fn start_axum_server(tx: tokio::sync::broadcast::Sender<String>) -> Result<(), AppError> {
    use axum::{
        extract::ws::{Message, WebSocket, WebSocketUpgrade},
        extract::State,
        routing::get,
        Router,
    };
    use std::net::SocketAddr;

    #[derive(Clone)]
    struct WsState {
        tx: tokio::sync::broadcast::Sender<String>,
    }

    async fn handle_socket(mut socket: WebSocket, state: WsState) {
        let mut rx = state.tx.subscribe();

        loop {
            tokio::select! {
                inbound = socket.recv() => match inbound {
                    Some(Ok(Message::Text(_text))) => {
                        // Placeholder: handle client messages in later phases
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
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                },
            }
        }
    }

    let state = WsState { tx };

    let app = Router::new()
        .route("/ping", get(|| async { "pong" }))
        .route(
            "/ws",
            get(
                |ws: WebSocketUpgrade, State(state): State<WsState>| async move {
                    ws.on_upgrade(move |socket| handle_socket(socket, state))
                },
            ),
        )
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Mobile entry point
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run_mobile() {
    if let Err(error) = run() {
        eprintln!("failed to run mobile application: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_db() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("failed to connect sqlite");

        init_schema(&db).await.expect("failed to init schema");
        db
    }

    #[tokio::test]
    async fn upsert_and_get_nodes_roundtrip() {
        let db = create_test_db().await;

        upsert_nodes_internal(
            &db,
            vec![NodePayload {
                id: "artifact-1".to_owned(),
                node_type: "text".to_owned(),
                x: 12.0,
                y: 34.0,
                content: "IOC discovered".to_owned(),
                width: Some(200.0),
                height: None,
            }],
        )
        .await
        .expect("upsert should succeed");

        let rows = list_nodes_internal(&db)
            .await
            .expect("query should succeed");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "shape:artifact-1");
        assert_eq!(rows[0].node_type, "text");
        assert_eq!(rows[0].content, "IOC discovered");
        assert_eq!(rows[0].width, Some(200.0));
    }

    #[tokio::test]
    async fn delete_nodes_removes_rows() {
        let db = create_test_db().await;

        upsert_nodes_internal(
            &db,
            vec![NodePayload {
                id: "shape:artifact-2".to_owned(),
                node_type: "note".to_owned(),
                x: 0.0,
                y: 0.0,
                content: "temporary".to_owned(),
                width: None,
                height: None,
            }],
        )
        .await
        .expect("upsert should succeed");

        delete_nodes_internal(&db, vec!["artifact-2".to_owned()])
            .await
            .expect("delete should succeed");

        let rows = list_nodes_internal(&db)
            .await
            .expect("query should succeed");
        assert!(rows.is_empty());
    }

    #[test]
    fn validate_payload_rejects_unknown_shape_types() {
        let payload = NodePayload {
            id: "shape:x".to_owned(),
            node_type: "draw".to_owned(),
            x: 0.0,
            y: 0.0,
            content: String::new(),
            width: None,
            height: None,
        };

        let result = validate_node_payload(&payload);
        assert!(result.is_err());
    }
}
