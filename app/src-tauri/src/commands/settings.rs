use std::sync::atomic::Ordering;

use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;

use crate::{PoeFocusGate, StashScrollState};
use crate::stash_scroll;

/// Get the current autostart state.
#[tauri::command]
pub(crate) fn get_autostart(app: tauri::AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

/// Enable or disable launch on boot.
#[tauri::command]
pub(crate) fn set_autostart(app: tauri::AppHandle, enabled: bool) {
    let launcher = app.autolaunch();
    if enabled {
        let _ = launcher.enable();
    } else {
        let _ = launcher.disable();
    }
}

/// Update the PoE focus gate from frontend settings.
/// Applies to both keyboard hotkeys and stash scroll.
#[tauri::command]
pub(crate) fn set_require_poe_focus(app: tauri::AppHandle, enabled: bool) {
    app.state::<PoeFocusGate>()
        .0
        .store(enabled, Ordering::Relaxed);
    app.state::<StashScrollState>()
        .0
        .set_require_poe_focus(enabled);
    #[cfg(target_os = "windows")]
    if let Some(hook) = app.try_state::<crate::HotkeyHookState>() {
        hook.0.set_require_poe_focus(enabled);
    }
}

/// Enable or disable stash tab scrolling.
#[tauri::command]
pub(crate) fn set_stash_scroll(app: tauri::AppHandle, enabled: bool) {
    app.state::<StashScrollState>().0.set_enabled(enabled);
}

/// Set the modifier key for stash tab scrolling.
#[tauri::command]
pub(crate) fn set_stash_scroll_modifier(app: tauri::AppHandle, modifier: String) {
    app.state::<StashScrollState>()
        .0
        .set_modifier(stash_scroll::ScrollModifier::from_str(&modifier));
}

/// Update the tray menu with the given profile data (bypasses store read).
/// Called from the frontend after profile changes to avoid store race conditions.
/// Also emits `profiles-updated` so other windows (e.g. settings) can refresh.
#[tauri::command]
pub(crate) fn update_tray_profiles(app: tauri::AppHandle, profiles_json: String) {
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

    let menu = crate::build_tray_menu_with_profiles(&app, &profiles);
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
