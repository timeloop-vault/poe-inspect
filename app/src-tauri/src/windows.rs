use std::sync::atomic::Ordering;
use tauri::{Emitter, Manager};
use tauri_plugin_store::StoreExt;

use crate::ToastCounter;

/// Get the current cursor position (screen coordinates).
#[cfg(target_os = "windows")]
pub(crate) fn get_cursor_position() -> (i32, i32) {
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
pub(crate) fn get_cursor_position() -> (i32, i32) {
    // TODO: Use Core Graphics CGEventGetLocation or NSEvent.mouseLocation
    // See: https://developer.apple.com/documentation/coregraphics/1456611-cgeventgetlocation
    (100, 100)
}

#[cfg(target_os = "linux")]
pub(crate) fn get_cursor_position() -> (i32, i32) {
    if let Some(pos) = crate::wayland::get_cursor_position_hyprland() {
        return pos;
    }
    // Fallback for X11 / non-Hyprland compositors
    (100, 100)
}

/// Cursor position in CSS pixels, emitted to the frontend for panel positioning.
#[derive(serde::Serialize, Clone)]
pub(crate) struct OverlayPosition {
    pub x: f64,
    pub y: f64,
}

/// Expand the overlay window to fill the monitor containing the cursor.
/// Returns the cursor position in CSS pixels relative to the window.
pub(crate) fn setup_fullscreen_overlay(
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
        .try_state::<crate::wayland::WaylandOverlayState>()
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

/// Force WebKitGTK to reallocate its rendering buffers.
///
/// WebKitGTK doesn't clear its backing buffer for transparent windows between
/// hide/show cycles — old content accumulates. Two things must happen:
///
/// 1. **Tile invalidation** — `size_allocate(1×1)` then restore triggers
///    `webkitWebViewBaseSizeAllocate` → `drawingArea().setSize()`, which tells
///    the web process to discard cached tiles and repaint.
///
/// 2. **Buffer clear** — `queue_draw()` on the toplevel schedules a draw signal,
///    which triggers tao's `connect_draw` handler to clear the GDK window backing
///    buffer to transparent before WebKitGTK composites fresh tiles into it.
///    On non-layer-shell, the size_allocate alone suffices (GDK creates a new
///    buffer). On layer-shell, the buffer size is fixed by the compositor so GDK
///    reuses the same buffer — the explicit queue_draw is needed to clear it.
#[cfg(target_os = "linux")]
pub(crate) fn invalidate_webview_buffer(window: &tauri::WebviewWindow) {
    let _ = window.with_webview(|wv| {
        use gtk::prelude::WidgetExt;
        let webview = wv.inner();
        let alloc = webview.allocation();
        eprintln!(
            "[webview] Invalidating buffer (current {}x{})",
            alloc.width(),
            alloc.height()
        );
        // Shrink to 1x1 — discards existing rendering target in web process
        webview.size_allocate(&gdk::Rectangle::new(alloc.x(), alloc.y(), 1, 1));
        // Restore original size — web process allocates fresh tiles
        webview.size_allocate(&gdk::Rectangle::new(
            alloc.x(),
            alloc.y(),
            alloc.width(),
            alloc.height(),
        ));
        // Force a full redraw cycle — tao's connect_draw clears the GDK
        // window backing buffer to transparent before WebKitGTK paints.
        if let Some(toplevel) = webview.toplevel() {
            toplevel.queue_draw();
        }
    });
}

/// Show the overlay window with mock data for testing (tray debug button).
pub(crate) fn show_debug_overlay(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("overlay") {
        let (cursor_x, cursor_y) = get_cursor_position();
        let (css_x, css_y) = setup_fullscreen_overlay(&window, cursor_x, cursor_y);
        let _ = app.emit("overlay-position", OverlayPosition { x: css_x, y: css_y });
        let _ = window.show();
        let _ = window.set_ignore_cursor_events(false);
        let _ = window.set_focus();
        #[cfg(target_os = "linux")]
        invalidate_webview_buffer(&window);
    }
    let _ = app.emit("show-debug-overlay", ());
}

/// Show the settings window (create if needed, or just show+focus).
pub(crate) fn show_settings(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Show a toast notification in the small always-on-top toast window.
/// The toast window is pre-created at startup and reused.
#[tauri::command]
pub(crate) fn show_toast(app: tauri::AppHandle, profile_name: String, color: String) {
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
    let toast_w = (280.0_f64 * zoom).round();
    let toast_h = (46.0_f64 * zoom).round();
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
    let _ = window.set_ignore_cursor_events(true);
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
