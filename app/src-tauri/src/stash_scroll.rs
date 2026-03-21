//! Stash tab scrolling — extend PoE's native Ctrl+scroll (stash area only)
//! to work anywhere in the window.
//!
//! PoE natively supports Ctrl+scroll to navigate stash tabs, but only when
//! the mouse is over the stash tab header area (left side). This module
//! intercepts scroll events when the mouse is outside that area and sends
//! arrow key presses instead, so stash tab scrolling works everywhere.
//!
//! Uses `WH_MOUSE_LL` on a dedicated thread with its own message pump.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;

/// Modifier key required for stash scroll activation.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum ScrollModifier {
    None = 0,
    Ctrl = 1,
    Shift = 2,
    Alt = 3,
}

impl ScrollModifier {
    pub(crate) fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "ctrl" => Self::Ctrl,
            "shift" => Self::Shift,
            "alt" => Self::Alt,
            "none" => Self::None,
            _ => Self::Ctrl,
        }
    }
}

/// Shared state read by the hook callback (must be lock-free).
struct HookState {
    enabled: AtomicBool,
    modifier: AtomicU8,
    require_poe_focus: AtomicBool,
}

/// Handle to the scroll hook thread.
pub(crate) struct StashScrollHandle {
    state: Arc<HookState>,
    _thread: std::thread::JoinHandle<()>,
}

impl StashScrollHandle {
    pub(crate) fn set_enabled(&self, enabled: bool) {
        self.state.enabled.store(enabled, Ordering::Relaxed);
    }

    pub(crate) fn set_modifier(&self, modifier: ScrollModifier) {
        self.state.modifier.store(modifier as u8, Ordering::Relaxed);
    }

    pub(crate) fn set_require_poe_focus(&self, required: bool) {
        self.state
            .require_poe_focus
            .store(required, Ordering::Relaxed);
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn start() -> StashScrollHandle {
    let state = Arc::new(HookState {
        enabled: AtomicBool::new(false),
        modifier: AtomicU8::new(ScrollModifier::Ctrl as u8),
        require_poe_focus: AtomicBool::new(true),
    });
    let state_clone = Arc::clone(&state);

    let thread = std::thread::Builder::new()
        .name("stash-scroll".into())
        .spawn(move || {
            win32::run_hook(state_clone);
        })
        .expect("failed to spawn stash-scroll thread");

    StashScrollHandle {
        state,
        _thread: thread,
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn start() -> StashScrollHandle {
    let state = Arc::new(HookState {
        enabled: AtomicBool::new(false),
        modifier: AtomicU8::new(ScrollModifier::Ctrl as u8),
        require_poe_focus: AtomicBool::new(true),
    });

    let thread = std::thread::Builder::new()
        .name("stash-scroll".into())
        .spawn(|| loop {
            std::thread::park();
        })
        .expect("failed to spawn stash-scroll thread");

    StashScrollHandle {
        state,
        _thread: thread,
    }
}

#[cfg(target_os = "windows")]
mod win32 {
    use super::{HookState, ScrollModifier};
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    const WH_MOUSE_LL: i32 = 14;
    const WM_MOUSEWHEEL: u32 = 0x020A;
    const HC_ACTION: i32 = 0;
    const INPUT_KEYBOARD: u32 = 1;
    const KEYEVENTF_KEYUP: u32 = 0x0002;
    const VK_LEFT: u16 = 0x25;
    const VK_RIGHT: u16 = 0x27;
    const VK_CONTROL: i32 = 0x11;
    const VK_SHIFT: i32 = 0x10;
    const VK_MENU: i32 = 0x12;

    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    struct MsLlHookStruct {
        pt: Point,
        mouseData: u32,
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

    #[repr(C)]
    #[allow(non_snake_case)]
    struct KeybdInput {
        wVk: u16,
        wScan: u16,
        dwFlags: u32,
        time: u32,
        dwExtraInfo: usize,
    }

    /// Matches the Windows INPUT struct layout (40 bytes on x64).
    #[repr(C)]
    #[allow(non_snake_case)]
    struct InputEvent {
        input_type: u32,
        ki: KeybdInput,
        _union_pad: [u8; 8],
    }

    const _: () = assert!(std::mem::size_of::<InputEvent>() == 40);

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
        fn GetWindowRect(hwnd: isize, rect: *mut Rect) -> i32;
        fn GetAsyncKeyState(vkey: i32) -> i16;
        fn SendInput(count: u32, inputs: *const InputEvent, size: i32) -> u32;
        fn GetLastError() -> u32;
    }

    thread_local! {
        static HOOK_STATE: std::cell::Cell<*const HookState> = const { std::cell::Cell::new(std::ptr::null()) };
    }

    fn get_poe_foreground_hwnd() -> Option<isize> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd == 0 {
            return None;
        }
        let mut buf = [0u16; 256];
        let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
        if len <= 0 {
            return None;
        }
        let title = String::from_utf16_lossy(&buf[..len as usize]);
        if title.starts_with("Path of Exile") {
            Some(hwnd)
        } else {
            None
        }
    }

    /// Check if the mouse position is in PoE's stash tab header area.
    /// Uses the same heuristic as awakened-poe-trade:
    /// left sidebar (~37% of window width), y between 154/1600 and 1192/1600.
    fn is_stash_area(mouse_x: i32, mouse_y: i32, hwnd: isize) -> bool {
        let mut rect = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
            return false;
        }
        let win_x = rect.left;
        let win_width = rect.right - rect.left;
        let win_y = rect.top;
        let win_height = rect.bottom - rect.top;
        if win_height == 0 || win_width == 0 {
            return false;
        }
        let sidebar_right = win_x + (win_width * 600 / 1600);
        if mouse_x > sidebar_right {
            return false;
        }
        let header_top = win_y + (win_height * 154 / 1600);
        let header_bottom = win_y + (win_height * 1192 / 1600);
        mouse_y > header_top && mouse_y < header_bottom
    }

    fn is_modifier_held(modifier: ScrollModifier) -> bool {
        match modifier {
            ScrollModifier::None => true,
            ScrollModifier::Ctrl => key_is_down(VK_CONTROL),
            ScrollModifier::Shift => key_is_down(VK_SHIFT),
            ScrollModifier::Alt => key_is_down(VK_MENU),
        }
    }

    fn key_is_down(vk: i32) -> bool {
        (unsafe { GetAsyncKeyState(vk) } as u16 & 0x8000) != 0
    }

    fn send_key(vk: u16) {
        let inputs = [
            InputEvent {
                input_type: INPUT_KEYBOARD,
                ki: KeybdInput {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: 0,
                },
                _union_pad: [0; 8],
            },
            InputEvent {
                input_type: INPUT_KEYBOARD,
                ki: KeybdInput {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
                _union_pad: [0; 8],
            },
        ];
        unsafe {
            SendInput(2, inputs.as_ptr(), std::mem::size_of::<InputEvent>() as i32);
        }
    }

    unsafe extern "system" fn mouse_hook_proc(code: i32, w_param: usize, l_param: isize) -> isize {
        if code == HC_ACTION && w_param as u32 == WM_MOUSEWHEEL {
            let ptr = HOOK_STATE.get();
            if !ptr.is_null() {
                let state = unsafe { &*ptr };
                if state.enabled.load(Ordering::Relaxed) {
                    let poe_hwnd = get_poe_foreground_hwnd();
                    let require_focus = state.require_poe_focus.load(Ordering::Relaxed);

                    // If focus gate is on and PoE isn't foreground, pass through
                    if require_focus && poe_hwnd.is_none() {
                        return unsafe { CallNextHookEx(0, code, w_param, l_param) };
                    }

                    let modifier_code = state.modifier.load(Ordering::Relaxed);
                    let modifier = match modifier_code {
                        0 => ScrollModifier::None,
                        1 => ScrollModifier::Ctrl,
                        2 => ScrollModifier::Shift,
                        3 => ScrollModifier::Alt,
                        _ => ScrollModifier::Ctrl,
                    };

                    if is_modifier_held(modifier) {
                        let info = unsafe { &*(l_param as *const MsLlHookStruct) };

                        // Skip stash area check only when PoE is focused
                        // (stash area detection needs the PoE window handle)
                        if let Some(hwnd) = poe_hwnd {
                            if is_stash_area(info.pt.x, info.pt.y, hwnd) {
                                return unsafe { CallNextHookEx(0, code, w_param, l_param) };
                            }
                        }

                        let delta = (info.mouseData >> 16) as i16;
                        if delta > 0 {
                            send_key(VK_LEFT);
                        } else if delta < 0 {
                            send_key(VK_RIGHT);
                        }
                        return 1;
                    }
                }
            }
        }
        unsafe { CallNextHookEx(0, code, w_param, l_param) }
    }

    pub(super) fn run_hook(state: Arc<HookState>) {
        HOOK_STATE.set(Arc::as_ptr(&state));

        let hmod = unsafe { GetModuleHandleW(std::ptr::null()) };
        let hook = unsafe { SetWindowsHookExW(WH_MOUSE_LL, mouse_hook_proc, hmod, 0) };
        if hook == 0 {
            let err = unsafe { GetLastError() };
            eprintln!("[stash-scroll] Failed to install hook, error={err}");
            return;
        }
        eprintln!("[stash-scroll] Mouse hook installed");

        unsafe {
            let mut msg = std::mem::zeroed::<Msg>();
            while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}
