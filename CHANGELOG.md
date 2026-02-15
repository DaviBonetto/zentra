# Changelog

All notable changes to Zentra will be documented here.

## [1.0.1] - 2026-02-15

### Fixed
- Added explicit microphone input device selection and persisted it in app config.
- Prevented Setup Step 4 microphone monitor re-entry/race conditions and stabilized monitor lifecycle.
- Added dashboard window controls (minimize, maximize/restore, close/hide) with reliable behavior.
- Ensured setup completion safely stops monitor capture before transitioning windows.

### Improved
- Added input device listing/selection commands for setup and runtime configuration.
- Improved audio input fallback by avoiding loopback-like default devices when possible.
- Updated docs screenshots with real product captures (bar, setup, dashboard).

## [1.0.0] — 2026-02-15

### Added
- Floating bar UI with glass morphism design
- Global hotkey (`Ctrl+Shift+Space`) for instant voice capture
- Groq API integration (Whisper large-v3) — ~5–10s transcription
- Auto-paste via Windows SendInput API
- VOSK offline fallback for internet-free usage
- Silero VAD for accurate voice activity detection
- Session stitching for recordings beyond 59s
- Failover orchestrator with circuit breaker pattern
- Setup wizard with API key validation (4 steps)
- Dashboard with transcription history and stats
- System tray integration
- Configurable hotkey via UI
- Toast notifications ("Pasted!" / "Copied!")
- Dark glass morphism UI — DM Sans typography
