//! PoE-focused global hotkeys via `WH_KEYBOARD_LL`.
//!
//! Unlike `tauri-plugin-global-shortcut` (which uses `RegisterHotKey` and always
//! consumes the key event), this module only intercepts keypresses when Path of
//! Exile is the foreground window. All other apps receive the key normally.
//!
//! Architecture mirrors `stash_scroll.rs`: dedicated thread with message pump,
//! lock-free atomic state, thread-local hook pointer.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

/// A parsed hotkey: virtual key code + modifier flags.
#[derive(Clone, Copy, Default)]
struct ParsedHotkey {
    vk: u16,
    ctrl: bool,
    shift: bool,
    alt: bool,
}

/// A single hotkey binding: key combo + action tag.
#[derive(Clone)]
struct HotkeyBinding {
    key: ParsedHotkey,
    action: String,
}

/// Shared state read by the hook callback (lock-free for hot path).
struct HookState {
    enabled: AtomicBool,
    require_poe_focus: AtomicBool,
    /// Generation counter — incremented on every `set_bindings` call.
    /// The hook thread compares this to its cached value to detect changes.
    generation: AtomicU8,
    /// Fixed-size binding array — written under lock, read lock-free.
    /// The callback reads bindings when generation changes.
    bindings: Mutex<Vec<HotkeyBinding>>,
}

/// Callback sender — the hook posts action strings here, lib.rs reads them.
static ACTION_SENDER: Mutex<Option<std::sync::mpsc::Sender<String>>> = Mutex::new(None);

/// Handle to the keyboard hook thread.
pub(crate) struct HotkeyHookHandle {
    state: Arc<HookState>,
}

impl HotkeyHookHandle {
    pub(crate) fn set_enabled(&self, enabled: bool) {
        self.state.enabled.store(enabled, Ordering::Relaxed);
    }

    pub(crate) fn set_require_poe_focus(&self, required: bool) {
        self.state
            .require_poe_focus
            .store(required, Ordering::Relaxed);
    }

    /// Replace all hotkey bindings.
    /// `entries`: list of (shortcut_string, action_name) pairs.
    pub(crate) fn set_bindings(&self, entries: &[(&str, &str)]) {
        let parsed: Vec<HotkeyBinding> = entries
            .iter()
            .filter_map(|(shortcut, action)| {
                let key = parse_shortcut(shortcut)?;
                Some(HotkeyBinding {
                    key,
                    action: action.to_string(),
                })
            })
            .collect();
        let mut bindings = self.state.bindings.lock().unwrap();
        *bindings = parsed;
        // Increment generation to signal the hook thread to refresh its cache.
        // Wrapping add is fine — we only need "changed" detection, not ordering.
        self.state.generation.fetch_add(1, Ordering::Release);
    }
}

/// Parse a shortcut string like "ctrl+shift+i" into a `ParsedHotkey`.
fn parse_shortcut(s: &str) -> Option<ParsedHotkey> {
    let mut key = ParsedHotkey::default();
    for part in s.to_lowercase().split('+') {
        match part.trim() {
            "ctrl" | "control" => key.ctrl = true,
            "shift" => key.shift = true,
            "alt" => key.alt = true,
            "escape" => key.vk = 0x1B,
            "space" => key.vk = 0x20,
            "tab" => key.vk = 0x09,
            "return" | "enter" => key.vk = 0x0D,
            "backspace" => key.vk = 0x08,
            "delete" => key.vk = 0x2E,
            "insert" => key.vk = 0x2D,
            "home" => key.vk = 0x24,
            "end" => key.vk = 0x23,
            "pageup" => key.vk = 0x21,
            "pagedown" => key.vk = 0x22,
            "up" => key.vk = 0x26,
            "down" => key.vk = 0x28,
            "left" => key.vk = 0x25,
            "right" => key.vk = 0x27,
            s if s.starts_with('f') && s.len() <= 3 => {
                // F1-F24
                if let Ok(n) = s[1..].parse::<u16>() {
                    if (1..=24).contains(&n) {
                        key.vk = 0x6F + n; // VK_F1 = 0x70
                    }
                }
            }
            s if s.len() == 1 => {
                let ch = s.chars().next()?;
                if ch.is_ascii_alphanumeric() {
                    key.vk = ch.to_ascii_uppercase() as u16;
                }
            }
            _ => {
                eprintln!("[hotkey-hook] Unknown key part: {part}");
            }
        }
    }
    if key.vk == 0 {
        return None;
    }
    Some(key)
}

/// Start the keyboard hook thread. Returns a handle for configuration
/// and a receiver for action strings dispatched by the hook.
#[cfg(target_os = "windows")]
pub(crate) fn start() -> (HotkeyHookHandle, std::sync::mpsc::Receiver<String>) {
    let state = Arc::new(HookState {
        enabled: AtomicBool::new(true),
        require_poe_focus: AtomicBool::new(true),
        generation: AtomicU8::new(0),
        bindings: Mutex::new(Vec::new()),
    });

    let (tx, rx) = std::sync::mpsc::channel();
    *ACTION_SENDER.lock().unwrap() = Some(tx);

    let state_clone = Arc::clone(&state);
    std::thread::Builder::new()
        .name("hotkey-hook".into())
        .spawn(move || {
            win32::run_hook(state_clone);
        })
        .expect("failed to spawn hotkey-hook thread");

    (HotkeyHookHandle { state }, rx)
}

/// Non-Windows stub — global shortcut plugin handles hotkeys on other platforms.
#[cfg(not(target_os = "windows"))]
pub(crate) fn start() -> (HotkeyHookHandle, std::sync::mpsc::Receiver<String>) {
    let state = Arc::new(HookState {
        enabled: AtomicBool::new(true),
        require_poe_focus: AtomicBool::new(true),
        generation: AtomicU8::new(0),
        bindings: Mutex::new(Vec::new()),
    });
    let (_tx, rx) = std::sync::mpsc::channel();
    (HotkeyHookHandle { state }, rx)
}

#[cfg(target_os = "windows")]
mod win32 {
    use super::{HookState, HotkeyBinding, ParsedHotkey, ACTION_SENDER};
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    const WH_KEYBOARD_LL: i32 = 13;
    const HC_ACTION: i32 = 0;
    const WM_KEYDOWN: u32 = 0x0100;
    const WM_SYSKEYDOWN: u32 = 0x0104;
    const VK_CONTROL: i32 = 0x11;
    const VK_SHIFT: i32 = 0x10;
    const VK_MENU: i32 = 0x12; // Alt

    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    struct KbdLlHookStruct {
        vkCode: u32,
        scanCode: u32,
        flags: u32,
        time: u32,
        dwExtraInfo: usize,
    }

    #[repr(C)]
    struct Msg {
        hwnd: isize,
        message: u32,
        w_param: usize,
        l_param: isize,
        time: u32,
        pt: Point,
    }

    type HookProc = unsafe extern "system" fn(code: i32, w_param: usize, l_param: isize) -> isize;

    extern "system" {
        fn SetWindowsHookExW(id_hook: i32, lpfn: HookProc, hmod: isize, thread_id: u32) -> isize;
        fn CallNextHookEx(hhk: isize, code: i32, w_param: usize, l_param: isize) -> isize;
        fn GetMessageW(msg: *mut Msg, hwnd: isize, filter_min: u32, filter_max: u32) -> i32;
        fn TranslateMessage(msg: *const Msg) -> i32;
        fn DispatchMessageW(msg: *const Msg) -> isize;
        fn GetModuleHandleW(module_name: *const u16) -> isize;
        fn GetForegroundWindow() -> isize;
        fn GetWindowTextW(hwnd: isize, string: *mut u16, max_count: i32) -> i32;
        fn GetAsyncKeyState(vkey: i32) -> i16;
        fn GetLastError() -> u32;
    }

    thread_local! {
        static HOOK_STATE: std::cell::Cell<*const HookState> = const { std::cell::Cell::new(std::ptr::null()) };
        /// Snapshot of bindings — copied from Mutex to avoid locking in the hot path.
        static BINDINGS_CACHE: std::cell::RefCell<(u8, Vec<HotkeyBinding>)> =
            const { std::cell::RefCell::new((0, Vec::new())) };
    }

    fn is_poe_focused() -> bool {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd == 0 {
            return false;
        }
        let mut buf = [0u16; 256];
        let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
        if len <= 0 {
            return false;
        }
        let title = String::from_utf16_lossy(&buf[..len as usize]);
        title.starts_with("Path of Exile")
    }

    fn key_is_down(vk: i32) -> bool {
        (unsafe { GetAsyncKeyState(vk) } as u16 & 0x8000) != 0
    }

    fn modifiers_match(key: &ParsedHotkey) -> bool {
        key.ctrl == key_is_down(VK_CONTROL)
            && key.shift == key_is_down(VK_SHIFT)
            && key.alt == key_is_down(VK_MENU)
    }

    /// Refresh the thread-local binding cache if bindings were updated.
    fn refresh_bindings(state: &HookState) {
        let current_gen = state.generation.load(Ordering::Acquire);
        BINDINGS_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            if cache.0 != current_gen {
                if let Ok(bindings) = state.bindings.lock() {
                    cache.1 = bindings.clone();
                    cache.0 = current_gen;
                }
            }
        });
    }

    /// Find matching binding for the given virtual key code.
    fn find_matching_action(vk: u32) -> Option<String> {
        BINDINGS_CACHE.with(|cache| {
            let cache = cache.borrow();
            for binding in &cache.1 {
                if binding.key.vk as u32 == vk && modifiers_match(&binding.key) {
                    return Some(binding.action.clone());
                }
            }
            None
        })
    }

    unsafe extern "system" fn keyboard_hook_proc(
        code: i32,
        w_param: usize,
        l_param: isize,
    ) -> isize {
        if code == HC_ACTION && (w_param as u32 == WM_KEYDOWN || w_param as u32 == WM_SYSKEYDOWN) {
            let ptr = HOOK_STATE.get();
            if !ptr.is_null() {
                let state = unsafe { &*ptr };
                if state.enabled.load(Ordering::Relaxed) {
                    let info = unsafe { &*(l_param as *const KbdLlHookStruct) };

                    // Refresh binding cache if config changed
                    refresh_bindings(state);

                    // Check if this key matches any registered hotkey
                    if let Some(action) = find_matching_action(info.vkCode) {
                        // Only consume if PoE is focused (or focus gate disabled)
                        let require_focus = state.require_poe_focus.load(Ordering::Relaxed);
                        if !require_focus || is_poe_focused() {
                            // Send action to the main thread
                            if let Ok(sender) = ACTION_SENDER.lock() {
                                if let Some(tx) = sender.as_ref() {
                                    let _ = tx.send(action);
                                }
                            }
                            return 1; // Consume
                        }
                        // Not PoE focused — fall through to pass key to other apps
                    }
                }
            }
        }
        unsafe { CallNextHookEx(0, code, w_param, l_param) }
    }

    pub(super) fn run_hook(state: Arc<HookState>) {
        HOOK_STATE.set(Arc::as_ptr(&state));

        let hmod = unsafe { GetModuleHandleW(std::ptr::null()) };
        let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, keyboard_hook_proc, hmod, 0) };
        if hook == 0 {
            let err = unsafe { GetLastError() };
            eprintln!("[hotkey-hook] Failed to install hook, error={err}");
            return;
        }
        eprintln!("[hotkey-hook] Keyboard hook installed");

        unsafe {
            let mut msg = std::mem::zeroed::<Msg>();
            while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}
