# Reference Image Overlays — Design

## Problem

Players constantly alt-tab to look at community cheat sheets (Betrayal rewards, atlas strategies, scarab maps, expedition logbooks, etc.). A second monitor helps, but not everyone has one — and even then, the images aren't overlaid on the game.

## Core Concept

User loads local image files → assigns them hotkeys or pins them → images render as transparent always-on-top overlays on top of PoE. Each image is independently positionable, resizable, and dismissable.

---

## Visual Design

### Image Overlay Window

```
┌──────────────────────────────────────┐
│ ▫ Betrayal Rewards          ○ ✕  │  ← thin title bar (only visible in edit mode)
│                                      │
│   ┌────────────────────────────┐     │
│   │                            │     │
│   │     (user's image)         │     │
│   │                            │     │
│   │                            │     │
│   └────────────────────────────┘     │
│                                      │
└──────────────────────────────────────┘
```

**Normal mode (viewing):**
- No title bar, no borders — just the image floating on screen
- Semi-transparent background (user-configurable opacity, default ~90%)
- Click-through when not in edit mode (clicks pass to PoE)
- Subtle rounded corners to match overlay aesthetic

**Edit mode (positioning):**
- Thin PoE-styled title bar appears: image name + pin toggle + close button
- Drag from title bar to reposition
- Drag from corners/edges to resize (maintains aspect ratio by default, Shift to free-resize)
- Opacity slider appears below the image
- Border glow indicates "edit mode active"

### What It Looks Like In-Game

```
┌─────────────────────────────────────────────────────────────┐
│                         PoE Game                            │
│                                                             │
│   ┌──────────────┐                                          │
│   │ Betrayal      │    ┌─────────────────┐                  │
│   │ Rewards Chart │    │  Item Overlay    │                  │
│   │ (pinned,      │    │  (normal inspect)│                  │
│   │  semi-opaque) │    │                  │                  │
│   └──────────────┘    └─────────────────┘                  │
│                                                             │
│                                          ┌────────────┐     │
│                                          │ Atlas Map   │     │
│                                          │ (pinned)    │     │
│                                          └────────────┘     │
└─────────────────────────────────────────────────────────────┘
```

Multiple images can be visible simultaneously. They sit below the item inspect overlay in z-order so inspecting an item always wins.

---

## Modes

| Mode | Trigger | Behavior |
|------|---------|----------|
| **Toggle** | Hotkey (per-image or global) | Show/hide the image. Pressing again hides it. |
| **Pinned** | Pin button in edit mode | Image stays visible until explicitly closed. Survives hotkey toggle cycles. |
| **Edit** | Right-click image, or button in settings | Title bar + resize handles + opacity slider. Click-through disabled so you can drag. |
| **Hidden** | Close button or hotkey | Window hidden, position/size remembered. |

### Hotkey Options

1. **Global toggle** — single hotkey shows/hides ALL non-pinned images (e.g., `Ctrl+Shift+R`)
2. **Per-image hotkey** — each image gets its own hotkey (optional, configured in settings)
3. Pinned images ignore the global toggle — they only respond to their own hotkey or close button

---

## Settings UI — Image Management

New tab in settings: **"Reference Images"**

```
┌─ Reference Images ──────────────────────────────────────────┐
│                                                              │
│  Toggle All Hotkey: [Ctrl+Shift+R]  [Edit]                  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │ ☐  Betrayal Rewards                                    │  │
│  │    📁 C:\Users\...\betrayal-chart.png                  │  │
│  │    Hotkey: [Ctrl+Shift+1]   Opacity: [90%]             │  │
│  │    Screen: Primary   Position: Top-Left                 │  │
│  │    [Edit Position]  [Remove]                            │  │
│  ├────────────────────────────────────────────────────────┤  │
│  │ ☐  Atlas Scarabs                                       │  │
│  │    📁 C:\Users\...\atlas-scarabs.png                   │  │
│  │    Hotkey: [Ctrl+Shift+2]   Opacity: [85%]             │  │
│  │    Screen: Primary   Position: Bottom-Right             │  │
│  │    [Edit Position]  [Remove]                            │  │
│  ├────────────────────────────────────────────────────────┤  │
│  │             [ + Add Image ]                             │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Add Image Flow

1. Click **"+ Add Image"**
2. File picker opens (filters: `*.png, *.jpg, *.jpeg, *.webp, *.bmp`)
3. Image appears in the list with default name (filename without extension)
4. User can:
   - Rename it (click the name to edit inline)
   - Assign a hotkey (optional)
   - Set default opacity
   - Click **"Edit Position"** → image window opens in edit mode on the primary monitor, centered

### Remove Image Flow

1. Click **"Remove"** on an image entry
2. Confirmation: "Remove *Betrayal Rewards*? The image file won't be deleted."
3. Image config removed from settings, window closed if open

### Edit Position Flow

1. Click **"Edit Position"** in settings, OR right-click a visible overlay image
2. Image enters edit mode:
   - Title bar visible with name
   - Drag to move anywhere (across monitors)
   - Drag corners to resize
   - Opacity slider on the title bar
   - Pin/unpin toggle (📌)
3. Press **Escape** or click **"Done"** to save position and exit edit mode
4. Position stored as: `{ monitor, x, y, width, height }` (monitor-relative coordinates)

---

## Multi-Monitor Handling

- **Position stored per-monitor**: Uses monitor index or identifier, plus coordinates relative to that monitor's top-left
- **"Screen" selector in settings**: Dropdown listing detected monitors (Primary, Monitor 2, etc.)
- **Drag across monitors**: In edit mode, dragging an image to another monitor updates its stored monitor
- **Monitor disconnected**: If the saved monitor is gone, fall back to primary monitor at the saved relative position
- **Scaling**: Images render at their pixel size by default, but user can resize. Size is stored as a percentage of original image dimensions so DPI changes don't break layout

---

## Data Model

Stored in `settings.json` under a `referenceImages` key:

```typescript
interface ReferenceImageConfig {
  id: string;              // UUID
  name: string;            // User-editable display name
  filePath: string;        // Absolute path to image file
  hotkey: string | null;   // Per-image hotkey (optional)
  opacity: number;         // 0.0 - 1.0, default 0.9
  pinned: boolean;         // Survives global toggle
  visible: boolean;        // Current visibility state (persisted across restarts)

  // Placement
  monitor: number;         // Monitor index (0 = primary)
  x: number;               // X offset from monitor top-left (CSS px)
  y: number;               // Y offset from monitor top-left (CSS px)
  width: number;           // Display width (CSS px)
  height: number;          // Display height (CSS px)
}

interface ReferenceImageSettings {
  globalToggleHotkey: string;   // e.g. "ctrl+shift+r"
  images: ReferenceImageConfig[];
}
```

---

## Implementation Approach

### Window Strategy: One Window Per Image

Each reference image gets its own Tauri window. This is better than a single full-screen window because:
- Independent z-order per image
- Independent monitor placement (span across monitors naturally)
- OS-level always-on-top per window
- Simpler resize/drag (OS window management)
- Each can be independently click-through or interactive

Window labels: `ref-image-{id}` (dynamic, created at runtime like toast)

### Window Properties

| Property | Value |
|----------|-------|
| `decorations` | `false` |
| `transparent` | `true` |
| `alwaysOnTop` | `true` |
| `skipTaskbar` | `true` |
| `focus` | `false` |
| `resizable` | `false` (we handle resize ourselves in edit mode) |
| `visible` | `false` (shown via command) |

### Z-Order

Item inspect overlay must always be on top of reference images. Options:
1. Hide ref images when inspect overlay is visible (simplest)
2. Set window level: ref images at "floating", inspect overlay at "screen-saver" (Tauri supports this)
3. Just accept rare overlap — inspect is centered on cursor, ref images are in corners

Option 2 is cleanest if Tauri's `window_level` API works on Windows.

### Frontend Per Window

Each ref-image window loads a minimal component:
```
RefImageWindow.tsx
  - Receives image path + config via window URL params or event
  - Renders <img> with opacity
  - In edit mode: shows title bar, handles drag/resize
  - Reports position changes back to main app via events
```

### Hotkey Integration

- Add `globalToggleRefImages` to `HotkeyConfig`
- Per-image hotkeys stored in settings, registered dynamically
- On Windows: add to keyboard hook bindings array
- Dispatch: `dispatch_hotkey_action(app, "toggle-ref-image:{id}")` or `"toggle-ref-images"`

---

## Phasing

### Phase 1 — Core (MVP)
- Settings UI: add/remove images with file picker
- Single global hotkey to show/hide all images
- Images render as always-on-top overlay windows
- Edit mode: drag to position, drag corners to resize
- Opacity per image
- Position persisted in settings.json

### Phase 2 — Polish
- Per-image hotkeys
- Pin mode (survives global toggle)
- Multi-monitor: screen selector in settings, drag across monitors
- Z-order management (below inspect overlay)
- Aspect ratio lock toggle

### Phase 3 — Nice-to-Have
- Image groups (show/hide a set of related images together)
- Preset positions (top-left, center, bottom-right quick-pick)
- Import/export image configs (share setups with friends)
- URL-based images (fetch from web, cache locally)
