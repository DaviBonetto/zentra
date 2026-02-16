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
const TARGET_SAMPLE_RATE: u32 = 16_000;
const TARGET_CHANNELS: u16 = 1;
const TRANSCRIPTION_PROMPT: &str =
    "Transcreva exatamente a fala em português brasileiro. Não invente texto quando houver silêncio.";

pub struct GroqAdapter {
    api_key: String,
    client: reqwest::Client,
    model: String,
    language: Option<String>,
}

impl GroqAdapter {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .build()
            .expect("Failed to create HTTP client");

        let model = std::env::var("GROQ_STT_MODEL")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "whisper-large-v3".to_string());

        let language = std::env::var("GROQ_STT_LANGUAGE")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .and_then(|value| {
                if value.eq_ignore_ascii_case("auto") {
                    None
                } else {
                    Some(value)
                }
            })
            .or_else(|| Some(DEFAULT_LANGUAGE.to_string()));

        tracing::info!(
            "Groq adapter initialized (model={}, language={})",
            model,
            language.clone().unwrap_or_else(|| "auto".to_string())
        );

        Self {
            api_key,
            client,
            model,
            language,
        }
    }

    /// Convert AudioBuffer to WAV bytes
    fn to_wav_bytes(audio: &AudioBuffer) -> Result<Vec<u8>, STTError> {
        let sample_rate = audio.sample_rate.max(1);
        let channels = audio.channels.max(1);
        let samples = &audio.samples;

        if samples.is_empty() {
            return Err(STTError::InvalidAudio);
        }

        // Downmix to mono and resample to 16kHz before uploading.
        // This matches Groq recommendations and avoids device-specific channel/layout artifacts.
        let mono = Self::downmix_to_mono(samples, channels);
        let normalized = Self::resample_linear(&mono, sample_rate, TARGET_SAMPLE_RATE);

        let mut wav = Vec::new();

        // RIFF header
        wav.extend_from_slice(b"RIFF");
        let file_size = (36 + normalized.len() * 2) as u32;
        wav.extend_from_slice(&file_size.to_le_bytes());
        wav.extend_from_slice(b"WAVE");

        // fmt chunk
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes()); // chunk size
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM format
        wav.extend_from_slice(&TARGET_CHANNELS.to_le_bytes());
        wav.extend_from_slice(&TARGET_SAMPLE_RATE.to_le_bytes());
        let byte_rate = TARGET_SAMPLE_RATE * TARGET_CHANNELS as u32 * 2;
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&(TARGET_CHANNELS * 2).to_le_bytes()); // block align
        wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

        // data chunk
        wav.extend_from_slice(b"data");
        let data_size = (normalized.len() * 2) as u32;
        wav.extend_from_slice(&data_size.to_le_bytes());

        // PCM samples (i16)
        for &sample in &normalized {
            wav.extend_from_slice(&sample.to_le_bytes());
        }

        Ok(wav)
    }

    fn downmix_to_mono(samples: &[i16], channels: u16) -> Vec<f32> {
        if channels <= 1 {
            return samples.iter().map(|sample| *sample as f32).collect();
        }

        let ch = channels as usize;
        let frame_count = samples.len() / ch;
        let mut mono = Vec::with_capacity(frame_count);

        for frame_idx in 0..frame_count {
            let base = frame_idx * ch;
            let mut sum = 0.0f32;
            for channel_idx in 0..ch {
                sum += samples[base + channel_idx] as f32;
            }
            mono.push(sum / channels as f32);
        }

        mono
    }

    fn resample_linear(input: &[f32], source_rate: u32, target_rate: u32) -> Vec<i16> {
        if input.is_empty() {
            return Vec::new();
        }

        if source_rate == target_rate {
            return input
                .iter()
                .map(|sample| sample.clamp(i16::MIN as f32, i16::MAX as f32) as i16)
                .collect();
        }

        let ratio = target_rate as f64 / source_rate as f64;
        let out_len = ((input.len() as f64) * ratio).round().max(1.0) as usize;
        let mut output = Vec::with_capacity(out_len);

        for out_idx in 0..out_len {
            let src_pos = out_idx as f64 * (source_rate as f64 / target_rate as f64);
            let left_idx = src_pos.floor() as usize;
            let right_idx = usize::min(left_idx + 1, input.len() - 1);
            let frac = (src_pos - left_idx as f64) as f32;
            let interpolated = input[left_idx] * (1.0 - frac) + input[right_idx] * frac;
            output.push(interpolated.clamp(i16::MIN as f32, i16::MAX as f32) as i16);
        }

        output
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
            "Groq STT: transcribing {:.1}s audio with model {}",
            duration_secs,
            self.model
        );

        // Convert to WAV once
        let wav_bytes = Self::to_wav_bytes(audio)?;

        // Create multipart form
        let file_part = multipart::Part::bytes(wav_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| STTError::ProviderError(e.to_string()))?;

        let form = multipart::Form::new()
            .text("model", self.model.clone())
            .text("response_format", RESPONSE_FORMAT)
            .text("temperature", "0")
            .text("prompt", TRANSCRIPTION_PROMPT)
            .part("file", file_part);

        let form = if let Some(language) = self.language.as_deref() {
            form.text("language", language.to_string())
        } else {
            form
        };

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
                        language: self.language.clone(),
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

