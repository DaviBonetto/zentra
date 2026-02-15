// src-tauri/src/stt/mod.rs
// STT Module - Speech-to-Text Adapters

mod types;
mod groq;
mod elevenlabs;
#[cfg(feature = "vosk-stt")]
mod vosk;
mod whisper;

pub use types::{Transcript, STTError};
pub use groq::GroqAdapter;
pub use elevenlabs::ElevenLabsAdapter;
#[cfg(feature = "vosk-stt")]
pub use vosk::VoskAdapter;
pub use whisper::WhisperAdapter;

use crate::audio::AudioBuffer;
use async_trait::async_trait;

/// Unified STT Adapter trait
#[async_trait]
pub trait STTAdapter: Send + Sync {
    /// Transcribe audio buffer to text
    async fn transcribe(&self, audio: &AudioBuffer) -> Result<Transcript, STTError>;

    /// Get provider name
    fn name(&self) -> &str;
}

/// STT Manager with failover support
pub struct STTManager {
    groq: Option<GroqAdapter>,
    #[cfg(feature = "vosk-stt")]
    vosk: Option<VoskAdapter>,
    elevenlabs: Option<ElevenLabsAdapter>,
    whisper: Option<WhisperAdapter>,
}

impl STTManager {
    /// Create new STT Manager from environment variables
    pub fn new() -> Self {
        let groq_key = std::env::var("GROQ_API_KEY").ok();
        let eleven_key = std::env::var("ELEVENLABS_API_KEY").ok();

        let groq = groq_key
            .filter(|k| k.starts_with("gsk_"))
            .map(|k| GroqAdapter::new(k));

        let elevenlabs = eleven_key
            .filter(|k| k.starts_with("sk_"))
            .map(|k| ElevenLabsAdapter::new(k));

        #[cfg(feature = "vosk-stt")]
        let vosk = {
            let model_pt = std::env::var("VOSK_MODEL_PT")
                .unwrap_or_else(|_| "models/vosk-model-small-pt-0.3".to_string());
            let model_en = std::env::var("VOSK_MODEL_EN")
                .unwrap_or_else(|_| "models/vosk-model-small-en-us-0.15".to_string());

            match VoskAdapter::new(&model_pt, &model_en) {
                Ok(v) => Some(v),
                Err(e) => {
                    tracing::warn!("VOSK init skipped: {}", e);
                    None
                }
            }
        };

        #[cfg(feature = "vosk-stt")]
        let vosk_available = vosk.is_some();
        #[cfg(not(feature = "vosk-stt"))]
        let vosk_available = false;

        let whisper = WhisperAdapter::from_env();

        tracing::info!(
            "STT Manager initialized: Groq={}, VOSK={}, ElevenLabs={}, Whisper={}",
            groq.is_some(),
            vosk_available,
            elevenlabs.is_some(),
            whisper.is_some()
        );

        Self {
            groq,
            #[cfg(feature = "vosk-stt")]
            vosk,
            elevenlabs,
            whisper,
        }
    }

    /// Transcribe with automatic failover: Groq -> VOSK -> ElevenLabs -> Whisper
    pub async fn transcribe(&self, audio: &AudioBuffer) -> Result<Transcript, STTError> {
        // 1. Try Groq (Primary)
        if let Some(ref adapter) = self.groq {
            match adapter.transcribe(audio).await {
                Ok(transcript) => {
                    tracing::info!("Groq STT success: {} chars", transcript.text.len());
                    return Ok(transcript);
                }
                Err(e) => {
                    tracing::warn!("Groq STT failed: {:?}, trying fallback...", e);
                }
            }
        }

        // 2. VOSK (Local fallback)
        #[cfg(feature = "vosk-stt")]
        if let Some(ref adapter) = self.vosk {
            match adapter.transcribe(audio).await {
                Ok(transcript) => {
                    tracing::info!("VOSK STT success: {} chars", transcript.text.len());
                    return Ok(transcript);
                }
                Err(e) => {
                    tracing::warn!("VOSK STT failed: {:?}, trying fallback ElevenLabs...", e);
                }
            }
        }

        // 3. Try ElevenLabs (Cloud Fallback)
        if let Some(ref adapter) = self.elevenlabs {
            match adapter.transcribe(audio).await {
                Ok(transcript) => {
                    tracing::info!("ElevenLabs STT success: {} chars", transcript.text.len());
                    return Ok(transcript);
                }
                Err(e) => {
                    tracing::warn!("ElevenLabs STT failed: {:?}, trying fallback Whisper...", e);
                }
            }
        }

        // 4. Try Whisper.cpp (Local fallback)
        if let Some(ref adapter) = self.whisper {
            match adapter.transcribe(audio).await {
                Ok(transcript) => {
                    tracing::info!("Whisper STT success: {} chars", transcript.text.len());
                    return Ok(transcript);
                }
                Err(e) => {
                    tracing::error!("Whisper STT failed: {:?}", e);
                    return Err(e);
                }
            }
        }

        Err(STTError::ProviderError(
            "No STT providers available or all failed.".to_string(),
        ))
    }
}

impl Default for STTManager {
    fn default() -> Self {
        Self::new()
    }
}
