mod bridge;
#[cfg(target_os = "linux")]
mod wayland;

use std::sync::{Arc, Mutex};

use poe_data::GameData;
use poe_eval::Profile;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_store::StoreExt;
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

/// Shared game data, loaded once at startup.
struct GameDataState(Arc<GameData>);

/// Active evaluation profile, loaded from JSON data files.
struct ProfileState(Mutex<Option<Profile>>);

/// Read the "dismiss on focus loss" setting from the store (default: true).
fn read_dismiss_on_focus_loss(app: &tauri::AppHandle) -> bool {
    app.store("settings.json")
        .ok()
        .and_then(|store| {
            store
                .get("general")
                .and_then(|v| v.get("dismissOnFocusLoss").and_then(|v| v.as_bool()))
        })
        .unwrap_or(true)
}

/// Read the overlay position setting from the store ("cursor" or "panel").
fn read_overlay_position(app: &tauri::AppHandle) -> String {
    let val = app
        .store("settings.json")
        .ok()
        .and_then(|store| {
            store
                .get("general")
                .and_then(|v| v.get("overlayPosition").and_then(|v| v.as_str().map(String::from)))
        })
        .unwrap_or_else(|| "cursor".into());
    // Migrate legacy value
    if val == "inventoryLeft" { "panel".into() } else { val }
}

/// Calculate overlay position for "panel" mode.
/// PoE's side panels scale with screen height: width = height * (986/1600).
/// Source: Awakened PoE Trade (OverlayWindow.vue `poePanelWidth`).
/// Detects which panel is open based on cursor position:
/// - Cursor on right half → inventory open → overlay left of inventory
/// - Cursor on left half → stash open → overlay right of stash
fn panel_position(window: &tauri::WebviewWindow) -> (i32, i32) {
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

    // PoE side panel width = screen_height * 986/1600 (from Awakened Trade)
    let panel_width = (mon_h as f64 * (986.0 / 1600.0)) as i32;
    let mid_x = mon_x + mon_w / 2;

    let x = if cursor_x >= mid_x {
        // Right half: inventory open → place overlay left of inventory panel
        mon_x + mon_w - panel_width - win_size.width as i32
    } else {
        // Left half: stash open → place overlay right of stash panel
        mon_x + panel_width
    };

    // Y: top-aligned
    let y = mon_y;

    // Clamp to monitor bounds
    let x = x.max(mon_x).min(mon_x + mon_w - win_size.width as i32);
    let y = y
        .max(mon_y)
        .min(mon_y + mon_h - win_size.height as i32);

    (x, y)
}

/// Position the overlay window based on the configured mode.
fn position_overlay(window: &tauri::WebviewWindow, mode: &str) {
    let (x, y) = if mode == "panel" {
        panel_position(window)
    } else {
        let (cx, cy) = get_cursor_position();
        clamp_to_monitor(window, cx, cy)
    };

    #[cfg(target_os = "linux")]
    {
        let is_wayland = window
            .app_handle()
            .try_state::<wayland::WaylandOverlayState>()
            .map(|s| s.active)
            .unwrap_or(false);
        if is_wayland {
            let win = window.clone();
            let win2 = win.clone();
            let _ = win.run_on_main_thread(move || {
                if let Ok(gtk_win) = win2.gtk_window() {
                    wayland::position_layer_surface(&gtk_win, x, y);
                }
            });
            return;
        }
    }

    let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(x, y)));
}

/// Send Ctrl+Alt+C to the foreground window (PoE) via enigo,
/// wait briefly, then read clipboard, parse, evaluate, and emit to frontend.
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

        // Try to parse and evaluate the item
        let gd = &app.state::<GameDataState>().0;
        let profile = app.state::<ProfileState>().0.lock().unwrap().clone();
        match poe_item::parse(&clipboard_text) {
            Ok(raw) => {
                let resolved = poe_item::resolve(&raw, gd);
                let evaluated =
                    bridge::build_evaluated_item(&resolved, gd, profile.as_ref());
                let _ = app.emit("item-evaluated", &evaluated);
            }
            Err(e) => {
                eprintln!("Item parse failed: {e}");
                // Fall back to raw text so the overlay still shows something
                let _ = app.emit("item-captured", &clipboard_text);
            }
        }
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
    if let Some(pos) = wayland::get_cursor_position_hyprland() {
        return pos;
    }
    // Fallback for X11 / non-Hyprland compositors
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

/// Reposition the overlay after the frontend has resized it.
/// Called from the frontend after auto-resize so the position accounts
/// for the actual (post-zoom) window size.
#[tauri::command]
fn reposition_overlay(app: tauri::AppHandle, window: tauri::WebviewWindow) {
    let mode = read_overlay_position(&app);
    position_overlay(&window, &mode);
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

/// Load hotkey config from the tauri-plugin-store settings file.
/// Falls back to defaults if the store doesn't exist or keys are missing.
fn load_hotkey_config(app: &tauri::AppHandle) -> HotkeyConfig {
    let defaults = HotkeyConfig::default();

    let Ok(store) = app.store("settings.json") else {
        return defaults;
    };

    let Some(hotkeys_val) = store.get("hotkeys") else {
        return defaults;
    };

    HotkeyConfig {
        inspect_item: hotkeys_val
            .get("inspectItem")
            .and_then(|v| v.as_str())
            .map(|s| s.to_lowercase())
            .unwrap_or(defaults.inspect_item),
        open_settings: hotkeys_val
            .get("openSettings")
            .and_then(|v| v.as_str())
            .map(|s| s.to_lowercase())
            .unwrap_or(defaults.open_settings),
    }
}

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

/// Parse and evaluate item text from clipboard.
/// Returns a structured `EvaluatedItem` for the overlay, or an error string.
#[tauri::command]
fn evaluate_item(
    item_text: String,
    state: tauri::State<'_, GameDataState>,
) -> Result<bridge::EvaluatedItem, String> {
    let gd = &state.0;

    // Pass 1: structural parse
    let raw = poe_item::parse(&item_text).map_err(|e| format!("Parse error: {e}"))?;

    // Pass 2: resolve against game data
    let resolved = poe_item::resolve(&raw, gd);

    // Build frontend-compatible response (no profile for direct command calls)
    Ok(bridge::build_evaluated_item(&resolved, gd, None))
}

/// Return the predicate schema so the frontend can build profile editors dynamically.
#[tauri::command]
fn get_predicate_schema() -> Vec<poe_eval::PredicateSchema> {
    poe_eval::predicate_schema()
}

/// Return suggestion values for a given data source.
/// Used by the profile editor for autocomplete on text fields.
#[tauri::command]
fn get_suggestions(source: String, state: tauri::State<'_, GameDataState>) -> Vec<String> {
    let gd = &state.0;
    match source.as_str() {
        "item_classes" => {
            let mut names: Vec<String> = gd.item_classes.iter().map(|c| c.name.clone()).collect();
            names.sort();
            names
        }
        "base_types" => {
            let mut names: Vec<String> =
                gd.base_item_types.iter().map(|b| b.name.clone()).collect();
            names.sort();
            names
        }
        "mod_names" => {
            let mut names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
            for m in &gd.mods {
                if !m.name.is_empty() {
                    names.insert(m.name.clone());
                }
            }
            names.into_iter().collect()
        }
        "stat_texts" => gd
            .reverse_index
            .as_ref()
            .map(|ri| {
                let mut keys = ri.template_keys();
                keys.sort();
                keys
            })
            .unwrap_or_default(),
        _ => vec![],
    }
}

/// Load game data from extracted datc64 files.
///
/// Looks for data in these locations (first match wins):
/// 1. `POE_DATA_DIR` environment variable
/// 2. `data/` directory next to the executable
/// 3. `%TEMP%/poe-dat/` (dev fallback — same dir used by poe-data tests)
///
/// Returns empty GameData if no data directory is found (overlay still works,
/// just without stat resolution or open affix detection).
fn load_game_data() -> GameData {
    let candidates = [
        std::env::var("POE_DATA_DIR").ok().map(std::path::PathBuf::from),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("data"))),
        // Committed game data in the repo (dev + release)
        Some(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/poe-data/data")),
        Some(std::env::temp_dir().join("poe-dat")),
    ];

    for candidate in candidates.iter().flatten() {
        if candidate.join("stats.datc64").exists() {
            match poe_data::load(candidate) {
                Ok(gd) => {
                    eprintln!("Loaded game data from {}", candidate.display());
                    return gd;
                }
                Err(e) => {
                    eprintln!("Failed to load game data from {}: {e}", candidate.display());
                }
            }
        }
    }

    eprintln!("No game data found — running without stat resolution");
    GameData::new(vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![])
}

/// Built-in default profile — compiled into the binary, can never be deleted.
const DEFAULT_PROFILE_JSON: &str = include_str!("../data/profiles/generic.json");

/// Parse the built-in default profile.
fn default_profile() -> Option<Profile> {
    match serde_json::from_str(DEFAULT_PROFILE_JSON) {
        Ok(p) => Some(p),
        Err(e) => {
            eprintln!("Failed to parse built-in default profile: {e}");
            None
        }
    }
}

pub fn run() {
    // WebKitGTK's DMA-BUF renderer has a bug with explicit sync on Wayland
    // compositors (Hyprland): it creates a wp_linux_drm_syncobj_surface but
    // doesn't set the acquire timeline before committing, causing a fatal
    // "Missing acquire timeline" protocol error. Disable DMA-BUF rendering
    // to use shared-memory buffers instead. Must be set before GTK init.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var_os("XDG_SESSION_TYPE")
            .is_some_and(|v| v == "wayland")
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

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
        .manage(GameDataState(Arc::new(load_game_data())))
        .manage(ProfileState(Mutex::new(default_profile())))
        .invoke_handler(tauri::generate_handler![
            reposition_overlay,
            dismiss_overlay,
            update_hotkeys,
            pause_hotkeys,
            resume_hotkeys,
            get_autostart,
            set_autostart,
            evaluate_item,
            get_predicate_schema,
            get_suggestions,
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

            // --- Wayland layer-shell setup ---
            #[cfg(target_os = "linux")]
            {
                let mut wayland_active = false;
                if let Some(overlay) = app.get_webview_window("overlay") {
                    if let Ok(gtk_win) = overlay.gtk_window() {
                        if wayland::detect_wayland(&gtk_win) {
                            wayland::init_overlay_layer_shell(&gtk_win);
                            wayland_active = true;
                            eprintln!("Wayland detected — layer-shell overlay initialized");
                        }
                    }
                }
                app.manage(wayland::WaylandOverlayState {
                    active: wayland_active,
                });
            }

            // --- Global hotkeys from stored settings (or defaults) ---
            #[cfg(desktop)]
            {
                let handle = app.handle().clone();
                handle.plugin(tauri_plugin_global_shortcut::Builder::new().build())?;

                // Load saved hotkeys from the store so custom bindings work on startup
                let config = load_hotkey_config(app.handle());
                register_hotkeys(app.handle(), &config);
                *app.state::<HotkeyState>().0.lock().unwrap() = config;
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

            if start_minimized {
                // Explicitly hide — window-state plugin may have restored it as visible
                if let Some(window) = app.get_webview_window("settings") {
                    let _ = window.hide();
                }
            } else {
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

            // Dismiss overlay on focus loss (if enabled in settings)
            if window.label() == "overlay" {
                if let tauri::WindowEvent::Focused(false) = event {
                    if read_dismiss_on_focus_loss(window.app_handle()) {
                        let _ = window.hide();
                        let _ = window.emit("overlay-dismissed", ());
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
