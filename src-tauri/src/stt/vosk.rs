// src-tauri/src/stt/vosk.rs
// VOSK Local STT Adapter (Fallback 1)

use super::{STTAdapter, STTError, Transcript};
use crate::audio::AudioBuffer;
use async_trait::async_trait;
use std::{path::Path, sync::Arc};
use vosk::{Model, Recognizer};

#[cfg(feature = "vosk-stt")]
pub struct VoskAdapter {
    model_pt: Model,
    model_en: Option<Model>,
}

#[cfg(feature = "vosk-stt")]
impl VoskAdapter {
    pub fn new(model_pt_path: &str, model_en_path: &str) -> Result<Self, STTError> {
        let model_pt = Model::new(model_pt_path).ok_or_else(|| {
            STTError::ModelNotFound(format!("PT model at '{}' not found", model_pt_path))
        })?;

        let model_en = if Path::new(model_en_path).exists() {
            let model = Model::new(model_en_path);
            if model.is_none() {
                tracing::warn!(
                    "VOSK EN model found at {}, but failed to load. EN fallback disabled.",
                    model_en_path
                );
            }
            model.map(Arc::new)
        } else {
            tracing::warn!(
                "VOSK EN model not found at {}. EN fallback disabled.",
                model_en_path
            );
            None
        };

        tracing::info!(
            "VOSK adapter initialized. PT model at {}, EN model {}",
            model_pt_path,
            if model_en.is_some() { "loaded" } else { "missing" }
        );

        Ok(Self {
            model_pt: Arc::new(model_pt),
            model_en,
        })
    }

    async fn transcribe_with_model(
        &self,
        model: &Model,
        audio: &AudioBuffer,
        language: &str,
    ) -> Result<Transcript, STTError> {
        // VOSK expects PCM 16kHz mono i16
        // Model must be created outside, recognizer created per request
        if audio.samples.is_empty() {
            return Err(STTError::InvalidAudio);
        }

        if audio.sample_rate != 16000 {
            // For simplify, we assume 16kHz. If not, we should resample.
            // Current project setup captures at 16kHz.
            tracing::warn!(
                "VOSK expects 16kHz audio, got {}Hz. Results may be poor.",
                audio.sample_rate
            );
        }

        let mut recognizer = Recognizer::new(model, audio.sample_rate as f32)
            .ok_or_else(|| STTError::ProviderError("Failed to create VOSK recognizer".to_string()))?;

        // VOSK crate accepts i16 samples directly
        recognizer
            .accept_waveform(&audio.samples)
            .map_err(|e| STTError::ProviderError(e.to_string()))?;

        let final_result = recognizer.final_result();
        let result_single = final_result
            .single()
            .ok_or_else(|| STTError::ProviderError("No result from VOSK".to_string()))?;
        let text = result_single.text.to_string();

        // Confidence estimation: VOSK doesn't give a simple confidence in simple result mode readily without parsing JSON result detail
        // For fallback, we assume modest confidence if text is present.
        let confidence = if text.trim().is_empty() { 0.0 } else { 0.7 };

        Ok(Transcript {
            text,
            confidence,
            language: Some(language.to_string()),
            duration_secs: audio.duration_secs,
            provider: "VOSK".to_string(),
        })
    }
}

#[async_trait]
impl STTAdapter for VoskAdapter {
    async fn transcribe(&self, audio: &AudioBuffer) -> Result<Transcript, STTError> {
        // Try PT-BR model first (assuming primary usage is PT)
        // Ideally we run both or detect, but for fallback sequence:
        // Run PT. If confidence low or empty, check EN?
        // Since VOSK is CPU bound, running twice adds latency.
        // Simple strategy: Run PT model, then EN if available and PT empty.

        tracing::info!("VOSK STT attempt (PT-BR)...");
        let pt = self
            .transcribe_with_model(&self.model_pt, audio, "pt-BR")
            .await?;

        if !pt.text.trim().is_empty() {
            return Ok(pt);
        }

        if let Some(ref model_en) = self.model_en {
            tracing::info!("VOSK STT fallback (EN-US)...");
            let en = self.transcribe_with_model(model_en, audio, "en-US").await?;
            if !en.text.trim().is_empty() {
                return Ok(en);
            }
        }

        Err(STTError::ProviderError(
            "Empty transcription from VOSK".to_string(),
        ))
    }

    fn name(&self) -> &str {
        "VOSK Local"
    }
}
