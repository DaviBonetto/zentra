// prompt_engine/llm/groq.rs — Groq LLM adapter (chat completions)

use super::LLMAdapter;
use crate::prompt_engine::types::LLMError;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const GROQ_CHAT_URL: &str = "https://api.groq.com/openai/v1/chat/completions";
const MODEL: &str = "llama-3.3-70b-versatile";

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

pub struct GroqLLMAdapter {
    client: Client,
    api_key: String,
}

impl GroqLLMAdapter {
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();

        Self { client, api_key }
    }
}

#[async_trait]
impl LLMAdapter for GroqLLMAdapter {
    async fn generate(&self, prompt: &str) -> Result<String, LLMError> {
        let request = ChatRequest {
            model: MODEL.to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens: 2048,
            temperature: 0.3,
        };

        let response = self
            .client
            .post(GROQ_CHAT_URL)
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|e| LLMError::NetworkError(format!("Groq: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(LLMError::ProviderError(format!(
                "Groq {}: {}", status, body
            )));
        }

        let chat: ChatResponse = response
            .json()
            .await
            .map_err(|e| LLMError::ProviderError(format!("Groq parse: {}", e)))?;

        chat.choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or(LLMError::InvalidResponse)
    }

    fn name(&self) -> &str {
        "groq"
    }
}
