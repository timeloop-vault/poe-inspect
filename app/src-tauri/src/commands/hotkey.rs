#[cfg(not(target_os = "windows"))]
use std::sync::atomic::Ordering;

use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::ShortcutState;

use crate::commands::chat_macro::execute_chat_macro;
use crate::commands::inspect::{handle_inspect, handle_inspect_with_mode};
use crate::windows::show_settings;
#[cfg(not(target_os = "windows"))]
use crate::PoeFocusGate;
use crate::{ChatMacroConfig, ChatMacroState, HotkeyState};

/// Global shortcut config — only inspect and settings are registered as global
/// shortcuts. Dismiss is handled at the overlay window level to avoid consuming
/// keys like Escape system-wide.
#[derive(Debug, Clone)]
pub(crate) struct HotkeyConfig {
    pub inspect_item: String,
    pub compact_inspect: String,
    pub trade_inspect: String,
    pub trade_edit_inspect: String,
    pub open_settings: String,
    pub cycle_profile: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            inspect_item: "ctrl+i".into(),
            compact_inspect: "ctrl+shift+i".into(),
            trade_inspect: "ctrl+t".into(),
            trade_edit_inspect: "ctrl+shift+t".into(),
            open_settings: "ctrl+shift+s".into(),
            cycle_profile: "ctrl+shift+p".into(),
        }
    }
}

/// Load hotkey config from the tauri-plugin-store settings file.
/// Falls back to defaults if the store doesn't exist or keys are missing.
pub(crate) fn load_hotkey_config(app: &tauri::AppHandle) -> HotkeyConfig {
    use tauri_plugin_store::StoreExt;

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
        compact_inspect: hotkeys_val
            .get("compactInspect")
            .and_then(|v| v.as_str())
            .map(|s| s.to_lowercase())
            .unwrap_or(defaults.compact_inspect),
        trade_inspect: hotkeys_val
            .get("tradeInspect")
            .and_then(|v| v.as_str())
            .map(|s| s.to_lowercase())
            .unwrap_or(defaults.trade_inspect),
        trade_edit_inspect: hotkeys_val
            .get("tradeEditInspect")
            .and_then(|v| v.as_str())
            .map(|s| s.to_lowercase())
            .unwrap_or(defaults.trade_edit_inspect),
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

/// Load chat macros from the tauri-plugin-store settings file.
pub(crate) fn load_chat_macros(app: &tauri::AppHandle) -> Vec<ChatMacroConfig> {
    use tauri_plugin_store::StoreExt;

    let Ok(store) = app.store("settings.json") else {
        return vec![];
    };
    let Some(val) = store.get("chatMacros") else {
        return vec![];
    };
    serde_json::from_value(val).unwrap_or_default()
}

// ── PoE foreground detection ─────────────────────────────────────────────

/// Check whether a Path of Exile window is the current foreground window.
/// On Windows, only used by stash_scroll via its own `get_poe_foreground_hwnd`.
/// Gameplay hotkeys use the keyboard hook which checks focus internally.
#[cfg(target_os = "windows")]
#[allow(dead_code)]
fn is_poe_focused() -> bool {
    extern "system" {
        fn GetForegroundWindow() -> isize;
        fn GetWindowTextW(hwnd: isize, string: *mut u16, max_count: i32) -> i32;
    }

    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd == 0 {
        return false;
    }

    let mut buf = [0u16; 256];
    let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if len <= 0 {
        return false;
    }

    let title = String::from_utf16_lossy(&buf[..len as usize]);
    // PoE 1 uses "Path of Exile", PoE 2 uses "Path of Exile 2"
    title.starts_with("Path of Exile")
}

/// Foreground detection not implemented — always returns true.
#[cfg(not(target_os = "windows"))]
fn is_poe_focused() -> bool {
    true
}

/// Returns true if the hotkey should proceed (either PoE is focused,
/// or the focus gate is disabled in settings).
/// On Windows, gameplay hotkeys use the keyboard hook (checks focus internally).
#[cfg(not(target_os = "windows"))]
fn should_allow_hotkey(app: &tauri::AppHandle) -> bool {
    let gate = app.state::<PoeFocusGate>();
    if !gate.0.load(Ordering::Relaxed) {
        return true; // gate disabled in settings
    }
    is_poe_focused()
}

/// Dispatch a hotkey action by name. Shared between hook-based and
/// global-shortcut-based hotkey paths.
pub(crate) fn dispatch_hotkey_action(app: &tauri::AppHandle, action: &str) {
    match action {
        "inspect" => {
            let settings_focused = app
                .get_webview_window("settings")
                .and_then(|w| w.is_focused().ok())
                .unwrap_or(false);
            if !settings_focused {
                // If overlay is already visible (compact mode showing),
                // expand to full without re-parsing
                let overlay_visible = app
                    .get_webview_window("overlay")
                    .and_then(|w| w.is_visible().ok())
                    .unwrap_or(false);
                if overlay_visible {
                    let _ = app.emit("inspect-mode", "inspect");
                    if let Some(w) = app.get_webview_window("overlay") {
                        let _ = w.set_ignore_cursor_events(false);
                        let _ = w.set_focus();
                    }
                } else {
                    handle_inspect(app);
                }
            }
        }
        "compact_inspect" => {
            let settings_focused = app
                .get_webview_window("settings")
                .and_then(|w| w.is_focused().ok())
                .unwrap_or(false);
            if !settings_focused {
                handle_inspect_with_mode(app, "compact");
            }
        }
        "trade_inspect" => {
            let settings_focused = app
                .get_webview_window("settings")
                .and_then(|w| w.is_focused().ok())
                .unwrap_or(false);
            if !settings_focused {
                handle_inspect_with_mode(app, "trade");
            }
        }
        "trade_edit_inspect" => {
            let settings_focused = app
                .get_webview_window("settings")
                .and_then(|w| w.is_focused().ok())
                .unwrap_or(false);
            if !settings_focused {
                handle_inspect_with_mode(app, "tradeEdit");
            }
        }
        "settings" => show_settings(app),
        "cycle_profile" => {
            let _ = app.emit("cycle-profile", ());
        }
        other => {
            // Chat macro: action format is "macro:<command>:<send>"
            if let Some(rest) = other.strip_prefix("macro:") {
                if let Some((command, send_str)) = rest.rsplit_once(':') {
                    let send = send_str == "1";
                    execute_chat_macro(app, command, send);
                }
            }
        }
    }
}

/// Register all global shortcuts: core hotkeys + chat macros.
///
/// On Windows, gameplay hotkeys use `WH_KEYBOARD_LL` (only fires when PoE is
/// focused, passes keys through to other apps). The "settings" hotkey uses
/// `global_shortcut` since it should work regardless of focused app.
///
/// On non-Windows, all hotkeys use `global_shortcut` (passthrough not available).
pub(crate) fn register_hotkeys(
    app: &tauri::AppHandle,
    config: &HotkeyConfig,
    macros: &[ChatMacroConfig],
) {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    // Build the list of PoE-only gameplay hotkeys for the keyboard hook
    let mut hook_entries: Vec<(&str, String)> = vec![
        (&config.inspect_item, "inspect".into()),
        (&config.compact_inspect, "compact_inspect".into()),
        (&config.trade_inspect, "trade_inspect".into()),
        (&config.trade_edit_inspect, "trade_edit_inspect".into()),
        (&config.cycle_profile, "cycle_profile".into()),
    ];

    // Chat macros — encode command+send into the action string
    for m in macros {
        if m.hotkey.is_empty() || m.command.is_empty() {
            continue;
        }
        let action = format!("macro:{}:{}", m.command, if m.send { "1" } else { "0" });
        hook_entries.push((&m.hotkey, action));
    }

    // On Windows, register gameplay hotkeys via the keyboard hook
    #[cfg(target_os = "windows")]
    {
        if let Some(hook_state) = app.try_state::<crate::HotkeyHookState>() {
            let entries_ref: Vec<(&str, &str)> =
                hook_entries.iter().map(|(k, v)| (*k, v.as_str())).collect();
            hook_state.0.set_bindings(&entries_ref);
        }
    }

    // On non-Windows, register gameplay hotkeys via global_shortcut (fallback)
    #[cfg(not(target_os = "windows"))]
    {
        for (shortcut_str, action_name) in &hook_entries {
            let action = action_name.clone();
            if let Err(e) = gs.on_shortcut(*shortcut_str, move |app, _shortcut, event| {
                if event.state != ShortcutState::Pressed {
                    return;
                }
                if !should_allow_hotkey(app) {
                    return;
                }
                dispatch_hotkey_action(app, &action);
            }) {
                eprintln!("Failed to register shortcut '{shortcut_str}': {e}");
            }
        }
    }

    // "Settings" hotkey always uses global_shortcut (works in any app)
    {
        let shortcut = config.open_settings.clone();
        if let Err(e) = gs.on_shortcut(shortcut.as_str(), move |app, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            show_settings(app);
        }) {
            eprintln!("Failed to register settings shortcut: {e}");
        }
    }
}

/// Update global hotkeys from frontend when settings change.
/// dismiss_overlay is accepted but not registered globally — it's handled
/// at the overlay window level via a keydown listener.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub(crate) fn update_hotkeys(
    app: tauri::AppHandle,
    inspect_item: String,
    compact_inspect: String,
    trade_inspect: String,
    trade_edit_inspect: String,
    #[allow(unused_variables)] dismiss_overlay: String,
    open_settings: String,
    cycle_profile: String,
) {
    let config = HotkeyConfig {
        inspect_item,
        compact_inspect,
        trade_inspect,
        trade_edit_inspect,
        open_settings,
        cycle_profile,
    };
    let macros = app.state::<ChatMacroState>().0.lock().unwrap().clone();

    // Re-enable the keyboard hook — it may have been disabled by pause_hotkeys
    // during hotkey capture. register_hotkeys updates bindings but doesn't
    // re-enable the hook itself.
    #[cfg(target_os = "windows")]
    if let Some(hook) = app.try_state::<crate::HotkeyHookState>() {
        hook.0.set_enabled(true);
    }

    register_hotkeys(&app, &config, &macros);
    *app.state::<HotkeyState>().0.lock().unwrap() = config;
}

/// Unregister all global hotkeys (used during hotkey capture in settings).
#[tauri::command]
pub(crate) fn pause_hotkeys(app: tauri::AppHandle) {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    let _ = app.global_shortcut().unregister_all();
    #[cfg(target_os = "windows")]
    if let Some(hook) = app.try_state::<crate::HotkeyHookState>() {
        hook.0.set_enabled(false);
    }
}

/// Re-register global hotkeys from saved config (used after hotkey capture).
#[tauri::command]
pub(crate) fn resume_hotkeys(app: tauri::AppHandle) {
    #[cfg(target_os = "windows")]
    if let Some(hook) = app.try_state::<crate::HotkeyHookState>() {
        hook.0.set_enabled(true);
    }
    let config = app.state::<HotkeyState>().0.lock().unwrap().clone();
    let macros = app.state::<ChatMacroState>().0.lock().unwrap().clone();
    register_hotkeys(&app, &config, &macros);
}

/// Update chat macros from the frontend. Re-registers all shortcuts.
#[tauri::command]
pub(crate) fn update_chat_macros(app: tauri::AppHandle, macros_json: String) {
    let macros: Vec<ChatMacroConfig> = serde_json::from_str(&macros_json).unwrap_or_default();
    let config = app.state::<HotkeyState>().0.lock().unwrap().clone();
    register_hotkeys(&app, &config, &macros);
    *app.state::<ChatMacroState>().0.lock().unwrap() = macros;
}
