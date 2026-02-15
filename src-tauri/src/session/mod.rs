use crate::audio::AudioBuffer;
use crate::orchestrator::FailoverOrchestrator;
use crate::stt::Transcript;
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

        if audio.duration_secs > self.max_segment_duration_secs {
            return Err(SessionError::SegmentTooLong {
                duration: audio.duration_secs,
                max: self.max_segment_duration_secs,
            });
        }

        let sequence_number = self.segments.len() as u32 + 1;
        let mut segment = AudioSegment::new(audio.duration_secs, sequence_number);

        tracing::info!(
            "Processing segment {} ({:.1}s)",
            sequence_number,
            segment.duration_secs
        );

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

                let placeholder_transcript = Transcript {
                    text: INAUDIBLE_PLACEHOLDER.to_string(),
                    confidence: 0.0,
                    language: None,
                    duration_secs: segment.duration_secs,
                    provider: "Placeholder".to_string(),
                };

                segment.set_transcript(placeholder_transcript.clone());
                self.segments.push(segment.clone());

                Ok(SegmentResult {
                    segment_id: segment.id,
                    transcript: placeholder_transcript,
                    is_final: false,
                })
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

const INAUDIBLE_PLACEHOLDER: &str = "[inaudível]";`r`n`r`nfn format_stitch_error(err: StitchError) -> String {
    match err {
        StitchError::SegmentNotTranscribed(id) => format!("Segment not transcribed: {}", id),
    }
}

