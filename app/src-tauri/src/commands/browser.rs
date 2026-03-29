use poe_data::browser::{BaseTypeDetail, ModPoolQuery, ModPoolResult, SearchResult};
use tauri::Manager;

use crate::game_data::GameDataState;

/// Search for base types, currency, gems, etc. by name.
#[tauri::command]
pub(crate) fn browser_search(
    query: String,
    limit: Option<usize>,
    state: tauri::State<'_, GameDataState>,
) -> Vec<SearchResult> {
    state.0.browser_search(&query, limit.unwrap_or(30))
}

/// Get full detail for a base item type.
#[tauri::command]
pub(crate) fn browser_base_type_detail(
    name: String,
    state: tauri::State<'_, GameDataState>,
) -> Option<BaseTypeDetail> {
    state.0.browser_base_type_detail(&name)
}

/// Compute the available mod pool for a base type + item level + constraints.
#[tauri::command]
pub(crate) fn browser_mod_pool(
    query: ModPoolQuery,
    state: tauri::State<'_, GameDataState>,
) -> Option<ModPoolResult> {
    state.0.browser_mod_pool(&query)
}

/// Show the browser window (pre-created at startup, hidden by default).
#[tauri::command]
pub(crate) fn open_browser_window(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("browser")
        .ok_or("browser window not found")?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}
