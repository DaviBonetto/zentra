#[cfg(feature = "onnx")]
use std::path::Path;
#[cfg(feature = "onnx")]
use tracing::info;

#[cfg(feature = "onnx")]
pub struct Vad {
    // State vectors for Silero VAD (RNN hidden states)
    h: Vec<f32>,
    c: Vec<f32>,
    threshold: f32,
    #[allow(dead_code)]
    sr: Vec<i64>,
}

#[cfg(feature = "onnx")]
impl Vad {
    pub fn new(model_path: &Path) -> Result<Self, String> {
        // Verify model file exists
        if !model_path.exists() {
            return Err(format!("VAD model not found at {:?}", model_path));
        }
        info!("VAD model found at {:?}", model_path);

        Ok(Self {
            h: vec![0.0f32; 2 * 1 * 64],
            c: vec![0.0f32; 2 * 1 * 64],
            sr: vec![16000],
            threshold: 0.05,
        })
    }

    pub fn reset(&mut self) {
        self.h = vec![0.0f32; 2 * 1 * 64];
        self.c = vec![0.0f32; 2 * 1 * 64];
    }

    /// Detect speech using RMS energy threshold
    pub fn is_speech(&mut self, samples: &[f32]) -> Result<bool, String> {
        if samples.is_empty() {
            return Ok(false);
        }
        let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
        let rms = (sum_sq / samples.len() as f32).sqrt();
        Ok(rms > self.threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_vad_silence() {
        let mut vad = Vad {
            h: vec![],
            c: vec![],
            sr: vec![],
            threshold: 0.05,
        };

        let silence = vec![0.0f32; 512];
        let result = vad.is_speech(&silence).unwrap();
        assert_eq!(result, false, "Silence should not be detected as speech");
    }

    #[test]
    fn test_vad_loud_noise() {
        let mut vad = Vad {
            h: vec![],
            c: vec![],
            sr: vec![],
            threshold: 0.05,
        };

        let noise = vec![0.5f32; 512]; // RMS = 0.5 > 0.05
        let result = vad.is_speech(&noise).unwrap();
        assert_eq!(result, true, "Loud noise should be detected as speech");
    }
}
