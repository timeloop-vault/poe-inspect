//! Send Ctrl+Alt+C via XTest in a single atomic X11 flush.
//!
//! Releases any held modifiers (from the inspect hotkey) and sends the
//! combo without restoring them — the physical key state re-asserts
//! naturally. This avoids the race condition in xdotool's
//! `--clearmodifiers` which re-presses modifiers after the combo,
//! colliding with the physical key release.

use x11rb::connection::Connection as _;
use x11rb::protocol::xproto::{ConnectionExt as _, KeyButMask};
use x11rb::protocol::xtest;
use x11rb::wrapper::ConnectionExt as _;

const KEY_PRESS: u8 = 2;
const KEY_RELEASE: u8 = 3;

// X11 keysyms
const XK_SHIFT_L: u32 = 0xffe1;
const XK_CONTROL_L: u32 = 0xffe3;
const XK_ALT_L: u32 = 0xffe9;
const XK_SUPER_L: u32 = 0xffeb;
const XK_C: u32 = 0x0063;

pub fn send_copy_keystroke() -> Result<(), String> {
    // Allow the global shortcut's keyboard grab to deactivate.
    std::thread::sleep(std::time::Duration::from_millis(100));

    let (conn, screen_num) =
        x11rb::connect(None).map_err(|e| format!("X11 connect: {e}"))?;
    let setup = conn.setup();
    let root = setup.roots[screen_num].root;
    let min_kc = setup.min_keycode;
    let max_kc = setup.max_keycode;

    // Initialize the XTest extension (required before use)
    xtest::get_version(&conn, 2, 2)
        .map_err(|e| format!("XTest get_version: {e}"))?
        .reply()
        .map_err(|e| format!("XTest get_version reply: {e}"))?;

    // Build keysym→keycode lookup from the keyboard mapping
    let mapping = conn
        .get_keyboard_mapping(min_kc, max_kc - min_kc + 1)
        .map_err(|e| format!("GetKeyboardMapping: {e}"))?
        .reply()
        .map_err(|e| format!("GetKeyboardMapping reply: {e}"))?;
    let per_kc = mapping.keysyms_per_keycode as usize;
    let find_keycode = |keysym: u32| -> Option<u8> {
        for kc in min_kc..=max_kc {
            let base = (kc - min_kc) as usize * per_kc;
            for i in 0..per_kc {
                if mapping.keysyms.get(base + i) == Some(&keysym) {
                    return Some(kc);
                }
            }
        }
        None
    };

    let kc_ctrl = find_keycode(XK_CONTROL_L).ok_or("No keycode for Control_L")?;
    let kc_alt = find_keycode(XK_ALT_L).ok_or("No keycode for Alt_L")?;
    let kc_c = find_keycode(XK_C).ok_or("No keycode for 'c'")?;

    // Query which modifiers are physically held right now
    let mask = u16::from(
        conn.query_pointer(root)
            .map_err(|e| format!("QueryPointer: {e}"))?
            .reply()
            .map_err(|e| format!("QueryPointer reply: {e}"))?
            .mask,
    );

    // Release held modifiers so the combo arrives clean
    if mask & u16::from(KeyButMask::CONTROL) != 0 {
        xtest::fake_input(&conn, KEY_RELEASE, kc_ctrl, 0, root, 0, 0, 0)
            .map_err(|e| format!("XTest: {e}"))?;
    }
    if mask & u16::from(KeyButMask::MOD1) != 0 {
        xtest::fake_input(&conn, KEY_RELEASE, kc_alt, 0, root, 0, 0, 0)
            .map_err(|e| format!("XTest: {e}"))?;
    }
    if mask & u16::from(KeyButMask::SHIFT) != 0 {
        if let Some(kc) = find_keycode(XK_SHIFT_L) {
            xtest::fake_input(&conn, KEY_RELEASE, kc, 0, root, 0, 0, 0)
                .map_err(|e| format!("XTest: {e}"))?;
        }
    }
    if mask & u16::from(KeyButMask::MOD4) != 0 {
        if let Some(kc) = find_keycode(XK_SUPER_L) {
            xtest::fake_input(&conn, KEY_RELEASE, kc, 0, root, 0, 0, 0)
                .map_err(|e| format!("XTest: {e}"))?;
        }
    }

    // Send Ctrl+Alt+C — use root=0 (None) to match Xlib's XTestFakeKeyEvent
    xtest::fake_input(&conn, KEY_PRESS, kc_ctrl, 0, 0, 0, 0, 0)
        .map_err(|e| format!("XTest: {e}"))?;
    xtest::fake_input(&conn, KEY_PRESS, kc_alt, 0, 0, 0, 0, 0)
        .map_err(|e| format!("XTest: {e}"))?;
    xtest::fake_input(&conn, KEY_PRESS, kc_c, 0, 0, 0, 0, 0)
        .map_err(|e| format!("XTest: {e}"))?;
    xtest::fake_input(&conn, KEY_RELEASE, kc_c, 0, 0, 0, 0, 0)
        .map_err(|e| format!("XTest: {e}"))?;
    xtest::fake_input(&conn, KEY_RELEASE, kc_alt, 0, 0, 0, 0, 0)
        .map_err(|e| format!("XTest: {e}"))?;
    xtest::fake_input(&conn, KEY_RELEASE, kc_ctrl, 0, 0, 0, 0, 0)
        .map_err(|e| format!("XTest: {e}"))?;

    // Flush + sync to ensure events are processed
    conn.flush().map_err(|e| format!("X11 flush: {e}"))?;
    conn.sync().map_err(|e| format!("X11 sync: {e}"))?;

    Ok(())
}
