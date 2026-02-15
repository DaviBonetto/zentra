// src-tauri/src/stt/elevenlabs.rs
// ElevenLabs Scribe STT Adapter (Fallback)

use super::{STTAdapter, STTError, Transcript};
use crate::audio::AudioBuffer;
use async_trait::async_trait;
use reqwest::multipart;
use serde::Deserialize;
use std::time::Duration;

const ELEVENLABS_API_URL: &str = "https://api.elevenlabs.io/v1/speech-to-text";
const TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Deserialize)]
struct ElevenLabsResponse {
    text: String,
    #[serde(default)]
    language_code: Option<String>,
}

pub struct ElevenLabsAdapter {
    api_key: String,
    client: reqwest::Client,
}

impl ElevenLabsAdapter {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .build()
            .expect("Failed to create HTTP client");

        tracing::info!("ElevenLabs adapter initialized");

        Self { api_key, client }
    }

    /// Convert AudioBuffer to WAV bytes
    fn to_wav_bytes(audio: &AudioBuffer) -> Result<Vec<u8>, STTError> {
        let sample_rate = audio.sample_rate;
        let channels = audio.channels;
        let samples = &audio.samples;

        if samples.is_empty() {
            return Err(STTError::InvalidAudio);
        }

        let mut wav = Vec::new();

        // RIFF header
        wav.extend_from_slice(b"RIFF");
        let file_size = (36 + samples.len() * 2) as u32;
        wav.extend_from_slice(&file_size.to_le_bytes());
        wav.extend_from_slice(b"WAVE");

        // fmt chunk
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&channels.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        let byte_rate = sample_rate * channels as u32 * 2;
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&(channels * 2).to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());

        // data chunk
        wav.extend_from_slice(b"data");
        let data_size = (samples.len() * 2) as u32;
        wav.extend_from_slice(&data_size.to_le_bytes());

        for &sample in samples {
            wav.extend_from_slice(&sample.to_le_bytes());
        }

        Ok(wav)
    }
}

#[async_trait]
impl STTAdapter for ElevenLabsAdapter {
    async fn transcribe(&self, audio: &AudioBuffer) -> Result<Transcript, STTError> {
        tracing::info!(
            "ElevenLabs STT: transcribing {:.1}s audio...",
            audio.duration_secs
        );

        let wav_bytes = Self::to_wav_bytes(audio)?;

        // Create form
        let file_part = multipart::Part::bytes(wav_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| STTError::ProviderError(e.to_string()))?;

        let form = multipart::Form::new()
            .text("model_id", "scribe_v1")
            .part("audio", file_part);

        let response = self
            .client
            .post(ELEVENLABS_API_URL)
            .header("xi-api-key", &self.api_key)
            .multipart(form)
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = resp.status();

                if status.is_success() {
                    let eleven_resp: ElevenLabsResponse = resp
                        .json()
                        .await
                        .map_err(|e| STTError::ProviderError(e.to_string()))?;

                    Ok(Transcript {
                        text: eleven_resp.text,
                        confidence: 0.90,
                        language: eleven_resp.language_code,
                        duration_secs: audio.duration_secs,
                        provider: "ElevenLabs".to_string(),
                    })
                } else if status.as_u16() == 401 {
                    Err(STTError::AuthenticationError)
                } else if status.as_u16() == 429 {
                    Err(STTError::RateLimitError)
                } else {
                    let error_text = resp.text().await.unwrap_or_default();
                    Err(STTError::ProviderError(format!(
                        "HTTP {}: {}",
                        status, error_text
                    )))
                }
            }
            Err(e) => {
                if e.is_timeout() {
                    Err(STTError::TimeoutError)
                } else {
                    Err(STTError::NetworkError(e.to_string()))
                }
            }
        }
    }

    fn name(&self) -> &str {
        "ElevenLabs Scribe"
    }
}
