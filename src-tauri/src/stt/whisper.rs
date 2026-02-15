// src-tauri/src/stt/whisper.rs
// Whisper.cpp Local STT Adapter (Fallback 3)

use super::{STTAdapter, STTError, Transcript};
use crate::audio::AudioBuffer;
use async_trait::async_trait;
use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const TARGET_SAMPLE_RATE: u32 = 16000;

pub struct WhisperAdapter {
    bin_path: PathBuf,
    model_path: PathBuf,
    language: String,
}

impl WhisperAdapter {
    pub fn from_env() -> Option<Self> {
        let bin_path = env::var("WHISPER_CPP_BIN")
            .ok()
            .map(PathBuf::from)
            .or_else(default_whisper_bin);

        let model_path = env::var("WHISPER_MODEL")
            .ok()
            .map(PathBuf::from)
            .or_else(default_whisper_model);

        let language = env::var("WHISPER_LANG").unwrap_or_else(|_| "auto".to_string());

        let bin_path = match bin_path {
            Some(p) if p.exists() => p,
            Some(p) => {
                tracing::warn!("Whisper bin not found at {}", p.display());
                return None;
            }
            None => {
                tracing::warn!("Whisper bin not configured. Set WHISPER_CPP_BIN.");
                return None;
            }
        };

        let model_path = match model_path {
            Some(p) if p.exists() => p,
            Some(p) => {
                tracing::warn!("Whisper model not found at {}", p.display());
                return None;
            }
            None => {
                tracing::warn!("Whisper model not configured. Set WHISPER_MODEL.");
                return None;
            }
        };

        tracing::info!(
            "Whisper adapter initialized: bin={}, model={}",
            bin_path.display(),
            model_path.display()
        );

        Some(Self {
            bin_path,
            model_path,
            language,
        })
    }

    fn to_wav_16k_mono(audio: &AudioBuffer) -> Result<Vec<u8>, STTError> {
        if audio.samples.is_empty() {
            return Err(STTError::InvalidAudio);
        }

        let channels = audio.channels.max(1) as usize;
        let frames = audio.samples.len() / channels;
        if frames == 0 {
            return Err(STTError::InvalidAudio);
        }

        let mut mono = Vec::with_capacity(frames);
        for i in 0..frames {
            let mut sum: i32 = 0;
            for c in 0..channels {
                sum += audio.samples[i * channels + c] as i32;
            }
            let avg = sum as f32 / channels as f32;
            mono.push(avg / i16::MAX as f32);
        }

        let src_rate = audio.sample_rate.max(1) as f32;
        let dst_rate = TARGET_SAMPLE_RATE as f32;

        let out_len = ((mono.len() as f32) * dst_rate / src_rate).ceil() as usize;
        let mut resampled = Vec::with_capacity(out_len.max(1));

        if out_len == 0 {
            return Err(STTError::InvalidAudio);
        }

        let ratio = src_rate / dst_rate;
        for i in 0..out_len {
            let src_pos = i as f32 * ratio;
            let idx = src_pos.floor() as usize;
            let frac = src_pos - idx as f32;
            let s0 = *mono.get(idx).unwrap_or(&0.0);
            let s1 = *mono.get(idx + 1).unwrap_or(&s0);
            let sample = s0 + (s1 - s0) * frac;
            let clamped = sample.clamp(-1.0, 1.0);
            resampled.push((clamped * i16::MAX as f32) as i16);
        }

        encode_wav_i16(&resampled, TARGET_SAMPLE_RATE, 1)
    }

    fn run_whisper(&self, wav_path: &Path, out_base: &Path) -> Result<String, STTError> {
        let output = Command::new(&self.bin_path)
            .arg("--model")
            .arg(&self.model_path)
            .arg("--file")
            .arg(wav_path)
            .arg("--output-txt")
            .arg("--output-file")
            .arg(out_base)
            .arg("--language")
            .arg(&self.language)
            .output()
            .map_err(|e| STTError::ProviderError(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(STTError::ProviderError(format!(
                "Whisper failed: {}",
                stderr.trim()
            )));
        }

        let txt_path = out_base.with_extension("txt");
        if let Ok(text) = fs::read_to_string(&txt_path) {
            return Ok(text);
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        if !stdout.trim().is_empty() {
            return Ok(stdout);
        }

        Err(STTError::ProviderError(
            "Whisper produced no output".to_string(),
        ))
    }
}

#[async_trait]
impl STTAdapter for WhisperAdapter {
    async fn transcribe(&self, audio: &AudioBuffer) -> Result<Transcript, STTError> {
        let wav_bytes = Self::to_wav_16k_mono(audio)?;

        let tmp_dir = env::temp_dir();
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| STTError::ProviderError(e.to_string()))?
            .as_millis();
        let pid = std::process::id();

        let input_path = tmp_dir.join(format!("whisper_input_{}_{}.wav", pid, ts));
        let output_base = tmp_dir.join(format!("whisper_out_{}_{}", pid, ts));

        fs::write(&input_path, wav_bytes)
            .map_err(|e| STTError::ProviderError(e.to_string()))?;

        let result = self.run_whisper(&input_path, &output_base);

        // Cleanup temp files
        let _ = fs::remove_file(&input_path);
        let _ = fs::remove_file(output_base.with_extension("txt"));
        let _ = fs::remove_file(output_base.with_extension("vtt"));
        let _ = fs::remove_file(output_base.with_extension("srt"));

        let text = result?;

        Ok(Transcript {
            text: text.trim().to_string(),
            confidence: 0.85,
            language: Some(self.language.clone()),
            duration_secs: audio.duration_secs,
            provider: "Whisper.cpp".to_string(),
        })
    }

    fn name(&self) -> &str {
        "Whisper.cpp"
    }
}

fn default_whisper_bin() -> Option<PathBuf> {
    let candidates = [
        "bin/whisper-cli.exe",
        "bin/whisper-cli",
        "bin/main.exe",
        "bin/main",
    ];

    for c in candidates {
        let path = PathBuf::from(c);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

fn default_whisper_model() -> Option<PathBuf> {
    let candidates = [
        "models/ggml-base.bin",
        "models/ggml-base.en.bin",
        "models/ggml-small.bin",
        "models/ggml-small.en.bin",
    ];

    for c in candidates {
        let path = PathBuf::from(c);
        if path.exists() {
            return Some(path);
        }
    }

    // Fallback: any ggml-*.bin in models/
    let dir = PathBuf::from("models");
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension() == Some(OsStr::new("bin")) {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("ggml-") {
                        return Some(path);
                    }
                }
            }
        }
    }

    None
}

fn encode_wav_i16(samples: &[i16], sample_rate: u32, channels: u16) -> Result<Vec<u8>, STTError> {
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

    for &sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }

    Ok(wav)
}
