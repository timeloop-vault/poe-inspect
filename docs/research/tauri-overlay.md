# Tauri v2 Overlay Capabilities Research

> Research date: March 2026. Based on Tauri v2 stable (v2.x series, initially released October 2024).
> Note: Web research tools were unavailable during this session. Findings are based on knowledge of Tauri v2's documented APIs, plugin ecosystem, and community discussions as of early-to-mid 2025. Some details may have changed — verify against current docs before making final decisions.

## Table of Contents

1. [Tauri v2 Overview](#tauri-v2-overview)
2. [Window Management (Overlay Capabilities)](#window-management)
3. [Global Hotkeys](#global-hotkeys)
4. [Clipboard Access](#clipboard-access)
5. [Sending Keystrokes to Other Applications](#sending-keystrokes)
6. [Cross-Platform Gotchas](#cross-platform-gotchas)
7. [Alternatives to Tauri](#alternatives)
8. [Recommendation](#recommendation)

---

## 1. Tauri v2 Overview <a name="tauri-v2-overview"></a>

Tauri v2 reached stable with version **2.0.0** in October 2024. Key changes from v1:

- **Mobile support** (iOS, Android) — not relevant to us but shows the framework's maturity trajectory.
- **Plugin-based architecture**: Many features that were built-in to v1 are now separate plugins (clipboard, global shortcuts, shell, etc.). This is a good architectural change — smaller binary, opt-in features.
- **IPC redesign**: Commands and events are more ergonomic. Permissions system added for security.
- **Multi-webview support**: A single window can host multiple webviews. Potentially useful for complex overlay layouts.
- **Rust-side window management** via `tauri::WebviewWindowBuilder` with full control over window properties.

Binary size is typically 2-8 MB (vs. 50-150 MB for Electron), and RAM usage is significantly lower since it uses the OS webview (WebView2 on Windows, WebKit on macOS/Linux).

---

## 2. Window Management (Overlay Capabilities) <a name="window-management"></a>

### What Tauri v2 provides

The `WebviewWindowBuilder` and `tauri.conf.json` window configuration support the following properties relevant to overlays:

| Property | API | Notes |
|----------|-----|-------|
| **Transparent background** | `transparent: true` in config, `.transparent(true)` in Rust | The webview background becomes transparent. Your HTML/CSS must also set `background: transparent`. |
| **No decorations** | `decorations: false` | Removes title bar and window frame. Essential for overlay. |
| **Always on top** | `always_on_top: true` / `.always_on_top(true)` | Window stays above other windows, including fullscreen games running in **borderless windowed** mode. Does NOT overlay exclusive fullscreen. |
| **Skip taskbar** | `skip_taskbar: true` | Prevents the overlay from showing in the taskbar. |
| **Resizable** | `resizable: false` | Lock overlay size. |
| **Position control** | `.position(x, y)` | Set window position programmatically. Can position near cursor. |
| **Size control** | `.inner_size(w, h)` | Set window dimensions. |
| **Visibility** | `.visible(false)` initially, then `window.show()` / `window.hide()` | Create hidden, show on demand. |
| **Focus control** | `focused: false` / `.focused(false)` | Create window without stealing focus from the game. Critical. |
| **Shadow** | `shadow: false` | Disable window shadow for clean overlay appearance. |

### Click-Through (the critical question)

**This is the most important capability for a game overlay and the area of greatest concern.**

Tauri v2 added support for **`ignore_cursor_events`** (also referred to as click-through or mouse passthrough):

- **Rust API**: `window.set_ignore_cursor_events(true)` — makes the window completely transparent to mouse input. Clicks pass through to the window behind it.
- **JavaScript API**: Available via `WebviewWindow.setIgnoreCursorEvents(true)`.
- This can be toggled dynamically, which is exactly what we need: click-through when not showing results, solid when displaying the overlay popup.

**Platform support for `ignore_cursor_events`:**

| Platform | Support | Notes |
|----------|---------|-------|
| Windows | Yes | Uses `WS_EX_TRANSPARENT` extended window style. Works well. |
| macOS | Yes | Uses `NSWindow.ignoresMouseEvents`. Works well. |
| Linux (X11) | Yes | Works via X11 input shape extension. |
| Linux (Wayland) | Partial/No | Wayland's security model restricts this. Some compositors may support it, but it is not guaranteed. Major concern for SteamOS (which uses Gamescope/Wayland). |

### Cursor Position

To position the overlay near the cursor, we need the cursor's screen coordinates. Tauri v2 provides:

- `window.cursor_position()` (Rust) / `cursorPosition()` (JS) — returns the cursor position relative to the window.
- For **screen-level** cursor position (needed when the cursor is in the game, not our window), we likely need to use a Rust crate like `mouse_position` or platform APIs directly. This is a minor gap but solvable.

### Multi-Monitor

Tauri v2 supports multi-monitor setups. You can query available monitors and position windows on specific monitors. The overlay should appear on whichever monitor the game is running on.

---

## 3. Global Hotkeys <a name="global-hotkeys"></a>

### tauri-plugin-global-shortcut

Tauri v2 provides global hotkey registration via **`tauri-plugin-global-shortcut`**.

**Capabilities:**
- Register system-wide keyboard shortcuts that work even when the Tauri window is not focused.
- Supports modifier combinations: `Ctrl+I`, `Ctrl+Shift+X`, etc.
- Callback fires on the Rust side, where you can execute logic and emit events to the frontend.
- Shortcuts can be registered/unregistered dynamically.

**Usage (Rust side):**
```rust
app.handle().plugin(tauri_plugin_global_shortcut::Builder::new().build())?;

// Register shortcut
app.global_shortcut().on_shortcut("CmdOrCtrl+I", |_app, _shortcut, event| {
    if event.state == ShortcutState::Pressed {
        // Trigger item inspection flow
    }
})?;
```

**Platform support:**

| Platform | Support | Notes |
|----------|---------|-------|
| Windows | Yes | Reliable. Uses `RegisterHotKey` Win32 API. |
| macOS | Yes | Works, but macOS may require accessibility permissions. |
| Linux (X11) | Yes | Uses `XGrabKey`. Works well. |
| Linux (Wayland) | Problematic | Wayland has no standard protocol for global hotkeys. Some compositors (KDE, GNOME via portals) support `GlobalShortcuts` portal, but coverage is inconsistent. **SteamOS/Gamescope is a concern.** |

**Key concern:** The shortcut must fire while Path of Exile has focus. On Windows and X11, this works reliably. On Wayland, it depends on compositor support for the `org.freedesktop.portal.GlobalShortcuts` portal or similar mechanisms.

---

## 4. Clipboard Access <a name="clipboard-access"></a>

### tauri-plugin-clipboard-manager

Tauri v2 provides clipboard access via **`tauri-plugin-clipboard-manager`**.

**Capabilities:**
- Read text from clipboard: `clipboard.readText()`
- Write text to clipboard: `clipboard.writeText(text)`
- Available from both Rust and JavaScript sides.

**Usage:**
```rust
// Rust side
use tauri_plugin_clipboard_manager::ClipboardExt;
let text = app.clipboard().read_text()?;
```

```typescript
// JavaScript side
import { readText } from '@tauri-apps/plugin-clipboard-manager';
const text = await readText();
```

**Platform support:** Works on all platforms (Windows, macOS, Linux). No known issues. This is the simplest part of our requirements.

**For our use case:** After sending Ctrl+Alt+C to PoE, we wait a short delay (~50-100ms), then read the clipboard. The item text will be there.

---

## 5. Sending Keystrokes to Other Applications <a name="sending-keystrokes"></a>

### This is NOT built into Tauri

Tauri has no built-in capability to send keystrokes to other applications. This is an OS-level operation that requires platform-specific code. This is one of the more complex parts of our requirements.

### Options

#### Option A: Platform-specific Rust code in Tauri command

Write a Tauri command that uses OS APIs to send keystrokes:

| Platform | API | Rust Crate |
|----------|-----|------------|
| Windows | `SendInput` (Win32 API) | `windows` crate (Microsoft's official Rust bindings) or `winapi` |
| macOS | `CGEventCreateKeyboardEvent` (Core Graphics) | `core-graphics` crate |
| Linux (X11) | `XSendEvent` / `XTest` extension | `x11` crate or `xcb` |
| Linux (Wayland) | **No standard way** | Wayland deliberately prevents applications from sending input to other applications for security reasons. Some workarounds exist (e.g., `ydotool` which requires root/special permissions, or `wtype` for wlroots compositors). |

#### Option B: Use the `enigo` crate

The [`enigo`](https://github.com/enigo-rs/enigo) crate provides cross-platform keyboard and mouse simulation in Rust:

- Supports Windows, macOS, Linux (X11 and limited Wayland via `libei`).
- Can send key combinations like Ctrl+Alt+C.
- Actively maintained.
- Wayland support was added more recently and may require `libei` support in the compositor.

#### Option C: Use the `rdev` crate

The [`rdev`](https://github.com/Narsil/rdev) crate can simulate keyboard events:

- Cross-platform (Windows, macOS, Linux).
- Can send key events.
- Also supports listening for key events (alternative to global hotkeys).

#### Recommendation for keystroke sending

Use `enigo` as the primary solution. It handles the cross-platform abstraction well. For Windows (our primary target), `SendInput` via `enigo` or direct `windows` crate calls are both reliable.

**Important note on anti-cheat:** Sending keystrokes to PoE is what existing tools like Awakened PoE Trade do. PoE's Terms of Service allow one server action per keypress. Sending Ctrl+Alt+C (which copies item text to clipboard but doesn't perform a game action) is universally accepted by the community and GGG. The keystroke must be a *real* input event (via `SendInput` or equivalent), not a simulated window message.

---

## 6. Cross-Platform Gotchas <a name="cross-platform-gotchas"></a>

### Windows (Primary target)

**Status: Best supported. Few concerns.**

- Transparent, always-on-top, click-through windows all work reliably.
- Global hotkeys via `RegisterHotKey` are rock-solid.
- `SendInput` for keystrokes works perfectly.
- PoE must be in **Borderless Windowed** or **Windowed** mode (not Exclusive Fullscreen) for the overlay to appear on top. This is the standard for overlay tools — PoE's default is borderless windowed anyway.
- WebView2 (Edge-based) is included with Windows 10/11. No additional runtime needed.

### Linux (X11)

**Status: Works, with some effort.**

- Transparent windows: Work on X11 with compositing enabled (which is standard in modern desktops).
- Always-on-top: Works (`_NET_WM_STATE_ABOVE`).
- Click-through: Works via X11 input shape extension.
- Global hotkeys: Work via `XGrabKey`.
- Sending keystrokes: Works via XTest extension.
- WebKitGTK is required. Most distros have it, but it may need to be installed.

### Linux (Wayland) — including SteamOS/Steam Deck

**Status: Significant concerns. Multiple features may not work.**

Wayland's security model is fundamentally more restrictive than X11. It was designed to prevent exactly the kind of cross-application interaction we need:

| Feature | Wayland Status | Notes |
|---------|---------------|-------|
| Transparent windows | Works | Compositor-dependent but generally fine. |
| Always-on-top | Works | Most compositors support `xdg_toplevel.set_parent` or layer-shell. |
| Click-through | Partial | No standard protocol. Some compositors support it, others don't. |
| Global hotkeys | Partial | Requires `org.freedesktop.portal.GlobalShortcuts` portal. Not all compositors implement it. |
| Sending keystrokes | Very difficult | Wayland deliberately blocks this. Options: `libei` (new, limited compositor support), `ydotool` (requires root), XWayland (runs X11 apps under Wayland, may work if PoE runs under XWayland). |
| Cursor position | Partial | Wayland doesn't expose global cursor position to applications for privacy reasons. |

**SteamOS / Steam Deck specifics:**
- SteamOS uses Gamescope, a custom Wayland compositor.
- PoE on Linux typically runs via Proton (Wine/DXVK), which uses XWayland.
- If PoE runs under XWayland, our tool might also be able to run under XWayland and use X11 APIs. This is the most likely viable path.
- Alternatively, Gamescope has its own overlay mechanisms that might be usable.

**Practical recommendation for Wayland:** Target XWayland mode initially. If the game runs under XWayland (which it does via Proton), and our tool also runs under XWayland, then X11 APIs should work. Pure Wayland support can be treated as a future goal.

### macOS

**Status: Works with caveats.**

- Transparent windows: Work fine.
- Always-on-top: Works, but **macOS fullscreen (Mission Control fullscreen) creates a separate Space** that other windows cannot overlay. PoE on macOS would need to run in windowed/borderless mode.
- Click-through: Works via `NSWindow.ignoresMouseEvents`.
- Global hotkeys: Work, but **macOS requires the app to have Accessibility permissions** (System Preferences > Privacy > Accessibility). Users must manually grant this. The app should detect and prompt for this.
- Sending keystrokes: Works via Core Graphics, but also requires Accessibility permissions.
- Clipboard: Works fine.
- WebKit (Safari engine) is used, which has some differences from Chromium-based engines in CSS/JS behavior. Test the UI on Safari/WebKit.

---

## 7. Alternatives to Tauri <a name="alternatives"></a>

### 7a. Electron

| Aspect | Assessment |
|--------|-----------|
| Overlay support | Mature. `BrowserWindow` with `transparent: true`, `alwaysOnTop: true`, `frame: false`, `focusable: false`. Well-tested in production overlay apps. |
| Click-through | `win.setIgnoreMouseEvents(true, { forward: true })` — the `forward` option even allows detecting mouse enter/leave while click-through, enabling hover-to-activate patterns. |
| Global hotkeys | Built-in `globalShortcut` module. Reliable on all platforms. |
| Clipboard | Built-in `clipboard` module. |
| Sending keystrokes | Not built-in. Same challenge as Tauri — need `robotjs`, `nut-tree`, or native modules. |
| Binary size | 50-150 MB. Bundles Chromium. |
| RAM usage | 80-200+ MB typical. Significant overhead for a background tool. |
| Cross-platform | Excellent. Mature on all three platforms. |

**Verdict:** Electron works well for overlays and is battle-tested (Awakened PoE Trade uses Electron). However, the resource overhead is significant for a tool that runs in the background during gaming, where you want maximum resources available for the game. Also, we want to write core logic in Rust, and Electron's native module story (node-gyp, napi-rs) adds complexity.

### 7b. Raw Platform APIs (Win32 + X11 + Cocoa)

| Aspect | Assessment |
|--------|-----------|
| Overlay support | Full control. Win32 layered windows (`WS_EX_LAYERED`, `WS_EX_TRANSPARENT`, `WS_EX_TOPMOST`) are the gold standard for game overlays. |
| Performance | Minimal overhead. No webview, no runtime. |
| UI flexibility | Limited. Drawing UI with Win32/GDI or Direct2D is tedious. No HTML/CSS. |
| Cross-platform | Requires writing separate implementations for each platform. Massive effort. |
| Maintenance | High. Three codebases for windowing. |

**Verdict:** Maximum control and minimum overhead, but the development cost is extremely high. Only makes sense if Tauri/Electron can't meet requirements, or if targeting only Windows.

### 7c. egui (Pure Rust, immediate-mode GUI)

| Aspect | Assessment |
|--------|-----------|
| Overlay support | Possible via `eframe` (egui's windowing integration). Supports transparent, undecorated, always-on-top windows. Uses `winit` for windowing. |
| Click-through | `winit` supports `Window::set_cursor_hittest(false)` which is the click-through equivalent. |
| Rendering | GPU-accelerated via `wgpu` or `glow`. Very fast. |
| UI flexibility | Immediate-mode GUI. Good for data-dense displays (perfect for item stats). Less flexible for complex layouts compared to HTML/CSS. No web technologies. |
| Ecosystem | Growing. Many widgets available. Custom rendering is straightforward. |
| Binary size | Small (2-5 MB). |
| Cross-platform | Good via `winit`. Same Wayland caveats apply. |

**Verdict:** Compelling for a Rust-first project. No webview overhead, fast rendering, good for data display. The UI would be less "web-like" but potentially more performant. Worth serious consideration, especially if we're willing to build the UI in Rust rather than TypeScript.

### 7d. iced (Rust GUI framework)

| Aspect | Assessment |
|--------|-----------|
| Overlay support | Uses `winit` for windowing, so similar capabilities to egui. Transparent windows and always-on-top are supported. |
| Click-through | Supported via `winit`. |
| Architecture | Elm-architecture (Model-View-Update). More structured than egui's immediate mode. |
| Maturity | Less mature than egui. API still evolving. |

**Verdict:** Similar to egui but with a different programming model. Less mature.

### 7e. Overlay-specific crates and approaches

- **`overlay` crate**: There have been some Rust crates specifically for game overlays, but none are mature or well-maintained enough to recommend.
- **DirectX/Vulkan hooking**: Tools like MSI Afterburner and Discord overlay inject into the game's rendering pipeline. This is the most invasive approach, works in exclusive fullscreen, but is complex, fragile, and likely to trigger anti-cheat concerns. **Not recommended** for this use case.
- **OBS-style capture**: Not applicable — we need to show UI, not capture frames.

### 7f. Overwolf

Worth mentioning: Overwolf is a platform specifically designed for game overlays. However:
- Closed-source, proprietary platform.
- Revenue sharing requirements.
- Not available on Linux/macOS.
- Limits what you can build.
- **Not suitable** for an open-source, cross-platform tool.

---

## 8. Recommendation <a name="recommendation"></a>

### Is Tauri v2 suitable for this use case?

**Yes, with caveats.** Tauri v2 is a strong choice for this project, particularly because:

1. **Windows support is excellent.** All required features (transparent windows, always-on-top, click-through, global hotkeys, clipboard) work reliably on Windows, which is the primary target.

2. **Rust backend is a natural fit.** The core logic (item parsing, affix evaluation, data pipeline) is planned in Rust anyway. Tauri lets us write Tauri commands in Rust that the frontend calls directly — no FFI overhead, no serialization hassle.

3. **Small binary and low resource usage.** Critical for a background gaming tool. Tauri is 10-20x smaller and lighter than Electron.

4. **HTML/CSS/TypeScript frontend.** Rich UI capabilities for the overlay display. Much easier to iterate on UI design than with egui or raw platform APIs.

5. **Plugin ecosystem covers most needs.** Global hotkeys and clipboard are handled by official plugins.

### Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| **Wayland support** (global hotkeys, click-through, keystroke sending) | High for Linux | Target XWayland initially. Pure Wayland can be a stretch goal. SteamOS/Proton games run under XWayland anyway. |
| **Sending keystrokes is not built-in** | Medium | Use `enigo` crate. Well-understood problem. Awakened PoE Trade solves this same issue. |
| **macOS Accessibility permissions** | Low | Standard for this type of tool. Detect and prompt user. |
| **WebKit differences on macOS/Linux** | Low | Test on WebKit early. Stick to well-supported CSS/JS features. |
| **Click-through toggle responsiveness** | Medium | Test that toggling `setIgnoreCursorEvents` is fast enough for a good UX. Should be near-instant but needs validation. |
| **Always-on-top vs. exclusive fullscreen** | Low | PoE defaults to borderless windowed. Document this requirement. |
| **Cursor position when our window is not focused** | Medium | May need platform-specific code or `enigo`/`rdev` to get global cursor position. Test this early. |

### What to validate early (prototype checklist)

Before committing to Tauri v2, build a minimal prototype that validates:

1. **Transparent, always-on-top, undecorated window** — does it appear over PoE in borderless windowed mode?
2. **Global hotkey fires while PoE has focus** — does `tauri-plugin-global-shortcut` work?
3. **Click-through toggle** — can we switch between click-through and interactive modes smoothly?
4. **Keystroke sending** — can we send Ctrl+Alt+C to PoE via `enigo` from a Tauri command?
5. **Clipboard read** — after sending the keystroke, can we read the item text?
6. **Window positioning near cursor** — can we get the cursor position and place the overlay window there?
7. **Focus behavior** — does showing the overlay steal focus from PoE? (It must not.)

If all seven work on Windows, Tauri v2 is confirmed as the right choice. Items 1-3 and 6-7 are the highest risk.

### Alternative worth considering: egui

If Tauri's webview adds unwanted complexity or overhead, **egui** is the strongest alternative:

- Pure Rust stack (no TypeScript, no webview, no web complexity).
- Excellent for data-dense displays (our primary UI is item stats + tier colors).
- Lower overhead than even Tauri.
- Same cross-platform caveats (Wayland issues come from `winit`, which both Tauri and egui use).

The tradeoff is losing HTML/CSS for UI design. For this project's UI (which is primarily structured data display, not a complex web app), egui could actually be a better fit. However, if we want a polished, customizable UI with themes, animations, or complex layouts in the future, Tauri's web-based frontend is more flexible.

### Final verdict

**Start with Tauri v2.** It provides the best balance of:
- Low resource overhead (critical for gaming)
- Rich UI capabilities (HTML/CSS for overlay display)
- Rust backend (natural fit for our core logic)
- Cross-platform support (best on Windows, workable on Linux/macOS)
- Active development and community

Build the 7-point prototype on Windows first. If any of the overlay capabilities prove unreliable, egui is the fallback. Do not consider Electron unless both Tauri and egui fail — the resource overhead is too high for a gaming background tool.
