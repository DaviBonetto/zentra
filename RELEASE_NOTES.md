# Zentra v1.0.2 - Mic + STT Reliability Hotfix

This hotfix restores reliable voice capture/transcription and improves setup/dashboard robustness while keeping the approved Zentra UI and workflow.

## Highlights
- Fixed Setup Step 4 microphone test lifecycle (no monitor freeze/re-entry).
- Added explicit microphone device selection and persistence.
- Improved default device fallback and reduced loopback capture risk.
- Locked production STT path to Groq-only for predictable output.
- Added clearer runtime errors for missing/invalid Groq key.
- Added dashboard window controls: minimize, maximize/restore, close (hide-only).
- Updated README screenshots with real captures (bar, setup, dashboard).

## Installer
- `Zentra_1.0.2_x64-setup.exe`

## Requirements
- Windows 10/11 (x64)
- Free Groq API key ([console.groq.com](https://console.groq.com))

---

If Zentra saves you time, please star the repository.
