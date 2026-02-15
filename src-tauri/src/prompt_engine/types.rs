// prompt_engine/types.rs — Core types for Prompt Engine

use serde::{Deserialize, Serialize};

/// A template profile for prompt optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub goal: String,
    pub return_format: String,
    pub warnings: Vec<String>,
    pub context_template: String,
}

/// Optimization mode selector
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OptimizationMode {
    AIOptimize,
    ClarityOnly,
}

/// Result of prompt optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedPrompt {
    pub text: String,
    pub profile_used: String,
    pub mode: OptimizationMode,
    pub provider: Option<String>,
    pub confidence: f32,
}

/// Prompt Engine errors
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("Config error: {0}")]
    ConfigError(String),

    #[error("Profile not found: {0}")]
    ProfileNotFound(String),

    #[error("LLM error: {0}")]
    LLMError(String),

    #[error("Template error: {0}")]
    TemplateError(String),
}

/// LLM adapter errors
#[derive(Debug, thiserror::Error)]
pub enum LLMError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("Invalid response from LLM")]
    InvalidResponse,

    #[error("Timeout")]
    Timeout,

    #[error("All LLM providers failed")]
    AllProvidersFailed,
}

/// JSON structure for profiles.json
#[derive(Debug, Deserialize)]
pub struct ProfilesConfig {
    pub profiles: Vec<Profile>,
}
