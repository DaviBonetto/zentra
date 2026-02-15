use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioBuffer {
    pub samples: Vec<i16>,
    pub sample_rate: u32,
    pub channels: u16,
    /// Cached duration in seconds
    #[serde(skip)]
    pub duration_secs: f32,
}

impl AudioBuffer {
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            samples: Vec::new(),
            sample_rate,
            channels,
            duration_secs: 0.0,
        }
    }

    /// Recalculate and update duration_secs
    pub fn update_duration(&mut self) {
        if self.sample_rate == 0 {
            self.duration_secs = 0.0;
        } else {
            let channels = self.channels.max(1) as f32;
            self.duration_secs = self.samples.len() as f32 / (self.sample_rate as f32 * channels);
        }
    }

    pub fn clear(&mut self) {
        self.samples.clear();
        self.duration_secs = 0.0;
    }

    pub fn append(&mut self, data: &[i16]) {
        self.samples.extend_from_slice(data);
        self.update_duration();
    }
}
