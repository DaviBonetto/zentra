// prompt_engine/llm/openrouter.rs — OpenRouter LLM adapter

use super::LLMAdapter;
use crate::prompt_engine::types::LLMError;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const OPENROUTER_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const PRIMARY_MODEL: &str = "deepseek/deepseek-r1-0528:free";
const FALLBACK_MODEL: &str = "meta-llama/llama-3.1-8b-instruct:free";

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

pub struct OpenRouterAdapter {
    client: Client,
    api_key: String,
}

impl OpenRouterAdapter {
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();

        Self { client, api_key }
    }

    async fn call_model(&self, model: &str, prompt: &str) -> Result<String, LLMError> {
        let request = ChatRequest {
            model: model.to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens: 2048,
            temperature: 0.3,
        };

        let response = self
            .client
            .post(OPENROUTER_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://voice-ai-project.local")
            .header("X-Title", "Voice AI Prompt Engine")
            .json(&request)
            .send()
            .await
            .map_err(|e| LLMError::NetworkError(format!("OpenRouter: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(LLMError::ProviderError(format!(
                "OpenRouter {} ({}): {}",
                model, status, body
            )));
        }

        let chat: ChatResponse = response
            .json()
            .await
            .map_err(|e| LLMError::ProviderError(format!("OpenRouter parse: {}", e)))?;

        chat.choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or(LLMError::InvalidResponse)
    }
}

#[async_trait]
impl LLMAdapter for OpenRouterAdapter {
    async fn generate(&self, prompt: &str) -> Result<String, LLMError> {
        // Try primary model first, then fallback
        match self.call_model(PRIMARY_MODEL, prompt).await {
            Ok(text) => Ok(text),
            Err(e) => {
                tracing::warn!("OpenRouter primary failed: {:?}, trying fallback", e);
                self.call_model(FALLBACK_MODEL, prompt).await
            }
        }
    }

    fn name(&self) -> &str {
        "openrouter"
    }
}
