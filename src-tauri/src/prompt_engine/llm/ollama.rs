// prompt_engine/llm/ollama.rs — Ollama local LLM adapter

use super::LLMAdapter;
use crate::prompt_engine::types::LLMError;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const OLLAMA_URL: &str = "http://localhost:11434/api/generate";
const DEFAULT_MODEL: &str = "qwen2.5:1.5b";
const FALLBACK_MODEL: &str = "llama3.2";
const TERTIARY_MODEL: &str = "mistral";

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: u32,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

pub struct OllamaAdapter {
    client: Client,
}

impl OllamaAdapter {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self { client }
    }

    async fn call_model(&self, model: &str, prompt: &str) -> Result<String, LLMError> {
        let request = OllamaRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            stream: false,
            options: OllamaOptions {
                temperature: 0.3,
                num_predict: 2048,
            },
        };

        let response = self
            .client
            .post(OLLAMA_URL)
            .json(&request)
            .send()
            .await
            .map_err(|e| LLMError::NetworkError(format!("Ollama: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(LLMError::ProviderError(format!(
                "Ollama {} ({}): {}",
                model, status, body
            )));
        }

        let ollama: OllamaResponse = response
            .json()
            .await
            .map_err(|e| LLMError::ProviderError(format!("Ollama parse: {}", e)))?;

        if ollama.response.trim().is_empty() {
            return Err(LLMError::InvalidResponse);
        }

        Ok(ollama.response)
    }
}

#[async_trait]
impl LLMAdapter for OllamaAdapter {
    async fn generate(&self, prompt: &str) -> Result<String, LLMError> {
        // Try qwen2.5:1.5b → llama3.2 → mistral
        match self.call_model(DEFAULT_MODEL, prompt).await {
            Ok(text) => Ok(text),
            Err(e1) => {
                tracing::warn!("Ollama '{}' failed: {:?}, trying '{}'", DEFAULT_MODEL, e1, FALLBACK_MODEL);
                match self.call_model(FALLBACK_MODEL, prompt).await {
                    Ok(text) => Ok(text),
                    Err(e2) => {
                        tracing::warn!("Ollama '{}' failed: {:?}, trying '{}'", FALLBACK_MODEL, e2, TERTIARY_MODEL);
                        self.call_model(TERTIARY_MODEL, prompt).await
                    }
                }
            }
        }
    }

    fn name(&self) -> &str {
        "ollama"
    }
}
