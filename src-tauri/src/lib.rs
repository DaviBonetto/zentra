mod audio;
mod config;
mod orchestrator;
mod paste;
mod prompt_engine;
mod session;
mod stt;
mod tray;

use audio::{AudioBuffer, AudioRecorder};
use config::{
    AppConfig, CompleteSetupPayload, RecordHistoryPayload, SetupPartialPayload, SetupState,
    UpdateSettingsPayload,
};
use cpal::traits::{DeviceTrait, HostTrait};
use orchestrator::FailoverOrchestrator;
use reqwest::Client;
use serde::Serialize;
use session::{SegmentResult, SessionProgress, SessionStitcher, StitchedResult};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tokio::sync::Mutex as TokioMutex;
use tokio::time::sleep;

struct AppState {
    recorder: Arc<Mutex<AudioRecorder>>,
    orchestrator: Arc<TokioMutex<FailoverOrchestrator>>,
    session_stitcher: Arc<TokioMutex<SessionStitcher>>,
    audio_level_flag: Arc<AtomicBool>,
    audio_level_task: Arc<Mutex<Option<tauri::async_runtime::JoinHandle<()>>>>,
    paste_context: Arc<Mutex<paste::PasteContext>>,
    hotkey: Arc<Mutex<String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MicrophoneInfo {
    available: bool,
    name: Option<String>,
}

fn start_audio_level_loop(
    state: &AppState,
    app_handle: tauri::AppHandle,
    level: Arc<std::sync::atomic::AtomicU32>,
) {
    state.audio_level_flag.store(true, Ordering::Relaxed);
    let flag = state.audio_level_flag.clone();
    let emit_handle = app_handle.clone();
    let handle = tauri::async_runtime::spawn(async move {
        while flag.load(Ordering::Relaxed) {
            let bits = level.load(Ordering::Relaxed);
            let value = f32::from_bits(bits).clamp(0.0, 1.0);
            let _ = emit_handle.emit("audio-level", value);
            sleep(std::time::Duration::from_millis(16)).await;
        }
        let _ = emit_handle.emit("audio-level", 0.0f32);
    });

    if let Ok(mut guard) = state.audio_level_task.lock() {
        if let Some(existing) = guard.take() {
            existing.abort();
        }
        *guard = Some(handle);
    }
}

fn stop_audio_level_loop(state: &AppState) {
    state.audio_level_flag.store(false, Ordering::Relaxed);
    if let Ok(mut guard) = state.audio_level_task.lock() {
        if let Some(handle) = guard.take() {
            handle.abort();
        }
    }
}

fn start_capture(
    state: &AppState,
    app_handle: &tauri::AppHandle,
    capture_paste_target: bool,
) -> Result<(), String> {
    let mut recorder = state.recorder.lock().map_err(|e| e.to_string())?;
    recorder.start_recording().map_err(|e| e.to_string())?;
    let level = recorder.audio_level_handle();
    drop(recorder);

    if capture_paste_target {
        let zentra_window = current_zentra_window_handle(app_handle);
        if let Ok(mut paste_context) = state.paste_context.lock() {
            paste_context.capture_target(zentra_window);
        }
    }

    start_audio_level_loop(state, app_handle.clone(), level);
    Ok(())
}

fn stop_capture_and_return_buffer(state: &AppState) -> Result<AudioBuffer, String> {
    let mut recorder = state.recorder.lock().map_err(|e| e.to_string())?;
    let buffer = recorder.stop_recording().map_err(|e| e.to_string())?;
    drop(recorder);
    stop_audio_level_loop(state);
    Ok(buffer)
}

fn stop_capture_safely(state: &AppState) {
    if let Ok(mut recorder) = state.recorder.lock() {
        let _ = recorder.stop_recording();
    }
    stop_audio_level_loop(state);
}

fn register_hotkey(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    hotkey: &str,
) -> Result<(), String> {
    let hotkey = config::normalize_hotkey(hotkey);
    app_handle
        .global_shortcut()
        .unregister_all()
        .map_err(|e| format!("Failed to clear shortcuts: {}", e))?;
    app_handle
        .global_shortcut()
        .register(hotkey.as_str())
        .map_err(|e| format!("Failed to register shortcut '{}': {}", hotkey, e))?;
    if let Ok(mut current) = state.hotkey.lock() {
        *current = hotkey;
    }
    Ok(())
}

fn apply_runtime_config(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    config: &AppConfig,
) -> Result<(), String> {
    match config::decode_api_key(config) {
        Some(api_key) => std::env::set_var("GROQ_API_KEY", api_key),
        None => std::env::remove_var("GROQ_API_KEY"),
    }

    {
        let mut orchestrator = state.orchestrator.blocking_lock();
        *orchestrator = FailoverOrchestrator::from_env();
    }

    register_hotkey(app_handle, state, &config.hotkey)
}

#[tauri::command]
fn start_recording(state: State<'_, AppState>, app_handle: tauri::AppHandle) -> Result<(), String> {
    start_capture(state.inner(), &app_handle, true)
}

#[tauri::command]
fn stop_recording(state: State<'_, AppState>) -> Result<AudioBuffer, String> {
    stop_capture_and_return_buffer(state.inner())
}

#[tauri::command]
fn start_mic_monitor(state: State<'_, AppState>, app_handle: tauri::AppHandle) -> Result<(), String> {
    start_capture(state.inner(), &app_handle, false)
}

#[tauri::command]
fn stop_mic_monitor(state: State<'_, AppState>) -> Result<(), String> {
    stop_capture_safely(state.inner());
    Ok(())
}

#[tauri::command]
fn get_microphone_info() -> MicrophoneInfo {
    let host = cpal::default_host();
    let device = host.default_input_device();
    MicrophoneInfo {
        available: device.is_some(),
        name: device.and_then(|d| d.description().ok().map(|desc| desc.name().to_string())),
    }
}

#[tauri::command]
async fn transcribe_audio(
    audio: AudioBuffer,
    state: State<'_, AppState>,
) -> Result<stt::Transcript, String> {
    let mut orchestrator = state.orchestrator.lock().await;
    orchestrator
        .transcribe(&audio)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_recording_session(state: State<'_, AppState>) -> Result<String, String> {
    let mut stitcher = state.session_stitcher.lock().await;
    stitcher.start_session().await.map_err(|e| format!("{:?}", e))
}

#[tauri::command]
async fn add_audio_segment(
    audio: AudioBuffer,
    state: State<'_, AppState>,
) -> Result<SegmentResult, String> {
    let mut stitcher = state.session_stitcher.lock().await;
    stitcher.add_segment(audio).await.map_err(|e| format!("{:?}", e))
}

#[tauri::command]
async fn finalize_recording_session(state: State<'_, AppState>) -> Result<StitchedResult, String> {
    let mut stitcher = state.session_stitcher.lock().await;
    stitcher.finalize_session().await.map_err(|e| format!("{:?}", e))
}

#[tauri::command]
async fn get_session_progress(state: State<'_, AppState>) -> Result<SessionProgress, String> {
    let stitcher = state.session_stitcher.lock().await;
    Ok(stitcher.get_progress())
}

#[tauri::command]
fn paste_text(state: State<'_, AppState>, app_handle: tauri::AppHandle) -> Result<paste::PasteAttempt, String> {
    let zentra_window = current_zentra_window_handle(&app_handle);
    let mut context = state.paste_context.lock().map_err(|e| e.to_string())?;
    Ok(context.try_auto_paste(zentra_window))
}

#[tauri::command]
fn get_setup_state(app_handle: tauri::AppHandle) -> Result<SetupState, String> {
    let config = config::load_or_create(&app_handle)?;
    Ok(config::setup_state(&config))
}

#[tauri::command]
fn save_setup_partial(
    payload: SetupPartialPayload,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let _ = config::save_setup_partial(&app_handle, payload)?;
    Ok(())
}

#[tauri::command]
fn complete_setup(
    payload: CompleteSetupPayload,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let config = config::complete_setup(&app_handle, payload)?;
    apply_runtime_config(&app_handle, state.inner(), &config)?;

    if let Some(setup_window) = app_handle.get_webview_window("setup") {
        let _ = setup_window.hide();
    }
    if let Some(main_window) = app_handle.get_webview_window("main") {
        let _ = main_window.show();
        let _ = main_window.set_focus();
    }
    let _ = tray::show_dashboard(&app_handle);
    Ok(())
}

#[tauri::command]
async fn validate_groq_key(api_key: String) -> Result<bool, String> {
    if api_key.trim().is_empty() {
        return Ok(false);
    }

    let response = Client::new()
        .get("https://api.groq.com/openai/v1/models")
        .bearer_auth(api_key.trim())
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(response.status().is_success())
}

#[tauri::command]
fn get_dashboard_data(app_handle: tauri::AppHandle) -> Result<config::DashboardData, String> {
    let version = app_handle.package_info().version.to_string();
    config::dashboard_data(&app_handle, &version)
}

#[tauri::command]
fn record_transcription_history(
    payload: RecordHistoryPayload,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    config::record_history(&app_handle, payload)?;
    let _ = app_handle.emit_to("dashboard", "dashboard:history-updated", ());
    Ok(())
}

#[tauri::command]
fn delete_history_item(id: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    config::delete_history_item(&app_handle, &id)
}

#[tauri::command]
fn clear_history(app_handle: tauri::AppHandle) -> Result<(), String> {
    config::clear_history(&app_handle)
}

#[tauri::command]
fn update_settings(
    payload: UpdateSettingsPayload,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let config = config::update_settings(&app_handle, payload)?;
    apply_runtime_config(&app_handle, state.inner(), &config)?;
    Ok(())
}

#[tauri::command]
fn open_dashboard(app_handle: tauri::AppHandle) -> Result<(), String> {
    tray::show_dashboard(&app_handle)
}

#[tauri::command]
fn hide_dashboard(app_handle: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("dashboard") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn hide_main_window(state: State<'_, AppState>, app_handle: tauri::AppHandle) -> Result<(), String> {
    stop_capture_safely(state.inner());
    if let Some(main_window) = app_handle.get_webview_window("main") {
        main_window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn current_zentra_window_handle(app_handle: &tauri::AppHandle) -> isize {
    if let Some(window) = app_handle.get_webview_window("main") {
        #[cfg(target_os = "windows")]
        {
            return window
                .hwnd()
                .map(|hwnd| hwnd.0 as isize)
                .unwrap_or_default();
        }

        #[cfg(target_os = "macos")]
        {
            return window
                .ns_window()
                .map(|handle| handle as isize)
                .unwrap_or_default();
        }
    }

    0
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load environment variables from .env file
    let _ = dotenvy::dotenv();

    let recorder = match AudioRecorder::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to initialize AudioRecorder: {}", e);
            AudioRecorder::new_dummy()
        }
    };

    let configured_hotkey = Arc::new(Mutex::new(config::DEFAULT_HOTKEY.to_string()));
    let orchestrator = Arc::new(TokioMutex::new(FailoverOrchestrator::from_env()));
    let session_stitcher = SessionStitcher::new(orchestrator.clone());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if let Some(main_window) = app.get_webview_window("main") {
                            if let Ok(false) = main_window.is_visible() {
                                let _ = main_window.show();
                                let _ = main_window.set_focus();
                            }
                        }
                        let _ = app.emit("toggle-recording", ());
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(AppState {
            recorder: Arc::new(Mutex::new(recorder)),
            orchestrator,
            session_stitcher: Arc::new(TokioMutex::new(session_stitcher)),
            audio_level_flag: Arc::new(AtomicBool::new(false)),
            audio_level_task: Arc::new(Mutex::new(None)),
            paste_context: Arc::new(Mutex::new(paste::PasteContext::default())),
            hotkey: configured_hotkey.clone(),
        })
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_shadow(false);
                if let Ok(Some(monitor)) = window.primary_monitor() {
                    let work = monitor.work_area();
                    let scale = monitor.scale_factor();
                    let win_size = window
                        .outer_size()
                        .unwrap_or(tauri::PhysicalSize::new(360, 72));
                    let win_width = win_size.width as f64 / scale;
                    let win_height = win_size.height as f64 / scale;
                    let work_x = work.position.x as f64 / scale;
                    let work_y = work.position.y as f64 / scale;
                    let work_w = work.size.width as f64 / scale;
                    let work_h = work.size.height as f64 / scale;
                    let x = work_x + (work_w - win_width) / 2.0;
                    let y = work_y + work_h - win_height - 16.0;
                    let _ = window.set_position(tauri::LogicalPosition::new(x, y));
                }
            }

            let state = app.state::<AppState>();
            let config = config::load_or_create(&app.handle())?;
            apply_runtime_config(&app.handle(), state.inner(), &config)?;
            tray::init_tray(&app.handle())?;

            if let Some(dashboard) = app.get_webview_window("dashboard") {
                let _ = dashboard.hide();
            }

            if config.setup_completed {
                if let Some(main) = app.get_webview_window("main") {
                    let _ = main.show();
                }
                if let Some(setup) = app.get_webview_window("setup") {
                    let _ = setup.hide();
                }
            } else {
                if let Some(main) = app.get_webview_window("main") {
                    let _ = main.hide();
                }
                if let Some(setup) = app.get_webview_window("setup") {
                    let _ = setup.show();
                    let _ = setup.set_focus();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_recording,
            start_mic_monitor,
            stop_mic_monitor,
            get_microphone_info,
            transcribe_audio,
            start_recording_session,
            add_audio_segment,
            finalize_recording_session,
            get_session_progress,
            paste_text,
            get_setup_state,
            save_setup_partial,
            complete_setup,
            validate_groq_key,
            get_dashboard_data,
            record_transcription_history,
            delete_history_item,
            clear_history,
            update_settings,
            open_dashboard,
            hide_dashboard,
            hide_main_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
