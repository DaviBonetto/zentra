use crate::stt::Transcript;
use std::time::Instant;
use uuid::Uuid;

#[derive(Clone)]
pub struct AudioSegment {
    pub id: String,
    pub transcript: Option<Transcript>,
    pub sequence_number: u32,
    pub timestamp: Instant,
    pub duration_secs: f32,
}

impl AudioSegment {
    pub fn new(duration_secs: f32, sequence_number: u32) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            duration_secs,
            transcript: None,
            sequence_number,
            timestamp: Instant::now(),
        }
    }

    pub fn set_transcript(&mut self, transcript: Transcript) {
        self.transcript = Some(transcript);
    }

    pub fn is_transcribed(&self) -> bool {
        self.transcript.is_some()
    }
}
