use crate::audio::AudioBuffer;
use crate::orchestrator::{FailoverOrchestrator, OrchestratorError};
use crate::stt::{STTError, Transcript};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use uuid::Uuid;

pub mod progress;
pub mod segment;
pub mod stitcher;

pub use progress::SessionProgress;
pub use segment::AudioSegment;
pub use stitcher::{StitchError, Stitcher};

pub struct SessionStitcher {
    max_segment_duration_secs: f32,
    segments: Vec<AudioSegment>,
    orchestrator: Arc<TokioMutex<FailoverOrchestrator>>,
    current_session_id: Option<String>,
    max_segments: usize,
}

#[derive(Clone, Serialize)]
pub struct StitchedResult {
    pub full_text: String,
    pub total_duration_secs: f32,
    pub segment_count: u32,
    pub confidence_avg: f32,
    pub providers_used: Vec<String>,
}

#[derive(Clone, Serialize)]
pub struct SegmentResult {
    pub segment_id: String,
    pub transcript: Transcript,
    pub is_final: bool,
}

#[derive(Debug)]
pub enum SessionError {
    NoActiveSession,
    EmptySession,
    SegmentTooLong { duration: f32, max: f32 },
    SegmentLimitReached { max: usize },
    StitchError(String),
    TranscriptionFailed(String),
}

impl SessionStitcher {
    pub fn new(orchestrator: Arc<TokioMutex<FailoverOrchestrator>>) -> Self {
        Self {
            max_segment_duration_secs: 59.0,
            segments: Vec::new(),
            orchestrator,
            current_session_id: None,
            max_segments: 100,
        }
    }

    pub async fn start_session(&mut self) -> Result<String, SessionError> {
        let session_id = Uuid::new_v4().to_string();
        self.current_session_id = Some(session_id.clone());
        self.segments.clear();

        tracing::info!("Started new session: {}", session_id);
        Ok(session_id)
    }

    pub async fn add_segment(&mut self, audio: AudioBuffer) -> Result<SegmentResult, SessionError> {
        if self.current_session_id.is_none() {
            return Err(SessionError::NoActiveSession);
        }

        if self.segments.len() >= self.max_segments {
            return Err(SessionError::SegmentLimitReached {
                max: self.max_segments,
            });
        }

        let effective_duration_secs = derive_duration_secs(&audio);

        if effective_duration_secs > self.max_segment_duration_secs {
            return Err(SessionError::SegmentTooLong {
                duration: effective_duration_secs,
                max: self.max_segment_duration_secs,
            });
        }

        let sequence_number = self.segments.len() as u32 + 1;
        let mut segment = AudioSegment::new(effective_duration_secs, sequence_number);

        tracing::info!(
            "Processing segment {} ({:.1}s)",
            sequence_number,
            segment.duration_secs
        );

        let metrics = audio_energy_metrics(&audio);
        tracing::info!(
            "Segment {} energy: rms={:.5}, peak={:.5}, speech_ratio={:.3}",
            sequence_number,
            metrics.rms,
            metrics.peak,
            metrics.speech_ratio
        );

        let silence_gate_active = silence_gate_enabled();
        if !silence_gate_active {
            tracing::debug!("Silence gate disabled; sending segment {} to Groq", sequence_number);
        }

        if silence_gate_active && is_probable_silence(metrics) {
            tracing::warn!(
                "Segment {} skipped: probable silence (rms={:.5}, peak={:.5}, speech_ratio={:.3})",
                sequence_number,
                metrics.rms,
                metrics.peak,
                metrics.speech_ratio
            );

            let silent_transcript = Transcript {
                text: String::new(),
                confidence: 0.0,
                language: None,
                duration_secs: effective_duration_secs,
                provider: "SilenceGate".to_string(),
            };

            segment.set_transcript(silent_transcript.clone());
            self.segments.push(segment.clone());

            return Ok(SegmentResult {
                segment_id: segment.id,
                transcript: silent_transcript,
                is_final: false,
            });
        }

        let transcript_result = {
            let mut orchestrator = self.orchestrator.lock().await;
            orchestrator.transcribe(&audio).await
        };

        match transcript_result {
            Ok(transcript) => {
                tracing::info!(
                    "Segment {} transcribed: provider={}, confidence={:.2}, text_len={}",
                    sequence_number,
                    transcript.provider,
                    transcript.confidence,
                    transcript.text.len()
                );
                segment.set_transcript(transcript.clone());
                self.segments.push(segment.clone());

                Ok(SegmentResult {
                    segment_id: segment.id,
                    transcript,
                    is_final: false,
                })
            }
            Err(e) => {
                tracing::error!("Segment {} failed: {:?}", sequence_number, e);
                Err(SessionError::TranscriptionFailed(map_orchestrator_error(&e)))
            }
        }
    }

    pub async fn finalize_session(&mut self) -> Result<StitchedResult, SessionError> {
        if self.current_session_id.is_none() {
            return Err(SessionError::NoActiveSession);
        }

        if self.segments.is_empty() {
            return Err(SessionError::EmptySession);
        }

        tracing::info!("Finalizing session: {} segments", self.segments.len());

        let full_text = Stitcher::stitch_transcripts(&self.segments)
            .map_err(|e| SessionError::StitchError(format_stitch_error(e)))?;

        let total_duration_secs: f32 = self.segments.iter().map(|s| s.duration_secs).sum();

        let mut confidence_sum = 0.0f32;
        let mut confidence_count = 0u32;
        let mut providers_used: Vec<String> = Vec::new();

        for segment in &self.segments {
            if let Some(transcript) = segment.transcript.as_ref() {
                confidence_sum += transcript.confidence;
                confidence_count += 1;

                if !providers_used.contains(&transcript.provider) {
                    providers_used.push(transcript.provider.clone());
                }
            }
        }

        let confidence_avg = if confidence_count == 0 {
            0.0
        } else {
            confidence_sum / confidence_count as f32
        };

        let result = StitchedResult {
            full_text,
            total_duration_secs,
            segment_count: self.segments.len() as u32,
            confidence_avg,
            providers_used,
        };

        self.current_session_id = None;
        self.segments.clear();

        tracing::info!(
            "Session finalized: {} chars, {:.1}s total",
            result.full_text.len(),
            result.total_duration_secs
        );

        Ok(result)
    }

    pub fn get_progress(&self) -> SessionProgress {
        let total_duration_secs: f32 = self.segments.iter().map(|s| s.duration_secs).sum();

        let current_text = if self.segments.is_empty() {
            String::new()
        } else {
            Stitcher::stitch_transcripts(&self.segments).unwrap_or_default()
        };

        SessionProgress {
            segment_count: self.segments.len() as u32,
            total_duration_secs,
            current_text,
        }
    }
}


fn derive_duration_secs(audio: &AudioBuffer) -> f32 {
    if audio.duration_secs > 0.05 {
        return audio.duration_secs;
    }

    let sample_rate = audio.sample_rate as f32;
    let channels = audio.channels.max(1) as f32;
    if sample_rate <= 0.0 {
        return 0.0;
    }

    audio.samples.len() as f32 / (sample_rate * channels)
}

fn format_stitch_error(err: StitchError) -> String {
    match err {
        StitchError::SegmentNotTranscribed(id) => format!("Segment not transcribed: {}", id),
    }
}


fn map_orchestrator_error(err: &OrchestratorError) -> String {
    match err {
        OrchestratorError::NoProvidersAvailable => {
            "Groq API key missing or invalid. Configure a valid key in Setup/Settings.".to_string()
        }
        OrchestratorError::AllProvidersFailed(errors) => {
            if errors.iter().any(|(_, e)| matches!(e, STTError::AuthenticationError)) {
                return "Groq authentication failed. Check if your API key is valid.".to_string();
            }
            if errors.iter().any(|(_, e)| matches!(e, STTError::RateLimitError)) {
                return "Groq rate limit reached. Please wait and try again.".to_string();
            }
            if errors.iter().any(|(_, e)| matches!(e, STTError::TimeoutError)) {
                return "Groq request timed out. Check your connection and try again.".to_string();
            }

            let details = errors
                .iter()
                .map(|(provider, error)| format!("{}: {}", provider, error))
                .collect::<Vec<_>>()
                .join(" | ");
            format!("Groq transcription failed. {}", details)
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct AudioEnergyMetrics {
    rms: f32,
    peak: f32,
    speech_ratio: f32,
}

fn audio_energy_metrics(audio: &AudioBuffer) -> AudioEnergyMetrics {
    if audio.samples.is_empty() {
        return AudioEnergyMetrics {
            rms: 0.0,
            peak: 0.0,
            speech_ratio: 0.0,
        };
    }

    let mut sum_squares = 0.0f32;
    let mut peak = 0.0f32;

    for &sample in &audio.samples {
        let normalized = sample as f32 / i16::MAX as f32;
        let abs = normalized.abs();
        sum_squares += normalized * normalized;
        if abs > peak {
            peak = abs;
        }
    }

    let rms = (sum_squares / audio.samples.len() as f32).sqrt();

    let channels = audio.channels.max(1) as usize;
    let frame_size = (audio.sample_rate as usize / 50).max(160) * channels; // ~20ms frames
    let mut total_frames = 0usize;
    let mut speech_frames = 0usize;

    let mut idx = 0usize;
    while idx < audio.samples.len() {
        let end = usize::min(idx + frame_size, audio.samples.len());
        let frame = &audio.samples[idx..end];
        if !frame.is_empty() {
            total_frames += 1;
            let frame_rms = (frame
                .iter()
                .map(|sample| {
                    let normalized = *sample as f32 / i16::MAX as f32;
                    normalized * normalized
                })
                .sum::<f32>()
                / frame.len() as f32)
                .sqrt();
            if frame_rms >= 0.003 {
                speech_frames += 1;
            }
        }
        idx = end;
    }

    let speech_ratio = if total_frames == 0 {
        0.0
    } else {
        speech_frames as f32 / total_frames as f32
    };

    AudioEnergyMetrics {
        rms,
        peak,
        speech_ratio,
    }
}

fn is_probable_silence(metrics: AudioEnergyMetrics) -> bool {
    metrics.rms < 0.0015 && metrics.peak < 0.010 && metrics.speech_ratio < 0.015
}

fn silence_gate_enabled() -> bool {
    std::env::var("ZENTRA_ENABLE_SILENCE_GATE")
        .map(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}
