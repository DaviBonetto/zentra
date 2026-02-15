// src-tauri/src/stt/types.rs
// STT Types and Error Definitions

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Transcription result from any STT provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    /// Transcribed text
    pub text: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Detected language (e.g., "pt-BR", "en")
    pub language: Option<String>,
    /// Audio duration in seconds
    pub duration_secs: f32,
    /// Provider name (e.g., "Groq", "VOSK", "ElevenLabs")
    pub provider: String,
}

/// STT Error types with retry classification
#[derive(Debug, Error)]
pub enum STTError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Request timeout")]
    TimeoutError,

    #[error("Audio too long (max 59s for Groq)")]
    AudioTooLong,

    #[error("Invalid audio format")]
    InvalidAudio,

    #[error("Authentication failed")]
    AuthenticationError,

    #[error("Rate limit exceeded")]
    RateLimitError,

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),
}

impl STTError {
    /// Returns true if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            STTError::NetworkError(_) | STTError::TimeoutError | STTError::RateLimitError
        )
    }
}
