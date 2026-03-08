//! Wayland overlay support via wlr-layer-shell protocol.
//!
//! Uses `gtk-layer-shell` to configure the overlay as a layer surface,
//! providing proper always-on-top behavior, positioning via margins,
//! and no taskbar entry — all things that are no-ops with regular
//! Wayland windows.

use gdk::prelude::*;
use gtk::prelude::*;
use gtk_layer_shell::LayerShell;

/// Managed state indicating whether the layer-shell path is active.
pub struct WaylandOverlayState {
    pub active: bool,
}

/// Detect whether the given window is running on a Wayland backend.
pub fn detect_wayland(gtk_window: &gtk::ApplicationWindow) -> bool {
    let display = gtk_window.display();
    display.backend().is_wayland()
}

/// Initialize the overlay window as a layer-shell surface.
///
/// Wry unconditionally calls `show_all()` during WebView creation (as a
/// workaround for a WebKitGTK display bug), which realizes the window and
/// assigns it the xdg_toplevel role. If `init_layer_shell()` is called on
/// an already-realized window, it unrealizes+re-realizes, but this races
/// with pending xdg protocol messages and causes "Protocol error 71".
///
/// Fix: unrealize the window ourselves first. `init_layer_shell()` then
/// just installs its `realize` signal handler without touching the Wayland
/// connection. The window will be cleanly realized as a layer surface when
/// `show()` is called later (in `handle_inspect` / `show_debug_overlay`).
pub fn init_overlay_layer_shell(gtk_window: &gtk::ApplicationWindow) {
    if gtk_window.is_realized() {
        gtk_window.unrealize();
    }
    gtk_window.init_layer_shell();
    gtk_window.set_layer(gtk_layer_shell::Layer::Overlay);

    // Anchor to top-left — positioning is done via margins from these edges
    gtk_window.set_anchor(gtk_layer_shell::Edge::Top, true);
    gtk_window.set_anchor(gtk_layer_shell::Edge::Left, true);

    // Don't reserve screen space (not a panel/dock)
    gtk_window.set_exclusive_zone(-1);

    // Receive keyboard input when focused
    gtk_window.set_keyboard_mode(gtk_layer_shell::KeyboardMode::OnDemand);

    // Namespace for compositor rules (e.g., Hyprland `layerrule`)
    gtk_window.set_namespace("poe-inspect");
}

/// Position a layer-shell surface at absolute screen coordinates (x, y).
///
/// Layer surfaces are positioned via margins from their anchor edges.
/// Since we anchor Top+Left, we set left-margin = x and top-margin = y
/// relative to the target monitor.
pub fn position_layer_surface(gtk_window: &gtk::ApplicationWindow, x: i32, y: i32) {
    let display = match gdk::Display::default() {
        Some(d) => d,
        None => return,
    };

    let monitor = display.monitor_at_point(x, y);
    let monitor = match monitor {
        Some(m) => m,
        None => return,
    };

    gtk_window.set_monitor(&monitor);

    let geometry = monitor.geometry();
    let margin_left = x - geometry.x();
    let margin_top = y - geometry.y();

    gtk_window.set_layer_shell_margin(gtk_layer_shell::Edge::Left, margin_left);
    gtk_window.set_layer_shell_margin(gtk_layer_shell::Edge::Top, margin_top);
}

/// Query cursor position via Hyprland's IPC socket.
///
/// Connects to the Hyprland Unix socket and sends `j/cursorpos`,
/// which returns `{"x": N, "y": N}`. Adds a 20px offset to match
/// the Windows cursor-offset behavior.
///
/// Returns `None` if not running under Hyprland or on any error.
pub fn get_cursor_position_hyprland() -> Option<(i32, i32)> {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;

    let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
    let xdg = std::env::var("XDG_RUNTIME_DIR").ok()?;
    let socket_path = format!("{xdg}/hypr/{sig}/.socket.sock");

    let mut stream = UnixStream::connect(&socket_path).ok()?;
    stream.write_all(b"j/cursorpos").ok()?;

    let mut buf = String::new();
    stream.read_to_string(&mut buf).ok()?;

    let v: serde_json::Value = serde_json::from_str(&buf).ok()?;
    let x = v.get("x")?.as_i64()? as i32;
    let y = v.get("y")?.as_i64()? as i32;

    // 20px offset so overlay doesn't cover the cursor
    Some((x + 20, y + 20))
}
