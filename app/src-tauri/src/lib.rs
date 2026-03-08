use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{Code, Modifiers, ShortcutState};

/// Send Ctrl+Alt+C to the foreground window (PoE) via enigo,
/// wait briefly, then read clipboard and emit to frontend.
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

        // Get cursor position for overlay placement
        let (cursor_x, cursor_y) = get_cursor_position();

        // Position and show the overlay window
        if let Some(window) = app.get_webview_window("overlay") {
            let _ = window.set_position(tauri::Position::Physical(
                tauri::PhysicalPosition::new(cursor_x, cursor_y),
            ));
            let _ = window.set_ignore_cursor_events(false);
            let _ = window.show();
            let _ = window.set_focus();
        }

        // Emit item text to frontend
        let _ = app.emit("item-captured", &clipboard_text);
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
    // TODO: X11 — use XQueryPointer via x11 crate
    // Wayland — no standard API, may need libei or compositor-specific approach
    (100, 100)
}

/// Dismiss the overlay: hide window, notify frontend.
#[tauri::command]
fn dismiss_overlay(window: tauri::WebviewWindow) {
    let _ = window.hide();
    let _ = window.emit("overlay-dismissed", ());
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![dismiss_overlay])
        .setup(|app| {
            // --- System tray ---
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("PoE Inspect")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .build(app)?;

            // --- Global hotkey: Ctrl+I ---
            #[cfg(desktop)]
            {
                let handle = app.handle().clone();
                handle.plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_shortcuts(["ctrl+i"])?
                        .with_handler(move |app, shortcut, event| {
                            if event.state == ShortcutState::Pressed
                                && shortcut.matches(Modifiers::CONTROL, Code::KeyI)
                            {
                                handle_inspect(app);
                            }
                        })
                        .build(),
                )?;
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
