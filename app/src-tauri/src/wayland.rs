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

    // Anchor to all 4 edges to fill the entire monitor (fullscreen overlay)
    gtk_window.set_anchor(gtk_layer_shell::Edge::Top, true);
    gtk_window.set_anchor(gtk_layer_shell::Edge::Left, true);
    gtk_window.set_anchor(gtk_layer_shell::Edge::Bottom, true);
    gtk_window.set_anchor(gtk_layer_shell::Edge::Right, true);

    // Don't reserve screen space (not a panel/dock)
    gtk_window.set_exclusive_zone(-1);

    // Receive keyboard input when focused
    gtk_window.set_keyboard_mode(gtk_layer_shell::KeyboardMode::OnDemand);

    // Namespace for compositor rules (e.g., Hyprland `layerrule`)
    gtk_window.set_namespace("poe-inspect");
}

/// Initialize the toast window as a layer-shell surface.
///
/// Anchored to the top edge only — the compositor centers it horizontally.
/// Uses `KeyboardMode::None` so it never steals focus. Click-through is
/// handled by `set_ignore_cursor_events` after the window is realized.
pub fn init_toast_layer_shell(gtk_window: &gtk::ApplicationWindow) {
    if gtk_window.is_realized() {
        gtk_window.unrealize();
    }
    gtk_window.init_layer_shell();
    gtk_window.set_layer(gtk_layer_shell::Layer::Overlay);

    // Anchor top only — compositor centers horizontally
    gtk_window.set_anchor(gtk_layer_shell::Edge::Top, true);

    // Small margin from the top edge
    gtk_window.set_layer_shell_margin(gtk_layer_shell::Edge::Top, 40);

    gtk_window.set_exclusive_zone(-1);
    gtk_window.set_keyboard_mode(gtk_layer_shell::KeyboardMode::None);
    gtk_window.set_namespace("poe-inspect-toast");
}

/// Position a layer-shell surface at absolute screen coordinates (x, y).
///
/// Layer surfaces are positioned via margins from their anchor edges.
/// Since we anchor Top+Left, we set left-margin = x and top-margin = y
/// relative to the target monitor.
#[allow(dead_code)]
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

/// Send a command to Hyprland's IPC socket and return the response.
fn hyprctl(command: &str) -> Option<String> {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;

    let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
    let xdg = std::env::var("XDG_RUNTIME_DIR").ok()?;
    let socket_path = format!("{xdg}/hypr/{sig}/.socket.sock");

    let mut stream = UnixStream::connect(&socket_path).ok()?;
    stream.write_all(command.as_bytes()).ok()?;

    let mut buf = String::new();
    stream.read_to_string(&mut buf).ok()?;
    Some(buf)
}

/// Query cursor position via Hyprland IPC.
pub fn get_cursor_position_hyprland() -> Option<(i32, i32)> {
    let buf = hyprctl("j/cursorpos")?;
    let v: serde_json::Value = serde_json::from_str(&buf).ok()?;
    let x = v.get("x")?.as_i64()? as i32;
    let y = v.get("y")?.as_i64()? as i32;
    Some((x, y))
}

/// Read text from the X11 CLIPBOARD selection directly via
/// `ConvertSelection` → `SelectionNotify` → `GetProperty`.
///
/// Used to snapshot and poll the clipboard in the X11 fallback path:
/// when the Wayland watcher times out (keystroke not processed by PoE),
/// we read the old value, retry the keystroke, then poll until it changes.
pub fn read_x11_clipboard(timeout_ms: u64) -> Result<String, String> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{ConnectionExt, WindowClass};
    use x11rb::protocol::Event;

    let (conn, screen_num) = x11rb::connect(None).map_err(|e| format!("X11 connect: {e}"))?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    // Create a temporary INPUT_ONLY window as requestor
    let requestor = conn
        .generate_id()
        .map_err(|e| format!("generate_id: {e}"))?;
    conn.create_window(
        0, // depth (0 = copy from parent, fine for InputOnly)
        requestor,
        root,
        0,
        0,
        1,
        1,
        0,
        WindowClass::INPUT_ONLY,
        0, // visual (0 = copy from parent)
        &x11rb::protocol::xproto::CreateWindowAux::new(),
    )
    .map_err(|e| format!("CreateWindow: {e}"))?;

    // Pipeline 3 intern_atom requests (1 roundtrip instead of 3)
    let cookie_clipboard = conn
        .intern_atom(false, b"CLIPBOARD")
        .map_err(|e| format!("intern_atom: {e}"))?;
    let cookie_utf8 = conn
        .intern_atom(false, b"UTF8_STRING")
        .map_err(|e| format!("intern_atom: {e}"))?;
    let cookie_prop = conn
        .intern_atom(false, b"_POE_INSPECT_CB")
        .map_err(|e| format!("intern_atom: {e}"))?;

    let atom_clipboard = cookie_clipboard
        .reply()
        .map_err(|e| format!("intern_atom reply: {e}"))?
        .atom;
    let atom_utf8 = cookie_utf8
        .reply()
        .map_err(|e| format!("intern_atom reply: {e}"))?
        .atom;
    let atom_prop = cookie_prop
        .reply()
        .map_err(|e| format!("intern_atom reply: {e}"))?
        .atom;

    // Ask the CLIPBOARD owner to convert to UTF8_STRING and store in our property
    conn.convert_selection(
        requestor,
        atom_clipboard,
        atom_utf8,
        atom_prop,
        x11rb::CURRENT_TIME,
    )
    .map_err(|e| format!("ConvertSelection: {e}"))?;
    conn.flush().map_err(|e| format!("Flush: {e}"))?;

    // Poll for the SelectionNotify response with timeout
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
    loop {
        if std::time::Instant::now() >= deadline {
            conn.destroy_window(requestor)
                .map_err(|e| format!("DestroyWindow: {e}"))?;
            conn.flush().map_err(|e| format!("Flush: {e}"))?;
            return Err("Timed out waiting for SelectionNotify".into());
        }

        match conn
            .poll_for_event()
            .map_err(|e| format!("poll_for_event: {e}"))?
        {
            Some(Event::SelectionNotify(ev))
                if ev.requestor == requestor && ev.selection == atom_clipboard =>
            {
                if ev.property == x11rb::NONE {
                    conn.destroy_window(requestor)
                        .map_err(|e| format!("DestroyWindow: {e}"))?;
                    conn.flush().map_err(|e| format!("Flush: {e}"))?;
                    return Err("Selection owner refused conversion".into());
                }
                break;
            }
            Some(_) => {} // Ignore other events
            None => {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    }

    // Read the property data
    let reply = conn
        .get_property(
            true, // delete after reading
            requestor,
            atom_prop,
            0u32, // AnyPropertyType
            0,
            256 * 1024, // 1MB in 32-bit units — PoE items are <10KB
        )
        .map_err(|e| format!("GetProperty: {e}"))?
        .reply()
        .map_err(|e| format!("GetProperty reply: {e}"))?;

    conn.destroy_window(requestor)
        .map_err(|e| format!("DestroyWindow: {e}"))?;
    conn.flush().map_err(|e| format!("Flush: {e}"))?;

    String::from_utf8(reply.value).map_err(|e| format!("UTF-8 decode: {e}"))
}
