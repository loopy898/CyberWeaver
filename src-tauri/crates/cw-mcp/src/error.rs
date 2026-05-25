use sea_orm::DbErr;
use tauri_app_lib::error::AppError;

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error(transparent)]
    App(AppError),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<AppError> for McpError {
    fn from(error: AppError) -> Self {
        Self::App(error)
    }
}

impl From<DbErr> for McpError {
    fn from(error: DbErr) -> Self {
        Self::App(AppError::from(error))
    }
}

impl From<rmcp::service::ServerInitializeError> for McpError {
    fn from(error: rmcp::service::ServerInitializeError) -> Self {
        Self::Internal(error.to_string())
    }
}

impl From<tokio::task::JoinError> for McpError {
    fn from(error: tokio::task::JoinError) -> Self {
        Self::Internal(error.to_string())
    }
}

impl From<McpError> for rmcp::ErrorData {
    fn from(error: McpError) -> Self {
        match error {
            McpError::App(app_error) => rmcp::ErrorData::internal_error(app_error.to_string(), None),
            McpError::NotFound(message) => rmcp::ErrorData::invalid_params(message, None),
            McpError::Internal(message) => rmcp::ErrorData::internal_error(message, None),
        }
    }
}
