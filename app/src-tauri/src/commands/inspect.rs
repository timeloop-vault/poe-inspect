use tauri::{Emitter, Manager};

use crate::clipboard_acquire::acquire_clipboard;
use crate::game_data::GameDataState;
use crate::windows::{get_cursor_position, setup_fullscreen_overlay, OverlayPosition};
use crate::ProfileState;

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

/// Send Ctrl+Alt+C to PoE and read the resulting clipboard text.
pub(crate) fn handle_inspect(app: &tauri::AppHandle) {
    handle_inspect_with_mode(app, "inspect");
}

pub(crate) fn handle_inspect_with_mode(app: &tauri::AppHandle, mode: &str) {
    let app = app.clone();
    let mode = mode.to_string();
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
            // Invalidate before show — forces web process to discard stale tiles
            #[cfg(target_os = "linux")]
            crate::windows::invalidate_webview_buffer(&window);
            let _ = window.show();

            if mode == "compact" {
                // Compact: click-through, no focus steal
                let _ = window.set_ignore_cursor_events(true);
            } else {
                // Full: interactive overlay with focus
                let _ = window.set_ignore_cursor_events(false);
                let _ = window.set_focus();
            }
        }

        // Emit inspect mode before item data
        let _ = app.emit("inspect-mode", &mode);

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

/// Dismiss the overlay: hide window, notify frontend.
///
/// Also invalidates the WebView buffer so the web process has time to create
/// fresh tiles before the next show (the show-time invalidation handles the
/// GDK buffer clear).
#[tauri::command]
pub(crate) fn dismiss_overlay(window: tauri::WebviewWindow) {
    let _ = window.set_ignore_cursor_events(false);
    let _ = window.hide();
    let _ = window.emit("overlay-dismissed", ());
    #[cfg(target_os = "linux")]
    crate::windows::invalidate_webview_buffer(&window);
}
