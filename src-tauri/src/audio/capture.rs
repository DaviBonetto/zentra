use crate::audio::AudioBuffer;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};
use tracing::{error, info};

const RMS_BOOST: f32 = 2.5;

pub struct AudioCapture {
    stream: Option<cpal::Stream>,
    is_recording: bool,
    buffer: Arc<Mutex<AudioBuffer>>,
    level: Arc<AtomicU32>,
    selected_input_device: Option<String>,
}

impl AudioCapture {
    pub fn new() -> Self {
        Self {
            stream: None,
            is_recording: false,
            buffer: Arc::new(Mutex::new(AudioBuffer::new(16000, 1))),
            level: Arc::new(AtomicU32::new(0.0f32.to_bits())),
            selected_input_device: None,
        }
    }

    pub fn start(&mut self) -> Result<(), String> {
        if self.is_recording {
            return Err("Already recording".into());
        }

        let host = cpal::default_host();
        let device = Self::pick_input_device(&host, self.selected_input_device.as_deref())
            .ok_or("No input device available")?;

        let device_name = Self::device_display_name(&device);
        info!("Input device: {}", device_name);

        let config = device.default_input_config().map_err(|e| e.to_string())?;
        if let Ok(mut guard) = self.buffer.lock() {
            guard.sample_rate = config.sample_rate();
            guard.channels = config.channels();
            guard.clear();
        }

        let buffer_clone = self.buffer.clone();
        let level_clone = self.level.clone();
        let err_fn = |err| error!("an error occurred on stream: {}", err);

        let stream = match config.sample_format() {
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &_| write_input_data(data, &buffer_clone, &level_clone),
                err_fn,
                None,
            ),
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &_| write_input_data_f32(data, &buffer_clone, &level_clone),
                err_fn,
                None,
            ),
            _ => return Err("Unsupported sample format".into()),
        }
        .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        self.stream = Some(stream);
        self.is_recording = true;

        Ok(())
    }

    pub fn stop(&mut self) -> Result<AudioBuffer, String> {
        if !self.is_recording {
            return Err("Not recording".into());
        }

        self.stream.take();
        self.is_recording = false;
        self.level.store(0.0f32.to_bits(), Ordering::Relaxed);

        let mut guard = self.buffer.lock().map_err(|e| e.to_string())?;
        let out = guard.clone();
        guard.clear();
        Ok(out)
    }

    pub fn audio_level_handle(&self) -> Arc<AtomicU32> {
        self.level.clone()
    }

    pub fn list_input_devices(&self) -> Result<Vec<String>, String> {
        let host = cpal::default_host();
        let devices = host
            .input_devices()
            .map_err(|e| e.to_string())?
            .map(|device| Self::device_display_name(&device))
            .collect::<Vec<_>>();
        Ok(devices)
    }

    pub fn selected_input_device(&self) -> Option<String> {
        self.selected_input_device.clone()
    }

    pub fn set_selected_input_device(&mut self, name: Option<String>) {
        self.selected_input_device = name
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty());
    }

    pub fn has_selected_device_available(&self) -> bool {
        let Some(selected) = self.selected_input_device.as_ref() else {
            return false;
        };

        let host = cpal::default_host();
        let Ok(mut devices) = host.input_devices() else {
            return false;
        };

        devices.any(|device| Self::device_display_name(&device) == *selected)
    }

    fn pick_input_device(host: &cpal::Host, preferred_name: Option<&str>) -> Option<cpal::Device> {
        if let Some(name) = preferred_name {
            if let Ok(mut devices) = host.input_devices() {
                if let Some(device) = devices.find(|d| Self::device_display_name(d) == name) {
                    return Some(device);
                }
            }
            tracing::warn!(
                "Preferred input device '{}' not found, falling back to default",
                name
            );
        }
        let default_device = host.default_input_device();
        let Some(default_device) = default_device else {
            return None;
        };

        let default_name = Self::device_display_name(&default_device);
        if !Self::looks_like_loopback(&default_name) {
            return Some(default_device);
        }

        tracing::warn!(
            "Default device '{}' looks like loopback, trying to pick a microphone input",
            default_name
        );

        if let Ok(mut devices) = host.input_devices() {
            if let Some(alternative) = devices.find(|d| {
                let name = Self::device_display_name(d);
                !Self::looks_like_loopback(&name)
            }) {
                return Some(alternative);
            }
        }

        Some(default_device)
    }

    fn device_display_name(device: &cpal::Device) -> String {
        device
            .name()
            .or_else(|_| device.description().map(|d| d.name().to_string()))
            .unwrap_or_else(|_| "Unknown input".to_string())
    }

    fn looks_like_loopback(name: &str) -> bool {
        let lower = name.to_ascii_lowercase();
        let patterns = [
            "stereo mix",
            "what u hear",
            "wave out",
            "loopback",
            "monitor",
        ];
        patterns.iter().any(|p| lower.contains(p))
    }
}

fn write_input_data(input: &[i16], buffer: &Arc<Mutex<AudioBuffer>>, level: &Arc<AtomicU32>) {
    if let Ok(mut guard) = buffer.lock() {
        // Simple downmix if stereo (not handled perfectly here, assuming mono for MVP or relying on config)
        // ideally we configure stream to mono, but default config might be stereo.
        // For MVP 16kHz mono requirement: we should resample if needed.
        // This is a simplified passthrough.
        guard.append(input);
    }

    let rms = rms_i16(input);
    let normalized = (rms * RMS_BOOST).clamp(0.0, 1.0);
    level.store(normalized.to_bits(), Ordering::Relaxed);
}

fn write_input_data_f32(input: &[f32], buffer: &Arc<Mutex<AudioBuffer>>, level: &Arc<AtomicU32>) {
    let rms = rms_f32(input);
    let normalized = (rms * RMS_BOOST).clamp(0.0, 1.0);
    level.store(normalized.to_bits(), Ordering::Relaxed);

    // Convert f32 to i16
    let samples: Vec<i16> = input.iter().map(|&x| (x * i16::MAX as f32) as i16).collect();
    if let Ok(mut guard) = buffer.lock() {
        guard.append(&samples);
    }
}

fn rms_i16(input: &[i16]) -> f32 {
    if input.is_empty() {
        return 0.0;
    }
    let sum: f32 = input
        .iter()
        .map(|&s| {
            let v = s as f32 / i16::MAX as f32;
            v * v
        })
        .sum();
    (sum / input.len() as f32).sqrt()
}

fn rms_f32(input: &[f32]) -> f32 {
    if input.is_empty() {
        return 0.0;
    }
    let sum: f32 = input.iter().map(|&s| s * s).sum();
    (sum / input.len() as f32).sqrt()
}

