pub mod buffer;
pub mod capture;
pub mod vad;

pub use buffer::AudioBuffer;

#[cfg(feature = "onnx")]
use std::path::PathBuf;
#[cfg(feature = "onnx")]
use std::path::Path;
use std::sync::{Arc, atomic::AtomicU32};

use capture::AudioCapture;
#[cfg(feature = "onnx")]
use vad::Vad;

pub struct AudioRecorder {
    capture: AudioCapture,
    #[cfg(feature = "onnx")]
    vad: Option<Vad>,
    is_recording: bool,
    // Store accumulated chunks here? Or inside capture?
    // Start simple: direct passthrough or basic logic
}

impl AudioRecorder {
    pub fn new() -> Result<Self, String> {
        let capture = AudioCapture::new();
        let is_recording = false;

        #[cfg(feature = "onnx")]
        let vad = {
            let model_path = PathBuf::from("resources/silero_vad.onnx");
            if model_path.exists() {
                Some(Vad::new(&model_path)?)
            } else {
                eprintln!("VAD model not found at {:?}. Running without VAD.", model_path);
                None
            }
        };

        Ok(Self {
            capture,
            #[cfg(feature = "onnx")]
            vad,
            is_recording,
        })
    }

    pub fn new_dummy() -> Self {
        Self {
            capture: AudioCapture::new(),
            #[cfg(feature = "onnx")]
            vad: None,
            is_recording: false,
        }
    }

    pub fn start_recording(&mut self) -> Result<(), String> {
        if self.is_recording {
            return Err("Already recording".into());
        }
        self.capture.start()?;
        self.is_recording = true;
        Ok(())
    }

    pub fn stop_recording(&mut self) -> Result<AudioBuffer, String> {
        if !self.is_recording {
            return Err("Not recording".into());
        }
        let buffer = self.capture.stop()?;
        self.is_recording = false;
        Ok(buffer)
    }

    pub fn audio_level_handle(&self) -> Arc<AtomicU32> {
        self.capture.audio_level_handle()
    }
}
