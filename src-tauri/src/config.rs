use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};

const CONFIG_DIR: &str = "zentra";
const CONFIG_FILE: &str = "config.json";
const HISTORY_LIMIT: usize = 50;
const API_KEY_XOR_KEY: &[u8] = b"zentra-local-key-v1";

pub const DEFAULT_HOTKEY: &str = "CommandOrControl+Shift+Space";
pub const DEFAULT_LANGUAGE: &str = "pt";
pub const DEFAULT_USE_CASE: &str = "general";
pub const GITHUB_URL: &str = "https://github.com/DaviBonetto/zentra";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub setup_completed: bool,
    pub user_name: String,
    pub use_case: String,
    pub groq_api_key_obfuscated: Option<String>,
    pub input_device_name: Option<String>,
    pub hotkey: String,
    pub language: String,
    pub stats: Stats,
    pub history: Vec<HistoryItem>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            setup_completed: false,
            user_name: String::new(),
            use_case: DEFAULT_USE_CASE.to_string(),
            groq_api_key_obfuscated: None,
            input_device_name: None,
            hotkey: DEFAULT_HOTKEY.to_string(),
            language: DEFAULT_LANGUAGE.to_string(),
            stats: Stats::default(),
            history: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Stats {
    pub total_transcriptions: u64,
    pub total_words: u64,
    pub total_recording_seconds: f32,
    pub total_seconds_saved: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub text: String,
    pub timestamp: String,
    #[serde(alias = "duration_seconds")]
    pub duration_seconds: f32,
    #[serde(alias = "word_count")]
    pub word_count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupState {
    pub setup_completed: bool,
    pub user_name: String,
    pub use_case: String,
    pub has_api_key: bool,
    pub input_device_name: Option<String>,
    pub hotkey: String,
    pub language: String,
    pub github_url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupPartialPayload {
    pub user_name: Option<String>,
    pub use_case: Option<String>,
    pub api_key: Option<String>,
    pub input_device_name: Option<String>,
    pub hotkey: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteSetupPayload {
    pub user_name: String,
    pub use_case: String,
    pub api_key: String,
    pub input_device_name: Option<String>,
    pub hotkey: String,
    pub language: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardData {
    pub user_name: String,
    pub has_api_key: bool,
    pub api_key_masked: Option<String>,
    pub input_device_name: Option<String>,
    pub hotkey: String,
    pub language: String,
    pub stats: DashboardStats,
    pub history: Vec<HistoryItem>,
    pub github_url: String,
    pub app_version: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub total_transcriptions: u64,
    pub total_words: u64,
    pub minutes_saved: f32,
    pub wpm: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordHistoryPayload {
    pub text: String,
    pub duration_seconds: f32,
    pub word_count: Option<u32>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsPayload {
    pub user_name: Option<String>,
    pub api_key: Option<String>,
    pub input_device_name: Option<String>,
    pub hotkey: Option<String>,
    pub language: Option<String>,
}

pub fn normalize_hotkey(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        DEFAULT_HOTKEY.to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn normalize_language(input: &str) -> String {
    match input.trim().to_lowercase().as_str() {
        "pt" => "pt".to_string(),
        "en" => "en".to_string(),
        "auto" => "auto".to_string(),
        _ => DEFAULT_LANGUAGE.to_string(),
    }
}

pub fn load_or_create(app: &AppHandle) -> Result<AppConfig, String> {
    let path = config_path(app)?;
    if !path.exists() {
        let config = AppConfig::default();
        save_raw(&path, &config)?;
        return Ok(config);
    }

    let raw = fs::read_to_string(&path).map_err(|e| format!("Failed to read config: {}", e))?;
    match serde_json::from_str::<AppConfig>(&raw) {
        Ok(mut config) => {
            normalize_config(&mut config);
            Ok(config)
        }
        Err(_) => {
            let backup = path.with_extension("json.bak");
            let _ = fs::copy(&path, backup);
            let config = AppConfig::default();
            save_raw(&path, &config)?;
            Ok(config)
        }
    }
}

pub fn save(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let path = config_path(app)?;
    save_raw(&path, config)
}

pub fn setup_state(config: &AppConfig) -> SetupState {
    SetupState {
        setup_completed: config.setup_completed,
        user_name: config.user_name.clone(),
        use_case: config.use_case.clone(),
        has_api_key: config.groq_api_key_obfuscated.is_some(),
        input_device_name: config.input_device_name.clone(),
        hotkey: normalize_hotkey(&config.hotkey),
        language: normalize_language(&config.language),
        github_url: GITHUB_URL.to_string(),
    }
}

pub fn save_setup_partial(app: &AppHandle, payload: SetupPartialPayload) -> Result<AppConfig, String> {
    let mut config = load_or_create(app)?;
    apply_partial(&mut config, payload);
    recompute_stats(&mut config);
    save(app, &config)?;
    Ok(config)
}

pub fn complete_setup(app: &AppHandle, payload: CompleteSetupPayload) -> Result<AppConfig, String> {
    let mut config = load_or_create(app)?;
    config.user_name = payload.user_name.trim().to_string();
    config.use_case = if payload.use_case.trim().is_empty() {
        DEFAULT_USE_CASE.to_string()
    } else {
        payload.use_case.trim().to_string()
    };
    if !payload.api_key.trim().is_empty() {
        config.groq_api_key_obfuscated = Some(obfuscate_api_key(payload.api_key.trim()));
    }
    config.input_device_name = normalize_device_name(payload.input_device_name);
    config.hotkey = normalize_hotkey(&payload.hotkey);
    config.language = normalize_language(&payload.language);
    config.setup_completed = true;
    recompute_stats(&mut config);
    save(app, &config)?;
    Ok(config)
}

pub fn dashboard_data(app: &AppHandle, app_version: &str) -> Result<DashboardData, String> {
    let mut config = load_or_create(app)?;
    recompute_stats(&mut config);
    save(app, &config)?;

    let minutes_saved = if config.stats.total_words == 0 {
        0.0
    } else {
        ((config.stats.total_words as f32 / 130.0) * 10.0).round() / 10.0
    };
    let wpm = if config.stats.total_words == 0 {
        0.0
    } else if config.stats.total_recording_seconds <= 0.1 {
        130.0
    } else {
        ((config.stats.total_words as f32 / (config.stats.total_recording_seconds / 60.0)) * 10.0)
            .round()
            / 10.0
    };

    Ok(DashboardData {
        user_name: config.user_name.clone(),
        has_api_key: config.groq_api_key_obfuscated.is_some(),
        api_key_masked: decode_api_key(&config).map(|key| mask_api_key(&key)),
        input_device_name: config.input_device_name.clone(),
        hotkey: normalize_hotkey(&config.hotkey),
        language: normalize_language(&config.language),
        stats: DashboardStats {
            total_transcriptions: config.stats.total_transcriptions,
            total_words: config.stats.total_words,
            minutes_saved,
            wpm,
        },
        history: config.history,
        github_url: GITHUB_URL.to_string(),
        app_version: app_version.to_string(),
    })
}

pub fn record_history(app: &AppHandle, payload: RecordHistoryPayload) -> Result<(), String> {
    let cleaned_text = payload.text.trim();
    if cleaned_text.is_empty() {
        return Ok(());
    }

    let mut config = load_or_create(app)?;
    let word_count = payload
        .word_count
        .unwrap_or_else(|| count_words(cleaned_text) as u32);
    let duration_seconds = if payload.duration_seconds > 0.05 {
        payload.duration_seconds
    } else {
        estimate_duration_from_words(word_count)
    };

    let item = HistoryItem {
        id: uuid::Uuid::new_v4().to_string(),
        text: cleaned_text.to_string(),
        timestamp: payload.timestamp.unwrap_or_else(|| Utc::now().to_rfc3339()),
        duration_seconds,
        word_count,
    };

    config.history.insert(0, item);
    if config.history.len() > HISTORY_LIMIT {
        config.history.truncate(HISTORY_LIMIT);
    }

    recompute_stats(&mut config);
    save(app, &config)
}

pub fn delete_history_item(app: &AppHandle, id: &str) -> Result<(), String> {
    let mut config = load_or_create(app)?;
    config.history.retain(|item| item.id != id);
    recompute_stats(&mut config);
    save(app, &config)
}

pub fn clear_history(app: &AppHandle) -> Result<(), String> {
    let mut config = load_or_create(app)?;
    config.history.clear();
    recompute_stats(&mut config);
    save(app, &config)
}

pub fn update_settings(app: &AppHandle, payload: UpdateSettingsPayload) -> Result<AppConfig, String> {
    let mut config = load_or_create(app)?;

    if let Some(user_name) = payload.user_name {
        config.user_name = user_name.trim().to_string();
    }

    if let Some(api_key) = payload.api_key {
        let trimmed = api_key.trim();
        if trimmed.is_empty() {
            config.groq_api_key_obfuscated = None;
        } else {
            config.groq_api_key_obfuscated = Some(obfuscate_api_key(trimmed));
        }
    }

    if payload.input_device_name.is_some() {
        config.input_device_name = normalize_device_name(payload.input_device_name);
    }

    if let Some(hotkey) = payload.hotkey {
        config.hotkey = normalize_hotkey(&hotkey);
    }

    if let Some(language) = payload.language {
        config.language = normalize_language(&language);
    }

    recompute_stats(&mut config);
    save(app, &config)?;
    Ok(config)
}

pub fn decode_api_key(config: &AppConfig) -> Option<String> {
    config
        .groq_api_key_obfuscated
        .as_deref()
        .and_then(deobfuscate_api_key)
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .resolve(CONFIG_DIR, BaseDirectory::AppData)
        .map_err(|e| format!("Failed to resolve config dir: {}", e))?;
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {}", e))?;
    Ok(dir.join(CONFIG_FILE))
}

fn save_raw(path: &PathBuf, config: &AppConfig) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    fs::write(path, json).map_err(|e| format!("Failed to save config: {}", e))
}

fn normalize_config(config: &mut AppConfig) {
    config.hotkey = normalize_hotkey(&config.hotkey);
    config.language = normalize_language(&config.language);
    config.input_device_name = normalize_device_name(config.input_device_name.clone());
    if config.use_case.trim().is_empty() {
        config.use_case = DEFAULT_USE_CASE.to_string();
    }
    recompute_stats(config);
}

fn apply_partial(config: &mut AppConfig, payload: SetupPartialPayload) {
    if let Some(user_name) = payload.user_name {
        config.user_name = user_name.trim().to_string();
    }

    if let Some(use_case) = payload.use_case {
        let trimmed = use_case.trim();
        if !trimmed.is_empty() {
            config.use_case = trimmed.to_string();
        }
    }

    if let Some(api_key) = payload.api_key {
        let trimmed = api_key.trim();
        if !trimmed.is_empty() {
            config.groq_api_key_obfuscated = Some(obfuscate_api_key(trimmed));
        }
    }

    if payload.input_device_name.is_some() {
        config.input_device_name = normalize_device_name(payload.input_device_name);
    }

    if let Some(hotkey) = payload.hotkey {
        config.hotkey = normalize_hotkey(&hotkey);
    }

    if let Some(language) = payload.language {
        config.language = normalize_language(&language);
    }
}

fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

fn estimate_duration_from_words(word_count: u32) -> f32 {
    if word_count == 0 {
        0.0
    } else {
        (word_count as f32 / 130.0) * 60.0
    }
}

fn recompute_stats(config: &mut AppConfig) {
    let total_transcriptions = config.history.len() as u64;
    let total_words = config
        .history
        .iter()
        .map(|item| item.word_count as u64)
        .sum::<u64>();
    let total_recording_seconds = config
        .history
        .iter()
        .map(|item| {
            if item.duration_seconds > 0.05 {
                item.duration_seconds
            } else {
                estimate_duration_from_words(item.word_count)
            }
        })
        .sum::<f32>();
    let total_seconds_saved = (total_words as f32 / 130.0) * 60.0;

    config.stats = Stats {
        total_transcriptions,
        total_words,
        total_recording_seconds,
        total_seconds_saved,
    };
}

fn obfuscate_api_key(api_key: &str) -> String {
    let mut bytes = api_key.as_bytes().to_vec();
    for (idx, byte) in bytes.iter_mut().enumerate() {
        *byte ^= API_KEY_XOR_KEY[idx % API_KEY_XOR_KEY.len()];
    }
    BASE64_STANDARD.encode(bytes)
}

fn deobfuscate_api_key(obfuscated: &str) -> Option<String> {
    let mut bytes = BASE64_STANDARD.decode(obfuscated).ok()?;
    for (idx, byte) in bytes.iter_mut().enumerate() {
        *byte ^= API_KEY_XOR_KEY[idx % API_KEY_XOR_KEY.len()];
    }
    String::from_utf8(bytes).ok()
}

fn mask_api_key(api_key: &str) -> String {
    if api_key.len() <= 10 {
        return "******".to_string();
    }

    let prefix = &api_key[..6];
    let suffix = &api_key[api_key.len().saturating_sub(4)..];
    format!("{}********{}", prefix, suffix)
}

fn normalize_device_name(name: Option<String>) -> Option<String> {
    name.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}
