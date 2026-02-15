use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub const MENU_OPEN_DASHBOARD: &str = "tray-open-dashboard";
pub const MENU_OPEN_SETTINGS: &str = "tray-open-settings";
pub const MENU_QUIT: &str = "tray-quit";

pub fn init_tray<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let open_dashboard = MenuItem::with_id(
        app,
        MENU_OPEN_DASHBOARD,
        "Open Dashboard",
        true,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;
    let open_settings = MenuItem::with_id(
        app,
        MENU_OPEN_SETTINGS,
        "Settings",
        true,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit Zentra", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let separator = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;

    let menu = Menu::with_items(app, &[&open_dashboard, &open_settings, &separator, &quit])
        .map_err(|e| e.to_string())?;

    let mut tray_builder = TrayIconBuilder::with_id("zentra-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("Zentra")
        .on_menu_event(|app, event| {
            match event.id().0.as_str() {
                MENU_OPEN_DASHBOARD => {
                    let _ = show_dashboard(app);
                }
                MENU_OPEN_SETTINGS => {
                    let _ = show_dashboard(app);
                    let _ = app.emit_to("dashboard", "dashboard:navigate", "settings");
                }
                MENU_QUIT => app.exit(0),
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app_handle = tray.app_handle().clone();
                let _ = show_dashboard(&app_handle);
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }

    tray_builder.build(app).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn show_dashboard<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let Some(window) = app.get_webview_window("dashboard") else {
        return Err("dashboard window not found".to_string());
    };

    window.show().map_err(|e| e.to_string())?;
    window.unminimize().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    let _ = app.emit_to("dashboard", "dashboard:refresh", ());
    Ok(())
}
