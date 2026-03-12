# Quick Profile Switching — Design

Fast ways to switch the active profile without opening Settings.

## Problem

Switching profiles is buried in Settings > Profiles. When swapping characters/builds mid-session, the user needs a quick way to change the active profile (which controls both eval rules and map danger classifications).

## Solution: Three Complementary Mechanisms

All three coexist. The user naturally gravitates to whichever fits their workflow.

### 1. Overlay Profile Pills

Small profile buttons in the overlay, visible on every item inspect.

**Location:** Bottom of the item card, below score section (similar to existing watching profile pills).

**Behavior:**
- Show all profiles as pill buttons (not just watching ones)
- Primary profile is highlighted (filled/accent border)
- Click a non-primary pill → switches it to primary, syncs to backend
- The overlay re-evaluates with the new profile immediately
- Map danger classifications also switch (same profile object)

**Visual:**
- Compact row of pills: `[RF Build] [Minion] [MF Culler]`
- Active pill has accent background, others are dimmed
- Reuse existing `.watching-pill` styling pattern but adapted for switching

**Differences from watching pills:**
- Watching pills show score percentages and are read-only indicators
- Profile switch pills are actionable buttons that change the primary profile
- Both can coexist — switch pills at bottom, watching indicators above them

### 2. Hotkey to Cycle Profiles

A configurable hotkey (default: `Ctrl+Shift+P`) that cycles through profiles.

**Behavior:**
- Each press advances to the next profile in list order
- Show a brief toast notification in the overlay: "Profile: RF Build"
- Toast auto-dismisses after 1.5 seconds
- Works even when the overlay isn't showing an item (toast appears briefly)
- Wraps around: last profile → first profile

**Implementation:**
- New hotkey in `HotkeySettings`: `cycleProfile`
- Rust backend: registered as global shortcut like inspect/dismiss
- On trigger: read current profiles, find next after primary, set as primary, sync
- Emit `profile-switched` event to frontend for toast display

### 3. System Tray Submenu

Profile list in the tray icon's right-click menu.

**Behavior:**
- Tray menu gains a "Profiles" submenu (or flat list if few profiles)
- Each profile is a menu item with a radio indicator for the active one
- Click to switch primary
- Menu rebuilds when profiles change (add/delete/rename)

**Implementation:**
- Extend existing tray menu setup in `lib.rs`
- Dynamic menu items from profile list
- On click: set profile as primary, sync, emit event

### 4. Startup Toast

On app launch, show a brief toast confirming the active profile.

**Behavior:**
- When the overlay window initializes, show: "Active profile: RF Build"
- Auto-dismiss after 2 seconds
- Only shows if a primary profile is set
- Same toast component used by hotkey cycling

## Toast Component

Shared by hotkey cycling and startup notification.

**Design:**
- Small floating bar at the top of the overlay window
- PoE-themed styling (dark bg, accent border)
- Profile name + optional profile color indicator
- Fade in, hold, fade out animation
- Positioned at top-center of screen (not tied to cursor/panel position)
- Click-through (doesn't block game interaction)

**Structure:**
```
┌──────────────────────────┐
│  ● Active: RF Build      │
└──────────────────────────┘
```

## State Flow

All three mechanisms do the same thing:

```
User triggers switch (pill click / hotkey / tray menu)
  → Find target profile in stored profiles
  → Set target profile role to "primary"
  → Set previous primary to "off" (unless it was "watching")
  → saveProfiles(updated)
  → syncActiveProfile(updated) → backend re-evaluates
  → Emit "profile-switched" event → toast notification
  → If overlay is showing an item: re-evaluate with new profile
```

## Settings

### New hotkey

| Key | Default | Description |
|-----|---------|-------------|
| `cycleProfile` | `Ctrl+Shift+P` | Cycle to next profile |

### No new general settings needed

The three mechanisms are always available. No toggles to enable/disable them.

## Build Order

### Step 1: Profile switch toast component

- New `ProfileToast.tsx` component (fade in/out, auto-dismiss)
- CSS for toast positioning and animation
- `profile-switched` event listener in overlay App.tsx
- Show on startup with current primary profile name

### Step 2: Overlay profile pills

- Add profile switch pills below the item card in `ItemOverlay.tsx`
- Load all profiles in App.tsx (already done for mapDanger)
- On pill click: update primary, save, sync, show toast
- Style: compact pills, accent highlight for active

### Step 3: Cycle hotkey

- Add `cycleProfile` to `HotkeySettings` type and defaults
- Register global shortcut in Rust backend
- On trigger: cycle primary profile, emit event
- Frontend shows toast on event

### Step 4: Tray submenu

- Extend tray menu in `lib.rs` with profile items
- Dynamic rebuild when profiles change
- On menu click: switch primary, sync, emit event

## Not in Scope

- Client.txt watching (shelved — character name detection too unreliable)
- Auto-switching based on any signal (all switching is manual/intentional)
- Profile reordering (list order comes from creation order, which is fine)

## Reference

- Existing watching pills: `app/src/components/ItemOverlay.tsx` lines 685-704
- Existing tray menu: `app/src-tauri/src/lib.rs` tray setup
- Existing hotkey system: `app/src/store.ts` HotkeySettings, `lib.rs` global shortcuts
