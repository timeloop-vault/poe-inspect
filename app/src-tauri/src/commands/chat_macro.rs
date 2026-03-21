/// Execute a chat macro: open chat, paste command, optionally send.
///
/// Sequence: save clipboard -> write command -> Enter (open chat) -> wait ->
/// Ctrl+V (paste) -> wait -> Enter (send, if enabled) -> restore clipboard.
#[cfg(not(target_os = "linux"))]
pub(crate) fn execute_chat_macro(app: &tauri::AppHandle, command: &str, send: bool) {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    use tauri_plugin_clipboard_manager::ClipboardExt;

    let app = app.clone();
    let command = command.to_string();
    std::thread::spawn(move || {
        // Save current clipboard
        let saved = app.clipboard().read_text().unwrap_or_default();

        // Write command to clipboard
        if app.clipboard().write_text(&command).is_err() {
            eprintln!("[macro] Failed to write to clipboard");
            return;
        }

        let Ok(mut enigo) = Enigo::new(&Settings::default()) else {
            eprintln!("[macro] Failed to create enigo");
            return;
        };

        // Open chat
        let _ = enigo.key(Key::Return, Direction::Click);
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Select all existing text (in case chat has leftover text) and paste
        let _ = enigo.key(Key::Control, Direction::Press);
        let _ = enigo.key(Key::Unicode('a'), Direction::Click);
        let _ = enigo.key(Key::Control, Direction::Release);
        std::thread::sleep(std::time::Duration::from_millis(20));

        let _ = enigo.key(Key::Control, Direction::Press);
        let _ = enigo.key(Key::Unicode('v'), Direction::Click);
        let _ = enigo.key(Key::Control, Direction::Release);
        std::thread::sleep(std::time::Duration::from_millis(30));

        // Send or leave chat open
        if send {
            let _ = enigo.key(Key::Return, Direction::Click);
        }

        // Restore clipboard
        std::thread::sleep(std::time::Duration::from_millis(30));
        let _ = app.clipboard().write_text(&saved);
    });
}

#[cfg(target_os = "linux")]
pub(crate) fn execute_chat_macro(_app: &tauri::AppHandle, _command: &str, _send: bool) {
    // TODO: Linux clipboard write needs wl-copy / xclip
    eprintln!("[macro] Chat macros not yet supported on Linux");
}
