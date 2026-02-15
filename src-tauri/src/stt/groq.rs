// src-tauri/src/stt/groq.rs
// Groq Whisper STT Adapter (Primary)

use super::{STTAdapter, STTError, Transcript};
use crate::audio::AudioBuffer;
use async_trait::async_trait;
use regex::Regex;
use reqwest::multipart;
use std::sync::OnceLock;
use std::time::Duration;

const GROQ_API_URL: &str = "https://api.groq.com/openai/v1/audio/transcriptions";
const MAX_DURATION_SECS: f32 = 59.0;
const TIMEOUT_SECS: u64 = 10;
const DEFAULT_LANGUAGE: &str = "pt";
const RESPONSE_FORMAT: &str = "text";

pub struct GroqAdapter {
    api_key: String,
    client: reqwest::Client,
}

impl GroqAdapter {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .build()
            .expect("Failed to create HTTP client");

        tracing::info!("Groq adapter initialized");

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
        wav.extend_from_slice(&16u32.to_le_bytes()); // chunk size
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM format
        wav.extend_from_slice(&channels.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        let byte_rate = sample_rate * channels as u32 * 2;
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&(channels * 2).to_le_bytes()); // block align
        wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

        // data chunk
        wav.extend_from_slice(b"data");
        let data_size = (samples.len() * 2) as u32;
        wav.extend_from_slice(&data_size.to_le_bytes());

        // PCM samples (i16)
        for &sample in samples {
            wav.extend_from_slice(&sample.to_le_bytes());
        }

        Ok(wav)
    }

    fn effective_duration_secs(audio: &AudioBuffer) -> f32 {
        if audio.duration_secs > 0.0 {
            return audio.duration_secs;
        }
        if audio.sample_rate == 0 {
            return 0.0;
        }
        let channels = audio.channels.max(1) as f32;
        audio.samples.len() as f32 / (audio.sample_rate as f32 * channels)
    }

    fn clean_transcript(text: &str) -> String {
        static TS_RE: OnceLock<Regex> = OnceLock::new();
        let re = TS_RE.get_or_init(|| {
            Regex::new(r"\[\d{2}:\d{2}.*?\]|\(\d{2}:\d{2}\)").expect("valid timestamp regex")
        });
        let stripped = re.replace_all(text, "");
        stripped.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

#[async_trait]
impl STTAdapter for GroqAdapter {
    async fn transcribe(&self, audio: &AudioBuffer) -> Result<Transcript, STTError> {
        let duration_secs = Self::effective_duration_secs(audio);

        // Validate duration (Groq hard limit: 59s)
        if duration_secs > MAX_DURATION_SECS {
            tracing::warn!(
                "Audio too long: {:.1}s > {:.1}s",
                duration_secs,
                MAX_DURATION_SECS
            );
            return Err(STTError::AudioTooLong);
        }

        tracing::info!(
            "Groq STT: transcribing {:.1}s audio...",
            duration_secs
        );

        // Convert to WAV once
        let wav_bytes = Self::to_wav_bytes(audio)?;

        // Create multipart form
        let file_part = multipart::Part::bytes(wav_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| STTError::ProviderError(e.to_string()))?;

        let form = multipart::Form::new()
            .text("model", "whisper-large-v3")
             .text("response_format", RESPONSE_FORMAT)
             .text("language", DEFAULT_LANGUAGE)
            .part("file", file_part);

        let response = self
            .client
            .post(GROQ_API_URL)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = resp.status();

                if status.is_success() {
                    let raw_text = resp
                        .text()
                        .await
                        .map_err(|e| STTError::ProviderError(e.to_string()))?;
                    let cleaned = Self::clean_transcript(&raw_text);

                    if cleaned.is_empty() {
                        return Err(STTError::ProviderError("Empty transcript".to_string()));
                    }

                    Ok(Transcript {
                        text: cleaned,
                        confidence: 0.95, // Groq doesn't return confidence, assume high
                        language: Some(DEFAULT_LANGUAGE.to_string()),
                        duration_secs: duration_secs,
                        provider: "Groq".to_string(),
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
        "Groq Whisper"
    }
}

