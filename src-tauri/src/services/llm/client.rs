//! LLM API client — OpenAI-compatible chat completions.

use crate::error::AppError;
use crate::state::LlmConfig;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    content: String,
}

pub struct LlmClient {
    config: LlmConfig,
    http_client: reqwest::Client,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    fn validate_config(&self) -> Result<(), AppError> {
        if !self.config.configured {
            return Err(AppError::LlmService("LLM not configured".into()));
        }
        if self.config.api_base.trim().is_empty() {
            return Err(AppError::LlmService("LLM api_base is empty".into()));
        }
        if self.config.model.trim().is_empty() {
            return Err(AppError::LlmService("LLM model is empty".into()));
        }
        Ok(())
    }

    pub async fn chat(&self, system_prompt: &str, user_message: &str) -> Result<String, AppError> {
        self.validate_config()?;

        let url = format!(
            "{}/v1/chat/completions",
            self.config.api_base.trim_end_matches('/')
        );
        let body = ChatRequest {
            model: self.config.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: system_prompt.into(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: user_message.into(),
                },
            ],
            temperature: 0.3,
            max_tokens: Some(4096),
        };

        let mut request = self
            .http_client
            .post(&url)
            .header(CONTENT_TYPE, "application/json");

        if !self.config.api_key.trim().is_empty() {
            request = request.header(
                AUTHORIZATION,
                format!("Bearer {}", self.config.api_key.trim()),
            );
        }

        let response = request
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::LlmService(format!("HTTP error: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = match response.text().await {
                Ok(text) => text,
                Err(error) => format!("<failed to read response body: {error}>"),
            };
            return Err(AppError::LlmService(format!("API {status}: {body}")));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| AppError::LlmService(format!("Parse error: {e}")))?;

        chat_response
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .ok_or_else(|| AppError::LlmService("No choices".into()))
    }
}
