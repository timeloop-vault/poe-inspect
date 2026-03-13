#[cfg(target_os = "linux")]
mod clipboard;
#[cfg(target_os = "linux")]
mod wayland;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use poe_data::GameData;
use poe_eval::{Profile, WatchingProfileInput};
use poe_trade::{LeagueList, TradeClient, TradeQueryConfig, TradeStatsIndex, TradeStatsResponse};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
#[cfg(not(target_os = "linux"))]
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_store::StoreExt;
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

/// Shared game data, loaded once at startup.
struct GameDataState(Arc<GameData>);

/// Primary + watching profiles, synced from the frontend.
#[derive(Clone)]
struct ProfileSet {
    primary: Option<Profile>,
    watching: Vec<WatchingProfileInput>,
}

/// Active evaluation profiles, loaded from JSON data files.
struct ProfileState(Mutex<ProfileSet>);

/// Monotonic counter for toast deduplication — prevents stale hide timers.
struct ToastCounter(AtomicU64);

/// Trade API client + stats index, managed as Tauri state.
///
/// Uses `tokio::sync::Mutex` because `TradeClient` methods take `&mut self`
/// and are async (held across `.await` points). The `RwLock` lets multiple
/// commands read the index concurrently while only `refresh` writes it.
struct TradeState {
    client: tokio::sync::Mutex<TradeClient>,
    index: tokio::sync::RwLock<Option<TradeStatsIndex>>,
}

/// Combined frontend payload: item display data + evaluation results.
/// Owned by the app — this is orchestration, not domain logic.
#[derive(Debug, serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ItemPayload {
    /// The parsed and resolved item (display data from poe-item).
    pub item: poe_item::types::ResolvedItem,
    /// Evaluation results (tier quality, affix analysis, scores from poe-eval).
    pub eval: poe_eval::ItemEvaluation,
    /// Raw clipboard text (needed for trade commands).
    pub raw_text: String,
}

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
        let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(
            mon_pos.x, mon_pos.y,
        )));
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
            let _ = window.set_ignore_cursor_events(false);
            let _ = window.show();
            let _ = window.set_focus();
        }

        // Try to parse and evaluate the item
        let gd = &app.state::<GameDataState>().0;
        let profiles = app.state::<ProfileState>().0.lock().unwrap().clone();
        match poe_item::parse(&clipboard_text) {
            Ok(raw) => {
                let resolved = poe_item::resolve(&raw, gd);
                let evaluation = poe_eval::evaluate_item(
                    &resolved,
                    gd,
                    profiles.primary.as_ref(),
                    &profiles.watching,
                );
                let payload = ItemPayload {
                    item: resolved,
                    eval: evaluation,
                    raw_text: clipboard_text.clone(),
                };
                let _ = app.emit("item-evaluated", &payload);
            }
            Err(e) => {
                eprintln!("Item parse failed: {e}");
                // Show parse-error overlay so the user can dismiss and report
                let _ = app.emit(
                    "item-parse-failed",
                    serde_json::json!({
                        "error": e.to_string(),
                        "rawText": clipboard_text,
                    }),
                );
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
    let _ = window.set_ignore_cursor_events(false);
    let _ = window.hide();
    let _ = window.emit("overlay-dismissed", ());
}

/// Show a toast notification in the small always-on-top toast window.
/// The toast window is pre-created at startup and reused.
#[tauri::command]
fn show_toast(app: tauri::AppHandle, profile_name: String, color: String) {
    let Some(window) = app.get_webview_window("toast") else {
        eprintln!("[toast] Toast window not found");
        return;
    };

    // Read overlay scale from settings store for toast sizing
    let scale_pct = app
        .store("settings.json")
        .ok()
        .and_then(|s| s.get("general"))
        .and_then(|v| v.get("overlayScale")?.as_f64())
        .unwrap_or(100.0);
    let zoom = scale_pct / 100.0;
    let toast_w = (280.0 * zoom).round();
    let toast_h = (46.0 * zoom).round();
    let _ = window.set_size(tauri::LogicalSize::new(toast_w, toast_h));

    // Position at top-center of the monitor where the cursor is
    let (cursor_x, cursor_y) = get_cursor_position();
    let monitors = window.available_monitors().unwrap_or_default();
    let monitor = monitors.iter().find(|m| {
        let pos = m.position();
        let size = m.size();
        cursor_x >= pos.x
            && cursor_x < pos.x + size.width as i32
            && cursor_y >= pos.y
            && cursor_y < pos.y + size.height as i32
    });

    if let Some(monitor) = monitor {
        let mon_pos = monitor.position();
        let mon_size = monitor.size();
        let mon_scale = monitor.scale_factor();
        let phys_w = (toast_w * mon_scale) as i32;
        let center_x = mon_pos.x + (mon_size.width as i32 - phys_w) / 2;
        let top_y = mon_pos.y + (40.0 * mon_scale) as i32;
        let _ = window.set_position(tauri::PhysicalPosition::new(center_x, top_y));
    }

    let _ = window.emit(
        "show-toast",
        serde_json::json!({ "profileName": profile_name, "color": color, "zoom": zoom }),
    );
    let _ = window.show();

    // Bump toast counter — only the latest toast's timer will hide the window
    let generation = app
        .state::<ToastCounter>()
        .0
        .fetch_add(1, Ordering::Relaxed)
        + 1;

    let app_clone = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(2300));
        let current = app_clone.state::<ToastCounter>().0.load(Ordering::Relaxed);
        if current == generation {
            if let Some(w) = app_clone.get_webview_window("toast") {
                let _ = w.hide();
            }
        }
    });
}

/// Update the tray menu with the given profile data (bypasses store read).
/// Called from the frontend after profile changes to avoid store race conditions.
/// Also emits `profiles-updated` so other windows (e.g. settings) can refresh.
#[tauri::command]
fn update_tray_profiles(app: tauri::AppHandle, profiles_json: String) {
    let profiles: Vec<(String, String, String)> =
        serde_json::from_str::<Vec<serde_json::Value>>(&profiles_json)
            .unwrap_or_default()
            .iter()
            .filter_map(|p| {
                let id = p.get("id")?.as_str()?.to_string();
                let name = p.get("name")?.as_str()?.to_string();
                let role = p.get("role")?.as_str().unwrap_or("off").to_string();
                Some((id, name, role))
            })
            .collect();

    let menu = build_tray_menu_with_profiles(&app, &profiles);
    match menu {
        Ok(menu) => {
            if let Some(tray) = app.tray_by_id("main-tray") {
                let _ = tray.set_menu(Some(menu));
            }
        }
        Err(e) => eprintln!("[tray] Failed to build menu: {e}"),
    }

    // Notify all windows so settings can refresh its profile list
    let _ = app.emit("profiles-updated", ());
}

/// Show the overlay window with mock data for testing (tray debug button).
fn show_debug_overlay(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("overlay") {
        let (cursor_x, cursor_y) = get_cursor_position();
        let (css_x, css_y) = setup_fullscreen_overlay(&window, cursor_x, cursor_y);
        let _ = app.emit("overlay-position", OverlayPosition { x: css_x, y: css_y });
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
    cycle_profile: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            inspect_item: "ctrl+i".into(),
            open_settings: "ctrl+shift+i".into(),
            cycle_profile: "ctrl+shift+p".into(),
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
        cycle_profile: hotkeys_val
            .get("cycleProfile")
            .and_then(|v| v.as_str())
            .map(|s| s.to_lowercase())
            .unwrap_or(defaults.cycle_profile),
    }
}

/// Register global shortcuts from config. Only inspect + settings are global.
fn register_hotkeys(app: &tauri::AppHandle, config: &HotkeyConfig) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    let entries: [(&str, &str); 3] = [
        (&config.inspect_item, "inspect"),
        (&config.open_settings, "settings"),
        (&config.cycle_profile, "cycle_profile"),
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
                "cycle_profile" => {
                    let _ = app.emit("cycle-profile", ());
                }
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
    cycle_profile: String,
) {
    let config = HotkeyConfig {
        inspect_item,
        open_settings,
        cycle_profile,
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
/// Returns item display data + evaluation results, or an error string.
#[tauri::command]
fn evaluate_item(
    item_text: String,
    state: tauri::State<'_, GameDataState>,
) -> Result<ItemPayload, String> {
    let gd = &state.0;

    // Pass 1: structural parse
    let raw = poe_item::parse(&item_text).map_err(|e| format!("Parse error: {e}"))?;

    // Pass 2: resolve against game data
    let resolved = poe_item::resolve(&raw, gd);

    // Evaluate (no profile for direct command calls)
    let evaluation = poe_eval::evaluate_item(&resolved, gd, None, &[]);
    Ok(ItemPayload {
        item: resolved,
        eval: evaluation,
        raw_text: item_text,
    })
}

/// Set primary + watching profiles from the frontend.
/// primaryJson: poe-eval Profile JSON (empty = built-in default).
/// watchingJson: JSON array of {name, color, profile} objects.
#[tauri::command]
fn set_active_profile(
    primary_json: String,
    watching_json: String,
    state: tauri::State<'_, ProfileState>,
) {
    // "none" = no primary (show overlay without scoring)
    // "" = use built-in default profile
    // JSON = custom profile
    let primary = if primary_json == "none" {
        None
    } else if primary_json.is_empty() {
        default_profile()
    } else {
        match serde_json::from_str::<Profile>(&primary_json) {
            Ok(p) => Some(p),
            Err(e) => {
                eprintln!("Failed to parse primary profile: {e}");
                default_profile()
            }
        }
    };

    let watching: Vec<WatchingProfileInput> =
        serde_json::from_str(&watching_json).unwrap_or_default();

    eprintln!(
        "[profiles] Primary: {}, Watching: {}",
        if primary.is_some() { "set" } else { "none" },
        watching.len()
    );

    let mut ps = state.0.lock().unwrap();
    ps.primary = primary;
    ps.watching = watching;
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

/// A map mod template with its stat IDs (for the map danger settings page).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MapModTemplate {
    template: String,
    stat_ids: Vec<String>,
}

/// Return all map/area mod templates for the map danger settings page.
#[tauri::command]
fn get_map_mod_templates(state: tauri::State<'_, GameDataState>) -> Vec<MapModTemplate> {
    let mut templates: Vec<MapModTemplate> = state
        .0
        .map_mod_templates()
        .into_iter()
        .filter(|(template, _)| template.chars().any(|c| c.is_alphabetic()))
        .map(|(template, stat_ids)| MapModTemplate {
            template: template.to_string(),
            stat_ids: stat_ids.to_vec(),
        })
        .collect();
    templates.sort_by(|a, b| a.template.cmp(&b.template));
    templates
}

/// Return enriched stat suggestions matching a text query.
///
/// Returns both single-stat suggestions and hybrid mod combos that include
/// the matching stat. Used by the stat picker to show hybrid options.
#[tauri::command]
fn get_stat_suggestions(
    query: String,
    state: tauri::State<'_, GameDataState>,
) -> Vec<poe_data::StatSuggestion> {
    state.0.stat_suggestions_for_query(&query)
}

// ── Trade commands (async) ──────────────────────────────────────────────────

/// Preview a trade query without executing it (no HTTP, no rate limit cost).
///
/// Returns the full `QueryBuildResult` including `mapped_stats` so the
/// frontend can populate the "Edit Search" UI with checkboxes and value inputs.
#[tauri::command]
async fn preview_trade_query(
    item_text: String,
    config: TradeQueryConfig,
    filter_config: Option<poe_trade::TradeFilterConfig>,
    gd: tauri::State<'_, GameDataState>,
    trade: tauri::State<'_, TradeState>,
) -> Result<poe_trade::QueryBuildResult, String> {
    let gd = &gd.0;

    let raw = poe_item::parse(&item_text).map_err(|e| format!("Parse error: {e}"))?;
    let resolved = poe_item::resolve(&raw, gd);

    let index_guard = trade.index.read().await;
    let index = index_guard
        .as_ref()
        .ok_or("Trade stats index not loaded — call refresh_trade_stats first")?;

    Ok(poe_trade::build_query(
        &resolved,
        index,
        &config,
        filter_config.as_ref(),
    ))
}

/// Full price check: parse item → build query → search → fetch prices.
///
/// Returns prices from the cheapest listings, or an error string.
#[tauri::command]
async fn price_check(
    item_text: String,
    config: TradeQueryConfig,
    filter_config: Option<poe_trade::TradeFilterConfig>,
    gd: tauri::State<'_, GameDataState>,
    trade: tauri::State<'_, TradeState>,
) -> Result<poe_trade::PriceCheckResult, String> {
    let gd = &gd.0;

    let raw = poe_item::parse(&item_text).map_err(|e| format!("Parse error: {e}"))?;
    let resolved = poe_item::resolve(&raw, gd);

    let index_guard = trade.index.read().await;
    let index = index_guard
        .as_ref()
        .ok_or("Trade stats index not loaded — call refresh_trade_stats first")?;

    let query_result = poe_trade::build_query(&resolved, index, &config, filter_config.as_ref());
    // Release the read lock before acquiring the client mutex.
    drop(index_guard);

    let mut client = trade.client.lock().await;
    client
        .price_check(&query_result.body, &config)
        .await
        .map_err(|e| e.to_string())
}

/// Build a trade query and return the trade site URL (no fetch).
///
/// Useful for "open on trade site" without waiting for price results.
#[tauri::command]
async fn trade_search_url(
    item_text: String,
    config: TradeQueryConfig,
    filter_config: Option<poe_trade::TradeFilterConfig>,
    gd: tauri::State<'_, GameDataState>,
    trade: tauri::State<'_, TradeState>,
) -> Result<String, String> {
    let gd = &gd.0;

    let raw = poe_item::parse(&item_text).map_err(|e| format!("Parse error: {e}"))?;
    let resolved = poe_item::resolve(&raw, gd);

    let index_guard = trade.index.read().await;
    let index = index_guard
        .as_ref()
        .ok_or("Trade stats index not loaded — call refresh_trade_stats first")?;

    let query_result = poe_trade::build_query(&resolved, index, &config, filter_config.as_ref());
    drop(index_guard);

    let mut client = trade.client.lock().await;
    let search = client
        .search(&query_result.body, &config.league)
        .await
        .map_err(|e| e.to_string())?;

    Ok(poe_trade::query::trade_url(&config.league, &search.id))
}

/// Fetch live trade stats from the API, rebuild the index, and cache to disk.
///
/// Returns the number of stats mapped (GGPK → trade).
#[tauri::command]
async fn refresh_trade_stats(
    app: tauri::AppHandle,
    gd: tauri::State<'_, GameDataState>,
    trade: tauri::State<'_, TradeState>,
) -> Result<u32, String> {
    let client = trade.client.lock().await;
    let response = client.fetch_stats().await.map_err(|e| e.to_string())?;
    drop(client);

    // Cache raw response to disk.
    if let Some(cache_path) = trade_stats_cache_path(&app) {
        if let Ok(json) = serde_json::to_string(&response) {
            let _ = std::fs::create_dir_all(cache_path.parent().unwrap());
            if let Err(e) = std::fs::write(&cache_path, json) {
                eprintln!("Failed to cache trade stats: {e}");
            }
        }
    }

    let result = TradeStatsIndex::from_response(&response, &gd.0);
    let matched = result.matched;

    eprintln!(
        "[trade] Refreshed index: {}/{} matched",
        result.matched,
        result.matched + result.unmatched,
    );

    *trade.index.write().await = Some(result.index);
    Ok(matched)
}

/// Open a URL in the user's default browser.
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| format!("Failed to open URL: {e}"))
}

/// Set the POESESSID cookie on the trade client.
///
/// Enables "online only" filtering. Pass empty string to clear.
#[tauri::command]
async fn set_trade_session(
    poesessid: String,
    trade: tauri::State<'_, TradeState>,
) -> Result<(), String> {
    let mut client = trade.client.lock().await;
    if poesessid.is_empty() {
        client.set_session_id(None);
    } else {
        client.set_session_id(Some(poesessid));
    }
    Ok(())
}

/// Get the current trade stats index status.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TradeIndexStatus {
    loaded: bool,
    stat_count: usize,
    mapped_count: usize,
}

#[tauri::command]
async fn get_trade_index_status(
    trade: tauri::State<'_, TradeState>,
) -> Result<TradeIndexStatus, String> {
    let guard = trade.index.read().await;
    match guard.as_ref() {
        Some(index) => Ok(TradeIndexStatus {
            loaded: true,
            stat_count: index.len(),
            mapped_count: index.mapped_stat_count(),
        }),
        None => Ok(TradeIndexStatus {
            loaded: false,
            stat_count: 0,
            mapped_count: 0,
        }),
    }
}

/// Fetch the list of active leagues from GGG.
#[tauri::command]
async fn fetch_leagues(
    trade: tauri::State<'_, TradeState>,
) -> Result<LeagueList, String> {
    let client = trade.client.lock().await;
    client.fetch_leagues().await.map_err(|e| e.to_string())
}

/// Return the path for the cached trade stats JSON.
fn trade_stats_cache_path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|dir| dir.join("trade_stats.json"))
}

/// Try to load the trade stats index from disk cache.
fn load_cached_trade_index(app: &tauri::AppHandle, gd: &GameData) -> Option<TradeStatsIndex> {
    let path = trade_stats_cache_path(app)?;
    let data = std::fs::read_to_string(&path).ok()?;
    let response: TradeStatsResponse = serde_json::from_str(&data).ok()?;
    let result = TradeStatsIndex::from_response(&response, gd);
    eprintln!(
        "[trade] Loaded cached index: {}/{} matched",
        result.matched,
        result.matched + result.unmatched,
    );
    Some(result.index)
}

/// Load game data from extracted datc64 files.
///
/// Looks for data in these locations (first match wins):
/// 1. `POE_DATA_DIR` environment variable
/// 2. `data/` directory next to the executable (Windows/Linux release)
/// 3. `../Resources/data/` relative to executable (macOS .app bundle)
/// 4. Repo path via `CARGO_MANIFEST_DIR` (dev builds)
/// 5. `%TEMP%/poe-dat/` (dev fallback — same dir used by poe-data tests)
///
/// Returns empty GameData if no data directory is found (overlay still works,
/// just without stat resolution or open affix detection).
fn load_game_data() -> GameData {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));

    let candidates = [
        std::env::var("POE_DATA_DIR")
            .ok()
            .map(std::path::PathBuf::from),
        // Windows/Linux: data/ next to executable
        exe_dir.as_ref().map(|d| d.join("data")),
        // macOS .app bundle: Contents/MacOS/../Resources/data/
        exe_dir.as_ref().map(|d| d.join("../Resources/data")),
        // Dev: committed game data in the repo
        Some(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/poe-data/data"),
        ),
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
    GameData::new(
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    )
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

/// Build the system tray menu with the given profile data.
fn build_tray_menu_with_profiles(
    app: &tauri::AppHandle,
    profiles: &[(String, String, String)],
) -> tauri::Result<Menu<tauri::Wry>> {
    #[cfg(debug_assertions)]
    let show_overlay = MenuItem::with_id(
        app,
        "show_overlay",
        "Show Overlay (Debug)",
        true,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    if profiles.len() > 1 {
        let sep1 = PredefinedMenuItem::separator(app)?;
        let sep2 = PredefinedMenuItem::separator(app)?;

        let profile_items: Vec<MenuItem<tauri::Wry>> = profiles
            .iter()
            .map(|(id, name, role)| {
                let prefix = if role == "primary" { "● " } else { "   " };
                MenuItem::with_id(
                    app,
                    format!("profile:{id}"),
                    format!("{prefix}{name}"),
                    true,
                    None::<&str>,
                )
            })
            .collect::<Result<_, _>>()?;

        let mut items: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = Vec::new();
        #[cfg(debug_assertions)]
        items.push(&show_overlay);
        items.push(&settings);
        items.push(&sep1);
        for item in &profile_items {
            items.push(item);
        }
        items.push(&sep2);
        items.push(&quit);

        Menu::with_items(app, &items)
    } else {
        #[cfg(debug_assertions)]
        return Menu::with_items(app, &[&show_overlay, &settings, &quit]);
        #[cfg(not(debug_assertions))]
        Menu::with_items(app, &[&settings, &quit])
    }
}

/// Build the system tray menu by reading profiles from the store.
fn build_tray_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let profiles: Vec<(String, String, String)> = (|| {
        let store = app.store("profiles.json").ok()?;
        let val = store.get("profiles")?;
        let arr = val.as_array()?;
        Some(
            arr.iter()
                .filter_map(|p| {
                    let id = p.get("id")?.as_str()?.to_string();
                    let name = p.get("name")?.as_str()?.to_string();
                    let role = p.get("role")?.as_str().unwrap_or("off").to_string();
                    Some((id, name, role))
                })
                .collect(),
        )
    })()
    .unwrap_or_default();

    build_tray_menu_with_profiles(app, &profiles)
}

pub fn run() {
    // WebKitGTK's DMA-BUF renderer has a bug with explicit sync on Wayland
    // compositors (Hyprland): it creates a wp_linux_drm_syncobj_surface but
    // doesn't set the acquire timeline before committing, causing a fatal
    // "Missing acquire timeline" protocol error. Disable DMA-BUF rendering
    // to use shared-memory buffers instead. Must be set before GTK init.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var_os("XDG_SESSION_TYPE").is_some_and(|v| v == "wayland")
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_denylist(&["overlay", "toast"])
                // Don't save/restore visibility — we control that in setup()
                // based on the "start minimized" setting
                .with_state_flags(StateFlags::all().difference(StateFlags::VISIBLE))
                .build(),
        )
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::default().build());

    // MCP plugin for AI-agent debugging — dev builds only
    #[cfg(debug_assertions)]
    let builder = builder.plugin(tauri_plugin_mcp::init_with_config(
        tauri_plugin_mcp::PluginConfig::new("poe-inspect".to_string())
            .start_socket_server(true)
            .tcp_localhost(4000),
    ));

    builder
        .manage(HotkeyState(std::sync::Mutex::new(HotkeyConfig::default())))
        .manage(GameDataState(Arc::new(load_game_data())))
        .manage(ProfileState(Mutex::new(ProfileSet {
            primary: default_profile(),
            watching: vec![],
        })))
        .manage(ToastCounter(AtomicU64::new(0)))
        .invoke_handler(tauri::generate_handler![
            dismiss_overlay,
            show_toast,
            update_tray_profiles,
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
            get_stat_suggestions,
            get_map_mod_templates,
            preview_trade_query,
            price_check,
            trade_search_url,
            refresh_trade_stats,
            fetch_leagues,
            open_url,
            set_trade_session,
            get_trade_index_status,
        ])
        .setup(|app| {
            // --- System tray ---
            let menu = build_tray_menu(app.handle())?;

            TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("PoE Inspect")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    let id = event.id.as_ref();
                    if id == "show_overlay" {
                        show_debug_overlay(app);
                    } else if id == "settings" {
                        show_settings(app);
                    } else if id == "quit" {
                        app.exit(0);
                    } else if let Some(profile_id) = id.strip_prefix("profile:") {
                        let _ = app.emit("switch-profile", profile_id.to_string());
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                        show_settings(tray.app_handle());
                    }
                })
                .build(app)?;

            // --- Toast notification window (hidden, click-through) ---
            {
                let url = tauri::WebviewUrl::App("index.html".into());
                let toast_win =
                    tauri::WebviewWindowBuilder::new(app, "toast", url)
                        .title("Toast")
                        .decorations(false)
                        .transparent(true)
                        .always_on_top(true)
                        .skip_taskbar(true)
                        .resizable(false)
                        .focused(false)
                        .visible(false)
                        .inner_size(300.0, 50.0)
                        .build()?;
                let _ = toast_win.set_ignore_cursor_events(true);
            }

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

            // --- Trade state (load cached index + POESESSID if available) ---
            {
                let gd = &app.state::<GameDataState>().0;
                let cached_index = load_cached_trade_index(app.handle(), gd);
                let mut client = TradeClient::new();

                // Restore POESESSID from settings store
                if let Some(sessid) = app
                    .store("settings.json")
                    .ok()
                    .and_then(|store| {
                        store
                            .get("trade")
                            .and_then(|v| v.get("poesessid").and_then(|v| v.as_str().map(String::from)))
                    })
                    .filter(|s| !s.is_empty())
                {
                    client.set_session_id(Some(sessid));
                    eprintln!("[trade] POESESSID loaded from settings");
                }

                app.manage(TradeState {
                    client: tokio::sync::Mutex::new(client),
                    index: tokio::sync::RwLock::new(cached_index),
                });
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
                    let _ = window
                        .app_handle()
                        .save_window_state(StateFlags::all().difference(StateFlags::VISIBLE));
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
