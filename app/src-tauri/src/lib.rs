mod bridge;
#[cfg(target_os = "linux")]
mod clipboard;
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


/// Cursor position in CSS pixels, emitted to the frontend for panel positioning.
#[derive(serde::Serialize, Clone)]
struct OverlayPosition {
    x: f64,
    y: f64,
}

/// Expand the overlay window to fill the monitor containing the cursor.
/// Returns the cursor position in CSS pixels relative to the window.
fn setup_fullscreen_overlay(
    window: &tauri::WebviewWindow,
    cursor_x: i32,
    cursor_y: i32,
) -> (f64, f64) {
    let monitors = window.available_monitors().unwrap_or_default();
    let monitor = monitors.iter().find(|m| {
        let pos = m.position();
        let size = m.size();
        cursor_x >= pos.x
            && cursor_x < pos.x + size.width as i32
            && cursor_y >= pos.y
            && cursor_y < pos.y + size.height as i32
    });

    let Some(monitor) = monitor else {
        return (200.0, 200.0);
    };

    let mon_pos = monitor.position();
    let mon_size = monitor.size();
    let scale = monitor.scale_factor();

    // Skip window positioning on Wayland (layer-shell manages geometry)
    #[cfg(target_os = "linux")]
    let is_wayland = window
        .app_handle()
        .try_state::<wayland::WaylandOverlayState>()
        .map(|s| s.active)
        .unwrap_or(false);
    #[cfg(not(target_os = "linux"))]
    let is_wayland = false;

    if !is_wayland {
        let _ = window.set_position(tauri::Position::Physical(
            tauri::PhysicalPosition::new(mon_pos.x, mon_pos.y),
        ));
        let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(
            mon_size.width,
            mon_size.height,
        )));
    }

    // Cursor position in CSS pixels relative to the window
    let css_x = (cursor_x - mon_pos.x) as f64 / scale;
    let css_y = (cursor_y - mon_pos.y) as f64 / scale;
    (css_x, css_y)
}

/// Send Ctrl+Alt+C to PoE and read the resulting clipboard text.
///
/// Linux: Wayland watcher (event-driven, <1ms) → X11 direct fallback
/// (retry keystroke, poll for change). Non-Linux: send keystroke → poll
/// via Tauri clipboard plugin.
fn handle_inspect(app: &tauri::AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        // Capture cursor position BEFORE sending keystroke (while hovering the item)
        let (cursor_x, cursor_y) = get_cursor_position();

        let clipboard_text = acquire_clipboard(&app);

        let Some(clipboard_text) = clipboard_text else {
            eprintln!("[inspect] No clipboard text, aborting");
            return;
        };

        let preview: String = clipboard_text.chars().take(80).collect();
        eprintln!("[inspect] Got {} bytes — {preview:?}", clipboard_text.len());

        // Expand overlay to fill the monitor and emit cursor position
        if let Some(window) = app.get_webview_window("overlay") {
            let (css_x, css_y) = setup_fullscreen_overlay(&window, cursor_x, cursor_y);
            let _ = app.emit("overlay-position", OverlayPosition { x: css_x, y: css_y });
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

/// Send Ctrl+Alt+C to PoE and read the resulting clipboard text.
#[cfg(target_os = "linux")]
fn acquire_clipboard(app: &tauri::AppHandle) -> Option<String> {
    clipboard::acquire_clipboard(app)
}

/// Send Ctrl+Alt+C to PoE and read the resulting clipboard text.
#[cfg(not(target_os = "linux"))]
fn acquire_clipboard(app: &tauri::AppHandle) -> Option<String> {
    if let Err(e) = send_copy_keystroke() {
        eprintln!("[inspect] Keystroke FAILED: {e}");
        return None;
    }

    let delays_ms = [100, 80, 80, 100, 150];
    for (attempt, &delay) in delays_ms.iter().enumerate() {
        std::thread::sleep(std::time::Duration::from_millis(delay));
        match app.clipboard().read_text() {
            Ok(text) if !text.is_empty() => return Some(text),
            Ok(_) => {}
            Err(e) => {
                eprintln!("[inspect] Clipboard read error (attempt {}): {e}", attempt + 1);
            }
        }
    }

    None
}

/// Send Ctrl+Alt+C keystroke to PoE via enigo.
pub(crate) fn send_copy_keystroke() -> Result<(), String> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo.key(Key::Control, Direction::Press).map_err(|e| e.to_string())?;
    enigo.key(Key::Alt, Direction::Press).map_err(|e| e.to_string())?;
    enigo.key(Key::Unicode('c'), Direction::Click).map_err(|e| e.to_string())?;
    enigo.key(Key::Alt, Direction::Release).map_err(|e| e.to_string())?;
    enigo.key(Key::Control, Direction::Release).map_err(|e| e.to_string())?;
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
        (point.x, point.y)
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


/// Dismiss the overlay: hide window, notify frontend.
#[tauri::command]
fn dismiss_overlay(window: tauri::WebviewWindow) {
    let _ = window.hide();
    let _ = window.emit("overlay-dismissed", ());
}

/// Show the overlay window with mock data for testing (tray debug button).
fn show_debug_overlay(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("overlay") {
        let (cursor_x, cursor_y) = get_cursor_position();
        let (css_x, css_y) = setup_fullscreen_overlay(&window, cursor_x, cursor_y);
        let _ = app.emit("overlay-position", OverlayPosition { x: css_x, y: css_y });
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

/// Set the active evaluation profile from the frontend.
/// Accepts a JSON string of poe-eval's Profile format.
/// Empty string = use the built-in default profile.
#[tauri::command]
fn set_active_profile(profile_json: String, state: tauri::State<'_, ProfileState>) {
    let profile = if profile_json.is_empty() {
        default_profile()
    } else {
        match serde_json::from_str::<Profile>(&profile_json) {
            Ok(p) => Some(p),
            Err(e) => {
                eprintln!("Failed to parse profile from frontend: {e}");
                default_profile()
            }
        }
    };
    *state.0.lock().unwrap() = profile;
}

/// Return the built-in default profile so the frontend can display or customize it.
#[tauri::command]
fn get_default_profile() -> Option<String> {
    default_profile().map(|p| serde_json::to_string(&p).unwrap_or_default())
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
        "stat_ids" => {
            let mut ids: Vec<String> = gd.stats.iter().map(|s| s.id.clone()).collect();
            ids.sort();
            ids
        }
        _ => vec![],
    }
}

/// Resolve a stat template key (e.g. "+# to maximum Life") to its internal stat IDs.
#[tauri::command]
fn resolve_stat_template(template: &str, gd: tauri::State<'_, GameDataState>) -> Vec<String> {
    let gd = &gd.0;
    gd.reverse_index
        .as_ref()
        .and_then(|ri| ri.stat_ids_for_template(template))
        .unwrap_or_default()
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
                // Don't save/restore visibility — we control that in setup()
                // based on the "start minimized" setting
                .with_state_flags(StateFlags::all().difference(StateFlags::VISIBLE))
                .build(),
        )
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(HotkeyState(std::sync::Mutex::new(HotkeyConfig::default())))
        .manage(GameDataState(Arc::new(load_game_data())))
        .manage(ProfileState(Mutex::new(default_profile())))
        .invoke_handler(tauri::generate_handler![
            dismiss_overlay,
            update_hotkeys,
            pause_hotkeys,
            resume_hotkeys,
            get_autostart,
            set_autostart,
            evaluate_item,
            set_active_profile,
            get_default_profile,
            get_predicate_schema,
            get_suggestions,
            resolve_stat_template,
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

                if wayland_active {
                    if let Some(watcher) = clipboard::ClipboardWatcher::start() {
                        eprintln!("[clipboard-watcher] Started successfully");
                        app.manage(watcher);
                    }
                }
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
                    let _ = window.app_handle().save_window_state(StateFlags::all().difference(StateFlags::VISIBLE));
                    api.prevent_close();
                    let _ = window.hide();
                }
            }

        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
