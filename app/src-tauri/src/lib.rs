#[cfg(target_os = "linux")]
mod clipboard;
mod clipboard_acquire;
mod commands;
mod game_data;
#[cfg(target_os = "windows")]
mod hotkey_hook;
mod stash_scroll;
mod trade_state;
#[cfg(target_os = "linux")]
mod wayland;
mod windows;
#[cfg(target_os = "linux")]
mod x11_input;

use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex};

use poe_eval::{Profile, WatchingProfileInput};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};
use tauri_plugin_store::StoreExt;
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

use crate::commands::evaluate::default_profile;
#[cfg(target_os = "windows")]
use crate::commands::hotkey::dispatch_hotkey_action;
use crate::commands::hotkey::{
    load_chat_macros, load_hotkey_config, register_hotkeys, HotkeyConfig,
};
use crate::game_data::{load_game_data, GameDataState};
use crate::windows::{show_debug_overlay, show_settings};

// ── State types ─────────────────────────────────────────────────────────

/// Primary + watching profiles, synced from the frontend.
#[derive(Clone)]
struct ProfileSet {
    primary: Option<Profile>,
    watching: Vec<WatchingProfileInput>,
}

/// Active evaluation profiles, loaded from JSON data files.
struct ProfileState(Mutex<ProfileSet>);

/// Monotonic counter for toast deduplication — prevents stale hide timers.
struct ToastCounter(AtomicU64);

/// When enabled, gameplay hotkeys (inspect, cycle profile) only fire when PoE
/// is the foreground window. Toggleable in Settings for platforms where the
/// foreground-window check isn't implemented (Linux/macOS).
struct PoeFocusGate(AtomicBool);

/// Handle to the stash-scroll mouse hook thread.
struct StashScrollState(stash_scroll::StashScrollHandle);

/// Handle to the keyboard hook thread (Windows only: WH_KEYBOARD_LL).
#[cfg(target_os = "windows")]
struct HotkeyHookState(hotkey_hook::HotkeyHookHandle);

/// Chat macro configuration, synced from the frontend.
#[derive(Debug, Clone, serde::Deserialize)]
struct ChatMacroConfig {
    hotkey: String,
    command: String,
    send: bool,
}

/// Active chat macros, stored so we can re-register after pause/resume.
struct ChatMacroState(Mutex<Vec<ChatMacroConfig>>);

struct HotkeyState(std::sync::Mutex<HotkeyConfig>);

// ── Tray menu ───────────────────────────────────────────────────────────

/// Build the system tray menu with the given profile data.
fn build_tray_menu_with_profiles(
    app: &tauri::AppHandle,
    profiles: &[(String, String, String)],
) -> tauri::Result<Menu<tauri::Wry>> {
    #[cfg(debug_assertions)]
    let show_overlay = MenuItem::with_id(
        app,
        "show_overlay",
        "Show Overlay (Debug)",
        true,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    if profiles.len() > 1 {
        let sep1 = PredefinedMenuItem::separator(app)?;
        let sep2 = PredefinedMenuItem::separator(app)?;

        let profile_items: Vec<MenuItem<tauri::Wry>> = profiles
            .iter()
            .map(|(id, name, role)| {
                let prefix = if role == "primary" { "● " } else { "   " };
                MenuItem::with_id(
                    app,
                    format!("profile:{id}"),
                    format!("{prefix}{name}"),
                    true,
                    None::<&str>,
                )
            })
            .collect::<Result<_, _>>()?;

        let mut items: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = Vec::new();
        #[cfg(debug_assertions)]
        items.push(&show_overlay);
        items.push(&settings);
        items.push(&sep1);
        for item in &profile_items {
            items.push(item);
        }
        items.push(&sep2);
        items.push(&quit);

        Menu::with_items(app, &items)
    } else {
        #[cfg(debug_assertions)]
        return Menu::with_items(app, &[&show_overlay, &settings, &quit]);
        #[cfg(not(debug_assertions))]
        Menu::with_items(app, &[&settings, &quit])
    }
}

/// Build the system tray menu by reading profiles from the store.
fn build_tray_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let profiles: Vec<(String, String, String)> = (|| {
        let store = app.store("profiles.json").ok()?;
        let val = store.get("profiles")?;
        let arr = val.as_array()?;
        Some(
            arr.iter()
                .filter_map(|p| {
                    let id = p.get("id")?.as_str()?.to_string();
                    let name = p.get("name")?.as_str()?.to_string();
                    let role = p.get("role")?.as_str().unwrap_or("off").to_string();
                    Some((id, name, role))
                })
                .collect(),
        )
    })()
    .unwrap_or_default();

    build_tray_menu_with_profiles(app, &profiles)
}

// ── App entry point ─────────────────────────────────────────────────────

pub fn run() {
    // WebKitGTK's DMA-BUF renderer has a bug with explicit sync on Wayland
    // compositors (Hyprland): it creates a wp_linux_drm_syncobj_surface but
    // doesn't set the acquire timeline before committing, causing a fatal
    // "Missing acquire timeline" protocol error. Disable DMA-BUF rendering
    // to use shared-memory buffers instead. Must be set before GTK init.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var_os("XDG_SESSION_TYPE").is_some_and(|v| v == "wayland")
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_denylist(&["overlay", "toast"])
                // Don't save/restore visibility — we control that in setup()
                // based on the "start minimized" setting
                .with_state_flags(StateFlags::all().difference(StateFlags::VISIBLE))
                .build(),
        )
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());

    // MCP plugin for AI-agent debugging — dev builds only
    #[cfg(debug_assertions)]
    let builder = builder.plugin(tauri_plugin_mcp::init_with_config(
        tauri_plugin_mcp::PluginConfig::new("poe-inspect".to_string())
            .start_socket_server(true)
            .tcp_localhost(4000),
    ));

    builder
        .manage(HotkeyState(std::sync::Mutex::new(HotkeyConfig::default())))
        .manage(PoeFocusGate(AtomicBool::new(true))) // loaded from store in setup()
        .manage(StashScrollState(stash_scroll::start()))
        .manage(ChatMacroState(Mutex::new(Vec::new())))
        .manage(GameDataState(Arc::new(load_game_data())))
        .manage(ProfileState(Mutex::new(ProfileSet {
            primary: default_profile(),
            watching: vec![],
        })))
        .manage(ToastCounter(AtomicU64::new(0)))
        .invoke_handler(tauri::generate_handler![
            commands::inspect::dismiss_overlay,
            windows::show_toast,
            commands::settings::update_tray_profiles,
            commands::hotkey::update_hotkeys,
            commands::hotkey::pause_hotkeys,
            commands::hotkey::resume_hotkeys,
            commands::settings::get_autostart,
            commands::settings::set_autostart,
            commands::settings::set_require_poe_focus,
            commands::settings::set_stash_scroll,
            commands::settings::set_stash_scroll_modifier,
            commands::hotkey::update_chat_macros,
            commands::evaluate::evaluate_item,
            commands::evaluate::set_active_profile,
            commands::evaluate::get_default_profile,
            commands::evaluate::get_predicate_schema,
            commands::evaluate::get_suggestions,
            commands::evaluate::get_stat_suggestions,
            commands::evaluate::resolve_stat_templates,
            commands::evaluate::get_map_mod_templates,
            commands::trade::preview_trade_query,
            commands::trade::get_trade_edit_schema,
            commands::trade::price_check,
            commands::trade::trade_search_url,
            commands::trade::refresh_trade_stats,
            commands::trade::get_listing_statuses,
            commands::trade::fetch_leagues,
            commands::trade::open_url,
            commands::trade::set_trade_session,
            commands::trade::get_trade_index_status,
            commands::updates::check_for_update,
            commands::updates::download_and_install_update,
            commands::browser::browser_search,
            commands::browser::browser_base_type_detail,
            commands::browser::browser_mod_pool,
            commands::browser::browser_affix_limits,
            commands::browser::open_browser_window,
        ])
        .setup(|app| {
            // --- System tray ---
            let menu = build_tray_menu(app.handle())?;

            TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("PoE Inspect")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    let id = event.id.as_ref();
                    if id == "show_overlay" {
                        show_debug_overlay(app);
                    } else if id == "settings" {
                        show_settings(app);
                    } else if id == "quit" {
                        app.exit(0);
                    } else if let Some(profile_id) = id.strip_prefix("profile:") {
                        let _ = app.emit("switch-profile", profile_id.to_string());
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                        show_settings(tray.app_handle());
                    }
                })
                .build(app)?;

            // --- Toast notification window (hidden, click-through) ---
            {
                let url = tauri::WebviewUrl::App("index.html".into());
                let _toast = tauri::WebviewWindowBuilder::new(app, "toast", url)
                    .title("Toast")
                    .decorations(false)
                    .transparent(true)
                    .always_on_top(true)
                    .skip_taskbar(true)
                    .resizable(false)
                    .focused(false)
                    .visible(false)
                    .inner_size(300.0, 50.0)
                    .build()?;
            }

            // --- Linux overlay setup ---
            #[cfg(target_os = "linux")]
            {
                let mut wayland_active = false;
                if let Some(overlay) = app.get_webview_window("overlay") {
                    if let Ok(gtk_win) = overlay.gtk_window() {
                        if wayland::detect_wayland(&gtk_win) {
                            wayland::init_overlay_layer_shell(&gtk_win);
                            wayland_active = true;
                            eprintln!("Wayland detected — layer-shell overlay initialized");
                        }
                    }
                }
                if wayland_active {
                    if let Some(toast) = app.get_webview_window("toast") {
                        if let Ok(gtk_win) = toast.gtk_window() {
                            wayland::init_toast_layer_shell(&gtk_win);
                            eprintln!("Wayland detected — layer-shell toast initialized");
                        }
                    }
                }
                app.manage(wayland::WaylandOverlayState {
                    active: wayland_active,
                });

                if wayland_active {
                    if let Some(watcher) = clipboard::ClipboardWatcher::start() {
                        eprintln!("[clipboard-watcher] Started successfully");
                        app.manage(watcher);
                    }
                }
            }

            // --- PoE focus gate + stash scroll from stored settings ---
            {
                let general = app
                    .store("settings.json")
                    .ok()
                    .and_then(|store| store.get("general"));

                let require_focus = general
                    .as_ref()
                    .and_then(|v| v.get("requirePoeFocus").and_then(|v| v.as_bool()))
                    .unwrap_or(true);
                app.state::<PoeFocusGate>()
                    .0
                    .store(require_focus, std::sync::atomic::Ordering::Relaxed);
                app.state::<StashScrollState>()
                    .0
                    .set_require_poe_focus(require_focus);

                let stash_scroll = general
                    .as_ref()
                    .and_then(|v| v.get("stashScroll").and_then(|v| v.as_bool()))
                    .unwrap_or(false);
                let scroll_modifier = general
                    .as_ref()
                    .and_then(|v| v.get("stashScrollModifier").and_then(|v| v.as_str()))
                    .unwrap_or("Ctrl");
                let scroll_state = &app.state::<StashScrollState>().0;
                scroll_state.set_enabled(stash_scroll);
                scroll_state.set_modifier(stash_scroll::ScrollModifier::from_str(scroll_modifier));
                if stash_scroll {
                    eprintln!("[stash-scroll] Enabled (modifier: {scroll_modifier})");
                }
            }

            // --- Global hotkeys from stored settings (or defaults) ---
            #[cfg(desktop)]
            {
                let handle = app.handle().clone();
                handle.plugin(tauri_plugin_global_shortcut::Builder::new().build())?;

                // Start keyboard hook (Windows) and action dispatch thread
                #[cfg(target_os = "windows")]
                {
                    let (hook_handle, action_rx) = hotkey_hook::start();
                    app.manage(HotkeyHookState(hook_handle));

                    // Spawn thread to read actions from the hook and dispatch them
                    let app_handle = app.handle().clone();
                    std::thread::Builder::new()
                        .name("hotkey-dispatch".into())
                        .spawn(move || {
                            while let Ok(action) = action_rx.recv() {
                                dispatch_hotkey_action(&app_handle, &action);
                            }
                        })
                        .expect("failed to spawn hotkey-dispatch thread");
                }

                // Load saved hotkeys + chat macros from the store
                let config = load_hotkey_config(app.handle());
                let macros = load_chat_macros(app.handle());
                register_hotkeys(app.handle(), &config, &macros);
                *app.state::<HotkeyState>().0.lock().unwrap() = config;
                *app.state::<ChatMacroState>().0.lock().unwrap() = macros;
            }

            // --- Trade state (load cached index + POESESSID if available) ---
            {
                let gd = &app.state::<GameDataState>().0;
                trade_state::init(app.handle(), gd);
            }

            // --- Start minimized check ---
            // Read the store to decide whether to show the settings window on startup.
            // Default: start minimized (just tray icon). If false, show settings.
            let start_minimized = app
                .store("settings.json")
                .ok()
                .and_then(|store| {
                    store
                        .get("general")
                        .and_then(|v| v.get("startMinimized").and_then(|v| v.as_bool()))
                })
                .unwrap_or(true);

            if start_minimized {
                // Explicitly hide — window-state plugin may have restored it as visible
                if let Some(window) = app.get_webview_window("settings") {
                    let _ = window.hide();
                }
            } else {
                show_settings(app.handle());
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            // Hide settings/browser windows on close instead of destroying them,
            // and save window state so position/size persists across restarts
            if window.label() == "settings" || window.label() == "browser" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    let _ = window
                        .app_handle()
                        .save_window_state(StateFlags::all().difference(StateFlags::VISIBLE));
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
