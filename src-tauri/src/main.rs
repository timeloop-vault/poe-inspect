#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use enigo::*;

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![send_adv_copy])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

#[tauri::command(async)]
fn send_adv_copy() {
  let c: char = 'c';
  let mut enigo = Enigo::new();
  enigo.key_down(Key::Control);
  enigo.key_down(Key::Alt);
  enigo.key_down(Key::Layout(c));
  enigo.key_up(Key::Layout(c));
  enigo.key_up(Key::Alt);
  enigo.key_up(Key::Control);
  println!("I was invoked from JS!");
}
