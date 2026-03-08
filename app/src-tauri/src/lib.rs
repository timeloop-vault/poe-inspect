use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_store::StoreExt;
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

/// Read the overlay position setting from the store ("cursor" or "inventoryLeft").
fn read_overlay_position(app: &tauri::AppHandle) -> String {
    app.store("settings.json")
        .ok()
        .and_then(|store| {
            store
                .get("general")
                .and_then(|v| v.get("overlayPosition").and_then(|v| v.as_str().map(String::from)))
        })
        .unwrap_or_else(|| "cursor".into())
}

/// Calculate overlay position for "inventoryLeft" mode.
/// PoE's inventory panel scales with screen height: width ≈ height * 0.587.
/// Places the overlay to the left of where the inventory panel would be,
/// vertically centered in the top half of the screen.
fn inventory_left_position(window: &tauri::WebviewWindow) -> (i32, i32) {
    let (cursor_x, cursor_y) = get_cursor_position();

    // Find monitor containing cursor
    let monitors = window.available_monitors().unwrap_or_default();
    let monitor = monitors.iter().find(|m| {
        let pos = m.position();
        let size = m.size();
        cursor_x >= pos.x
            && cursor_x < pos.x + size.width as i32
            && cursor_y >= pos.y
            && cursor_y < pos.y + size.height as i32
    });

    let (mon_x, mon_y, mon_w, mon_h) = match monitor {
        Some(m) => (
            m.position().x,
            m.position().y,
            m.size().width as i32,
            m.size().height as i32,
        ),
        None => return (200, 200),
    };

    let win_size = window
        .outer_size()
        .unwrap_or(tauri::PhysicalSize::new(440, 900));

    // PoE inventory panel width ≈ screen_height * 0.587
    let panel_width = (mon_h as f64 * 0.587) as i32;
    let padding = 10;

    // X: left of the inventory panel
    let x = mon_x + mon_w - panel_width - win_size.width as i32 - padding;
    // Y: vertically offset from top, roughly where items appear
    let y = mon_y + (mon_h as f64 * 0.15) as i32;

    // Clamp to monitor bounds
    let x = x.max(mon_x);
    let y = y
        .max(mon_y)
        .min(mon_y + mon_h - win_size.height as i32);

    (x, y)
}

/// Position the overlay window based on the configured mode.
fn position_overlay(window: &tauri::WebviewWindow, mode: &str) {
    let (x, y) = if mode == "inventoryLeft" {
        inventory_left_position(window)
    } else {
        let (cx, cy) = get_cursor_position();
        clamp_to_monitor(window, cx, cy)
    };
    let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(x, y)));
}

/// Send Ctrl+Alt+C to the foreground window (PoE) via enigo,
/// wait briefly, then read clipboard and emit to frontend.
fn handle_inspect(app: &tauri::AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        // Send Ctrl+Alt+C to PoE
        if let Err(e) = send_copy_keystroke() {
            eprintln!("Failed to send keystroke: {e}");
            return;
        }

        // Wait for clipboard to populate
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Read clipboard
        let clipboard_text: String = match app.clipboard().read_text() {
            Ok(text) => text,
            Err(e) => {
                eprintln!("Failed to read clipboard: {e}");
                return;
            }
        };

        if clipboard_text.is_empty() {
            return;
        }

        // Position and show the overlay window
        if let Some(window) = app.get_webview_window("overlay") {
            let mode = read_overlay_position(&app);
            position_overlay(&window, &mode);
            let _ = window.set_ignore_cursor_events(false);
            let _ = window.show();
            let _ = window.set_focus();
        }

        // Emit item text to frontend
        let _ = app.emit("item-captured", &clipboard_text);
    });
}

/// Send Ctrl+Alt+C keystroke to the active window via enigo.
fn send_copy_keystroke() -> Result<(), String> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Alt, Direction::Press)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Unicode('c'), Direction::Click)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Alt, Direction::Release)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Get the current cursor position (screen coordinates).
#[cfg(target_os = "windows")]
fn get_cursor_position() -> (i32, i32) {
    use std::mem::MaybeUninit;

    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }

    extern "system" {
        fn GetCursorPos(point: *mut Point) -> i32;
    }

    let mut point = MaybeUninit::<Point>::uninit();
    let success = unsafe { GetCursorPos(point.as_mut_ptr()) };
    if success != 0 {
        let point = unsafe { point.assume_init() };
        // Offset slightly so overlay doesn't cover the cursor
        (point.x + 20, point.y + 20)
    } else {
        (100, 100)
    }
}

#[cfg(target_os = "macos")]
fn get_cursor_position() -> (i32, i32) {
    // TODO: Use Core Graphics CGEventGetLocation or NSEvent.mouseLocation
    // See: https://developer.apple.com/documentation/coregraphics/1456611-cgeventgetlocation
    (100, 100)
}

#[cfg(target_os = "linux")]
fn get_cursor_position() -> (i32, i32) {
    // TODO: X11 — use XQueryPointer via x11 crate
    // Wayland — no standard API, may need libei or compositor-specific approach
    (100, 100)
}

/// Clamp overlay position to stay within the monitor that contains the cursor.
/// Offset 20px from cursor. If the overlay would overflow the right or bottom
/// edge, flip to the other side of the cursor.
fn clamp_to_monitor(window: &tauri::WebviewWindow, cursor_x: i32, cursor_y: i32) -> (i32, i32) {
    let offset = 20;
    let win_size = window
        .outer_size()
        .unwrap_or(tauri::PhysicalSize::new(440, 900));

    // Find the monitor containing the cursor
    let monitors = window.available_monitors().unwrap_or_default();
    let monitor = monitors.iter().find(|m| {
        let pos = m.position();
        let size = m.size();
        cursor_x >= pos.x
            && cursor_x < pos.x + size.width as i32
            && cursor_y >= pos.y
            && cursor_y < pos.y + size.height as i32
    });

    let (mon_x, mon_y, mon_w, mon_h) = match monitor {
        Some(m) => (
            m.position().x,
            m.position().y,
            m.size().width as i32,
            m.size().height as i32,
        ),
        None => {
            // Fallback: just offset from cursor
            return (cursor_x + offset, cursor_y + offset);
        }
    };

    let mon_right = mon_x + mon_w;
    let mon_bottom = mon_y + mon_h;

    // Try placing to the right and below cursor
    let mut x = cursor_x + offset;
    let mut y = cursor_y + offset;

    // Flip horizontally if overflow
    if x + win_size.width as i32 > mon_right {
        x = cursor_x - offset - win_size.width as i32;
    }
    // Flip vertically if overflow
    if y + win_size.height as i32 > mon_bottom {
        y = cursor_y - offset - win_size.height as i32;
    }

    // Final clamp to monitor edges (in case window is larger than remaining space)
    x = x.max(mon_x).min(mon_right - win_size.width as i32);
    y = y.max(mon_y).min(mon_bottom - win_size.height as i32);

    (x, y)
}

/// Dismiss the overlay: hide window, notify frontend.
#[tauri::command]
fn dismiss_overlay(window: tauri::WebviewWindow) {
    let _ = window.hide();
    let _ = window.emit("overlay-dismissed", ());
}

/// Show the overlay window with mock data for testing (tray debug button).
fn show_debug_overlay(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("overlay") {
        let mode = read_overlay_position(app);
        position_overlay(&window, &mode);
        let _ = window.set_ignore_cursor_events(false);
        let _ = window.show();
        let _ = window.set_focus();
    }
    let _ = app.emit("show-debug-overlay", ());
}

/// Show the settings window (create if needed, or just show+focus).
fn show_settings(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Global shortcut config — only inspect and settings are registered as global
/// shortcuts. Dismiss is handled at the overlay window level to avoid consuming
/// keys like Escape system-wide.
#[derive(Debug, Clone)]
struct HotkeyConfig {
    inspect_item: String,
    open_settings: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            inspect_item: "ctrl+i".into(),
            open_settings: "ctrl+shift+i".into(),
        }
    }
}

struct HotkeyState(std::sync::Mutex<HotkeyConfig>);

/// Register global shortcuts from config. Only inspect + settings are global.
fn register_hotkeys(app: &tauri::AppHandle, config: &HotkeyConfig) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    let entries: [(&str, &str); 2] = [
        (&config.inspect_item, "inspect"),
        (&config.open_settings, "settings"),
    ];

    for (shortcut_str, action_name) in entries {
        let action = action_name.to_string();
        if let Err(e) = gs.on_shortcut(shortcut_str, move |app, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            match action.as_str() {
                "inspect" => {
                    let settings_focused = app
                        .get_webview_window("settings")
                        .and_then(|w| w.is_focused().ok())
                        .unwrap_or(false);
                    if !settings_focused {
                        handle_inspect(app);
                    }
                }
                "settings" => show_settings(app),
                _ => {}
            }
        }) {
            eprintln!("Failed to register shortcut '{shortcut_str}': {e}");
        }
    }
}

/// Update global hotkeys from frontend when settings change.
/// dismiss_overlay is accepted but not registered globally — it's handled
/// at the overlay window level via a keydown listener.
#[tauri::command]
fn update_hotkeys(
    app: tauri::AppHandle,
    inspect_item: String,
    #[allow(unused_variables)] dismiss_overlay: String,
    open_settings: String,
) {
    let config = HotkeyConfig {
        inspect_item,
        open_settings,
    };
    register_hotkeys(&app, &config);
    *app.state::<HotkeyState>().0.lock().unwrap() = config;
}

/// Unregister all global hotkeys (used during hotkey capture in settings).
#[tauri::command]
fn pause_hotkeys(app: tauri::AppHandle) {
    let _ = app.global_shortcut().unregister_all();
}

/// Re-register global hotkeys from saved config (used after hotkey capture).
#[tauri::command]
fn resume_hotkeys(app: tauri::AppHandle) {
    let config = app.state::<HotkeyState>().0.lock().unwrap().clone();
    register_hotkeys(&app, &config);
}

/// Get the current autostart state.
#[tauri::command]
fn get_autostart(app: tauri::AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

/// Enable or disable launch on boot.
#[tauri::command]
fn set_autostart(app: tauri::AppHandle, enabled: bool) {
    let launcher = app.autolaunch();
    if enabled {
        let _ = launcher.enable();
    } else {
        let _ = launcher.disable();
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_denylist(&["overlay"])
                .build(),
        )
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(HotkeyState(std::sync::Mutex::new(HotkeyConfig::default())))
        .invoke_handler(tauri::generate_handler![
            dismiss_overlay,
            update_hotkeys,
            pause_hotkeys,
            resume_hotkeys,
            get_autostart,
            set_autostart,
        ])
        .setup(|app| {
            // --- System tray ---
            let show_overlay = MenuItem::with_id(
                app,
                "show_overlay",
                "Show Overlay (Debug)",
                true,
                None::<&str>,
            )?;
            let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_overlay, &settings, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("PoE Inspect")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show_overlay" => show_debug_overlay(app),
                    "settings" => show_settings(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                        show_settings(tray.app_handle());
                    }
                })
                .build(app)?;

            // --- Global hotkeys from settings (or defaults) ---
            #[cfg(desktop)]
            {
                let handle = app.handle().clone();
                handle.plugin(tauri_plugin_global_shortcut::Builder::new().build())?;
                // Register with default config; frontend will call update_hotkeys
                // with stored settings once it loads
                let config = app.state::<HotkeyState>().0.lock().unwrap().clone();
                register_hotkeys(app.handle(), &config);
            }

            // --- Start minimized check ---
            // Read the store to decide whether to show the settings window on startup.
            // Default: start minimized (just tray icon). If false, show settings.
            let start_minimized = app
                .store("settings.json")
                .ok()
                .and_then(|store| {
                    store
                        .get("general")
                        .and_then(|v| v.get("startMinimized").and_then(|v| v.as_bool()))
                })
                .unwrap_or(true);

            if !start_minimized {
                show_settings(app.handle());
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            // Hide settings window on close instead of destroying it,
            // and save window state so position/size persists across restarts
            if window.label() == "settings" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    let _ = window.app_handle().save_window_state(StateFlags::all());
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
