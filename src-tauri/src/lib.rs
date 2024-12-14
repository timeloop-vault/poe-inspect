use enigo::*;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager
};

const WINDOW: &str = "main";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![send_adv_copy])
        .setup(|app| {
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_i])?;

            let _tray = TrayIconBuilder::new()
                // .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        println!("quit menu item was clicked");
                        app.exit(0);
                    }
                    _ => {
                        println!("menu item {:?} not handled", event.id);
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::Focused(focused) => {
                // hide window whenever it loses focus
                if !focused {
                    window.hide().unwrap();
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command(async)]
fn send_adv_copy(app: AppHandle) {
    let mut enigo = Enigo::new(&Settings::default()).unwrap();

    println!("Copying to clipboard");
    let _ = enigo.key(Key::Control, Direction::Press);
    let _ = enigo.key(Key::Alt, Direction::Press);
    let _ = enigo.key(Key::Unicode('c'), Direction::Click);

    let _ = enigo.key(Key::Control, Direction::Release);
    let _ = enigo.key(Key::Alt, Direction::Release);

    println!("Display window");
    show_window(app);
}

fn show_window(app: AppHandle) {
    let window = app
        .get_webview_window(WINDOW)
        .expect("Did you label your window?");
    if let Ok(false) = window.is_visible() {
        if let Ok(_x) = window.show() {
            // TODO: ensure cursor is over the window
        }
    }
}
