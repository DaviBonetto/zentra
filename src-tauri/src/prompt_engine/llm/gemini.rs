// prompt_engine/llm/gemini.rs — Google Gemini LLM adapter

use super::LLMAdapter;
use crate::prompt_engine::types::LLMError;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const MODEL: &str = "gemini-2.5-flash";

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    temperature: f32,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: CandidateContent,
}

#[derive(Deserialize)]
struct CandidateContent {
    parts: Vec<CandidatePart>,
}

#[derive(Deserialize)]
struct CandidatePart {
    text: String,
}

pub struct GeminiAdapter {
    client: Client,
    api_key: String,
}

impl GeminiAdapter {
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap_or_default();

        Self { client, api_key }
    }
}

#[async_trait]
impl LLMAdapter for GeminiAdapter {
    async fn generate(&self, prompt: &str) -> Result<String, LLMError> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            MODEL, self.api_key
        );

        let request = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
            generation_config: GenerationConfig {
                temperature: 0.3,
                max_output_tokens: 2048,
            },
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| LLMError::NetworkError(format!("Gemini: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(LLMError::ProviderError(format!(
                "Gemini {}: {}", status, body
            )));
        }

        let gemini: GeminiResponse = response
            .json()
            .await
            .map_err(|e| LLMError::ProviderError(format!("Gemini parse: {}", e)))?;

        gemini
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or(LLMError::InvalidResponse)
    }

    fn name(&self) -> &str {
        "gemini"
    }
}
