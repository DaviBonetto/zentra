use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct SessionProgress {
    pub segment_count: u32,
    pub total_duration_secs: f32,
    pub current_text: String,
}

#[derive(Clone, Serialize)]
pub struct SegmentProgress {
    pub segment_id: String,
    pub sequence_number: u32,
    pub status: SegmentStatus,
    pub provider: Option<String>,
}

#[derive(Clone, Serialize)]
pub enum SegmentStatus {
    Recording,
    Transcribing,
    Completed,
    Failed,
}
