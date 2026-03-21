/// Send Ctrl+Alt+C to PoE and read the resulting clipboard text.
#[cfg(target_os = "linux")]
pub(crate) fn acquire_clipboard(app: &tauri::AppHandle) -> Option<String> {
    crate::clipboard::acquire_clipboard(app)
}

/// Send Ctrl+Alt+C to PoE and read the resulting clipboard text.
#[cfg(not(target_os = "linux"))]
pub(crate) fn acquire_clipboard(app: &tauri::AppHandle) -> Option<String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;

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
                eprintln!(
                    "[inspect] Clipboard read error (attempt {}): {e}",
                    attempt + 1
                );
            }
        }
    }

    None
}

/// Send Ctrl+Alt+C keystroke to PoE via XTest (Linux) or enigo (other).
#[cfg(target_os = "linux")]
pub(crate) fn send_copy_keystroke() -> Result<(), String> {
    crate::x11_input::send_copy_keystroke()
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn send_copy_keystroke() -> Result<(), String> {
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
