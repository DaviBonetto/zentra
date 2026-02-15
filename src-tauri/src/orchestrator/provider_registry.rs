use super::ProviderConfig;
use crate::stt::{ElevenLabsAdapter, GroqAdapter, WhisperAdapter};
#[cfg(feature = "vosk-stt")]
use crate::stt::VoskAdapter;
use std::env;

pub fn default_providers_from_env() -> Vec<ProviderConfig> {
    let mut providers = Vec::new();

    if let Some(key) = env::var("GROQ_API_KEY").ok().filter(|k| k.starts_with("gsk_")) {
        providers.push(ProviderConfig {
            id: "groq".to_string(),
            priority: 1,
            adapter: Box::new(GroqAdapter::new(key)),
            max_retries: 0,
            timeout_secs: 10,
            confidence_threshold: 0.7,
        });
    }

    #[cfg(feature = "vosk-stt")]
    {
        let model_pt = env::var("VOSK_MODEL_PT")
            .unwrap_or_else(|_| "models/vosk-model-small-pt-0.3".to_string());
        let model_en = env::var("VOSK_MODEL_EN")
            .unwrap_or_else(|_| "models/vosk-model-small-en-us-0.15".to_string());

        if let Ok(vosk) = VoskAdapter::new(&model_pt, &model_en) {
            providers.push(ProviderConfig {
                id: "vosk".to_string(),
                priority: 2,
                adapter: Box::new(vosk),
                max_retries: 0,
                timeout_secs: 15,
                confidence_threshold: 0.5,
            });
        }
    }

    if let Some(key) = env::var("ELEVENLABS_API_KEY").ok().filter(|k| k.starts_with("sk_")) {
        providers.push(ProviderConfig {
            id: "elevenlabs".to_string(),
            priority: 3,
            adapter: Box::new(ElevenLabsAdapter::new(key)),
            max_retries: 1,
            timeout_secs: 10,
            confidence_threshold: 0.6,
        });
    }

    if let Some(whisper) = WhisperAdapter::from_env() {
        providers.push(ProviderConfig {
            id: "whisper".to_string(),
            priority: 4,
            adapter: Box::new(whisper),
            max_retries: 0,
            timeout_secs: 20,
            confidence_threshold: 0.5,
        });
    }

    providers
}
