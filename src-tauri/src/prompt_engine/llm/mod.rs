// prompt_engine/llm/mod.rs — LLM trait + failover orchestrator

pub mod openrouter;
pub mod groq;
pub mod gemini;
pub mod ollama;

use super::types::LLMError;
use async_trait::async_trait;

/// Trait for LLM text generation adapters
#[async_trait]
pub trait LLMAdapter: Send + Sync {
    /// Generate text from prompt
    async fn generate(&self, prompt: &str) -> Result<String, LLMError>;

    /// Provider name
    fn name(&self) -> &str;
}

/// LLM Orchestrator with sequential failover
pub struct LLMOrchestrator {
    providers: Vec<Box<dyn LLMAdapter>>,
}

impl LLMOrchestrator {
    /// Create from environment variables — attempts all available providers
    pub fn from_env() -> Self {
        let mut providers: Vec<Box<dyn LLMAdapter>> = Vec::new();

        // 1. OpenRouter (primary)
        if let Ok(key) = std::env::var("OPENROUTER_API_KEY") {
            if !key.is_empty() {
                providers.push(Box::new(openrouter::OpenRouterAdapter::new(key)));
                tracing::info!("LLM: OpenRouter adapter loaded");
            }
        }

        // 2. Groq (secondary)
        if let Ok(key) = std::env::var("GROQ_API_KEY") {
            if key.starts_with("gsk_") {
                providers.push(Box::new(groq::GroqLLMAdapter::new(key)));
                tracing::info!("LLM: Groq adapter loaded");
            }
        }

        // 3. Gemini (tertiary)
        if let Ok(key) = std::env::var("GEMINI_API_KEY") {
            if !key.is_empty() {
                providers.push(Box::new(gemini::GeminiAdapter::new(key)));
                tracing::info!("LLM: Gemini adapter loaded");
            }
        }

        // 4. Ollama (local fallback — always available)
        providers.push(Box::new(ollama::OllamaAdapter::new()));
        tracing::info!("LLM: Ollama adapter loaded (local fallback)");

        tracing::info!("LLM Orchestrator: {} providers available", providers.len());

        Self { providers }
    }

    /// Generate text with failover across all providers
    pub async fn generate(&self, prompt: &str) -> Result<(String, String), LLMError> {
        let mut last_error = LLMError::AllProvidersFailed;

        for provider in &self.providers {
            tracing::info!("LLM: Trying provider '{}'...", provider.name());

            match provider.generate(prompt).await {
                Ok(text) => {
                    tracing::info!(
                        "LLM: '{}' succeeded ({} chars)",
                        provider.name(),
                        text.len()
                    );
                    return Ok((text, provider.name().to_string()));
                }
                Err(e) => {
                    tracing::warn!("LLM: '{}' failed: {:?}", provider.name(), e);
                    last_error = e;
                }
            }
        }

        tracing::error!("LLM: All providers failed");
        Err(last_error)
    }
}
