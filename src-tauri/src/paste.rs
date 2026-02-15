use serde::Serialize;
use std::{thread, time::Duration};

#[derive(Debug, Clone, Serialize)]
pub struct PasteAttempt {
    pub pasted: bool,
    pub reason: Option<String>,
}

impl PasteAttempt {
    fn pasted() -> Self {
        Self {
            pasted: true,
            reason: None,
        }
    }

    fn fallback(reason: impl Into<String>) -> Self {
        Self {
            pasted: false,
            reason: Some(reason.into()),
        }
    }
}

#[derive(Debug, Default)]
pub struct PasteContext {
    #[cfg(target_os = "windows")]
    target_hwnd: Option<isize>,
}

impl PasteContext {
    pub fn capture_target(&mut self, zentra_window: isize) {
        #[cfg(target_os = "windows")]
        {
            self.target_hwnd = capture_target_window(zentra_window);
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = zentra_window;
        }
    }

    pub fn try_auto_paste(&mut self, zentra_window: isize) -> PasteAttempt {
        #[cfg(target_os = "windows")]
        {
            let attempt = try_auto_paste_windows(self.target_hwnd, zentra_window);
            self.target_hwnd = None;
            return attempt;
        }

        #[cfg(target_os = "macos")]
        {
            let _ = zentra_window;
            return try_auto_paste_macos();
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            let _ = zentra_window;
            PasteAttempt::fallback("unsupported_platform")
        }
    }
}

#[cfg(target_os = "windows")]
fn is_same_window(a: isize, b: isize) -> bool {
    a != 0 && b != 0 && a == b
}

#[cfg(target_os = "windows")]
fn capture_target_window(zentra_window: isize) -> Option<isize> {
    use winapi::um::winuser::GetForegroundWindow;

    unsafe {
        let hwnd = GetForegroundWindow() as isize;
        if hwnd == 0 || is_same_window(hwnd, zentra_window) {
            None
        } else {
            Some(hwnd)
        }
    }
}

#[cfg(target_os = "windows")]
fn try_auto_paste_windows(target_hwnd: Option<isize>, zentra_window: isize) -> PasteAttempt {
    use std::mem;
    use winapi::shared::windef::HWND;
    use winapi::um::winuser::{
        GetForegroundWindow, SendInput, SetForegroundWindow, INPUT, VK_CONTROL,
    };

    const VK_V_KEY: u16 = 0x56;

    thread::sleep(Duration::from_millis(150));

    let target_hwnd = match target_hwnd {
        Some(hwnd) if hwnd != 0 => hwnd,
        _ => return PasteAttempt::fallback("no_target_window"),
    };

    unsafe {
        let mut current_hwnd = GetForegroundWindow() as isize;
        if current_hwnd == 0 {
            return PasteAttempt::fallback("no_foreground_window");
        }

        if is_same_window(current_hwnd, zentra_window) {
            let restored = SetForegroundWindow(target_hwnd as HWND) != 0;
            if !restored {
                return PasteAttempt::fallback("restore_focus_failed");
            }
            thread::sleep(Duration::from_millis(60));
            current_hwnd = GetForegroundWindow() as isize;
        }

        if current_hwnd != target_hwnd {
            return PasteAttempt::fallback("focus_changed");
        }

        if let Some(class_name) = window_class_name(target_hwnd as HWND) {
            if is_non_paste_window_class(&class_name) {
                return PasteAttempt::fallback(format!("unsupported_target_class:{}", class_name));
            }
        }

        if !has_focused_control(target_hwnd as HWND) {
            return PasteAttempt::fallback("no_focused_control");
        }

        let mut inputs: [INPUT; 4] = [
            make_key_input(VK_CONTROL as u16, false),
            make_key_input(VK_V_KEY, false),
            make_key_input(VK_V_KEY, true),
            make_key_input(VK_CONTROL as u16, true),
        ];

        let sent = SendInput(
            inputs.len() as u32,
            inputs.as_mut_ptr(),
            mem::size_of::<INPUT>() as i32,
        );

        if sent == inputs.len() as u32 {
            PasteAttempt::pasted()
        } else {
            PasteAttempt::fallback("send_input_incomplete")
        }
    }
}

#[cfg(target_os = "windows")]
fn window_class_name(hwnd: winapi::shared::windef::HWND) -> Option<String> {
    use winapi::um::winuser::GetClassNameW;

    let mut class_name = [0u16; 256];
    let length = unsafe { GetClassNameW(hwnd, class_name.as_mut_ptr(), class_name.len() as i32) };

    if length <= 0 {
        return None;
    }

    Some(String::from_utf16_lossy(&class_name[..length as usize]))
}

#[cfg(target_os = "windows")]
fn is_non_paste_window_class(class_name: &str) -> bool {
    let normalized = class_name.trim().to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "consolewindowclass"
            | "cascadia_hosting_window_class"
            | "virtualconsoleclass"
            | "applicationframewindow"
    )
}

#[cfg(target_os = "windows")]
fn has_focused_control(target_hwnd: winapi::shared::windef::HWND) -> bool {
    use std::{mem, ptr};
    use winapi::shared::minwindef::DWORD;
    use winapi::um::winuser::{GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO};

    unsafe {
        let thread_id = GetWindowThreadProcessId(target_hwnd, ptr::null_mut());
        if thread_id == 0 {
            return false;
        }

        let mut info: GUITHREADINFO = mem::zeroed();
        info.cbSize = mem::size_of::<GUITHREADINFO>() as DWORD;

        if GetGUIThreadInfo(thread_id, &mut info) == 0 {
            return false;
        }

        !info.hwndFocus.is_null() || !info.hwndCaret.is_null()
    }
}

#[cfg(target_os = "windows")]
unsafe fn make_key_input(vk: u16, key_up: bool) -> winapi::um::winuser::INPUT {
    let mut input: winapi::um::winuser::INPUT = std::mem::zeroed();
    input.type_ = winapi::um::winuser::INPUT_KEYBOARD;
    *input.u.ki_mut() = winapi::um::winuser::KEYBDINPUT {
        wVk: vk,
        wScan: 0,
        dwFlags: if key_up {
            winapi::um::winuser::KEYEVENTF_KEYUP
        } else {
            0
        },
        time: 0,
        dwExtraInfo: 0,
    };
    input
}

#[cfg(target_os = "macos")]
const MACOS_PASTE_DELAY_MS: u64 = 180;

#[cfg(target_os = "macos")]
fn try_auto_paste_macos() -> PasteAttempt {
    use std::process::Command;

    thread::sleep(Duration::from_millis(MACOS_PASTE_DELAY_MS));

    let output = Command::new("osascript")
        .args([
            "-e",
            r#"tell application "System Events" to keystroke "v" using command down"#,
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => PasteAttempt::pasted(),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if stderr.is_empty() {
                PasteAttempt::fallback("macos_applescript_failed_accessibility")
            } else {
                PasteAttempt::fallback(format!("macos_applescript_failed: {}", stderr))
            }
        }
        Err(err) => PasteAttempt::fallback(format!("macos_applescript_error: {}", err)),
    }
}


