# Client.txt Log Watcher — Design

Watch PoE's `Client.txt` log to detect the active character and auto-switch profiles.

## Goal

When the user switches characters in PoE, automatically activate the profile bound to that character — including eval rules and map danger classifications. No manual profile switching needed.

## Log Format

```
YYYY/MM/DD HH:MM:SS MILLIS HASH [LEVEL Client PID] MESSAGE
```

Real examples from Mirage 3.28:
```
2026/03/12 21:52:37 878365187 1186a886 [INFO Client 48516] : You have entered Excavation.
2026/03/12 21:52:37 878365062 ca95c82d [DEBUG Client 48516] Generating level 81 area "MapWorldsExcavation" with seed 138309354
2026/03/12 21:53:14 878402234 ca95c82d [INFO Client 48516] : Scripter_BoomBoom has been slain.
2026/03/12 21:55:01 878510000 ca95c82d [INFO Client 48516] [WINDOW] Gained focus
```

## Events We Parse

### Tier 1: Zone Entry (most reliable, fires every time)

```
: You have entered <AreaName>.
```

Regex: `": You have entered (.+)\."$`

Fires on every zone transition. Doesn't include character name, but useful for:
- Detecting map entry (area name ends with " Map" or matches known map areas)
- Detecting hideout (area name contains "Hideout")
- Detecting towns (hardcoded list or heuristic)

### Tier 2: Character Name Detection

Multiple signals, none 100% reliable alone. Use all and track last-known.

**a) Character selection (login/switch)**
```
: Character name is <Name> in league <League>
```
Regex: `": Character name is (.+) in league (.+)"`

Most direct signal but version-dependent — not always logged in all PoE versions. When present, gives both character name and league.

**b) Death event**
```
: <CharacterName> has been slain.
```
Regex: `": (.+) has been slain\."`

Reliable format but only fires on death. Gives character name.

**c) Level up event**
```
: <CharacterName> is now level <N>
```
Regex: `": (.+) is now level (\\d+)"`

Only fires on level-up. Gives character name + level.

**d) Area generation (DEBUG level)**
```
Generating level <N> area "<InternalAreaId>" with seed <N>
```
Regex: `"Generating level (\\d+) area \"(.+)\" with seed"`

Fires on every zone load. Doesn't give character name, but gives internal area ID (e.g., `MapWorldsExcavation`) which is more precise than the display name.

### Tier 3: Window Focus

```
[WINDOW] Gained focus
[WINDOW] Lost focus
```

Useful for pausing/resuming overlay activity. Not needed for MVP but trivial to capture.

## File Watching Strategy

### Tailing (not full read)

Client.txt can grow to hundreds of MB. Never read the whole file.

1. **On startup**: Open file, seek to EOF, record offset
2. **On file change**: Read from stored offset to new EOF, process new lines, update offset
3. **On file truncation**: If file size < stored offset, reset offset to 0 (file was deleted/recreated by a patch)

### Polling vs Notify

Use the `notify` crate (Rust) for cross-platform file change detection:
- Windows: `ReadDirectoryChangesW`
- Linux: `inotify`
- macOS: `FSEvents`

Awakened PoE Trade uses 450ms polling — acceptable latency. `notify` gives near-instant notifications on all platforms.

### Buffer

64 KB read buffer per read cycle. Lines are always < 1 KB. Read in a loop until `bytes_read == 0` to handle rapid appends.

## File Locations

### Auto-detection paths

| Platform | Variant | Path |
|----------|---------|------|
| Windows | Standalone | `C:\Program Files (x86)\Grinding Gear Games\Path of Exile\logs\Client.txt` |
| Windows | Steam | `C:\Program Files (x86)\Steam\steamapps\common\Path of Exile\logs\Client.txt` |
| Windows | Custom | User-configured (e.g., `D:\games\PathofExile\logs\Client.txt`) |
| Linux | Steam/Proton | `~/.steam/steam/steamapps/common/Path of Exile/logs/Client.txt` |
| macOS | Standalone | `~/Library/Application Support/Path of Exile/logs/Client.txt` |

PoE2 uses equivalent paths under `Path of Exile 2/`.

### Detection logic

1. Check common paths for the selected PoE version (poe1/poe2 from settings)
2. If exactly one exists, use it
3. If multiple exist or none found, require user to set the path in settings
4. Store the configured path in settings.json

## Architecture

### Rust Backend

```
app/src-tauri/src/
  log_watcher.rs    — File tailing + line parsing + event emission
```

**LogWatcher** struct:
- Holds file path, file handle, byte offset
- `start()` — opens file, seeks to EOF (or backward-scans for recovery), begins watching
- `stop()` — closes file handle, stops watcher
- On new lines: parse → classify → emit Tauri events

**Events emitted to frontend:**
- `character-detected` → `{ name: String, source: String }` (source = "login" | "death" | "levelup")
- `area-entered` → `{ name: String, isMap: bool }`

### Frontend

**Settings:**
- New field in GeneralSettings: `poeGamePath: string` (path to PoE install directory)
- Auto-detect button that tries common paths
- Manual browse button (Tauri file dialog)

**Profile bindings:**
- New field on `StoredProfile`: `boundCharacters: string[]` (character names bound to this profile)
- When `character-detected` event fires with a name that matches a binding, auto-switch primary profile
- Settings UI: per-profile character name list (add/remove)

**State flow:**
```
Client.txt change detected (notify)
  → Read new lines from offset
  → Parse: extract zone entry / character name
  → If character name changed:
    → Find profile with matching boundCharacters
    → Set that profile as primary
    → syncActiveProfile() to backend
    → Emit "profile-switched" to overlay
  → Update offset
```

## Character Recovery on Startup

On app startup, we don't know who's logged in. Two approaches:

**Option A: Backward scan (recommended)**
Seek to `max(0, file_size - 1MB)`, read forward, parse all lines for character signals. Use the most recent character name found. Fast (< 100ms for 1 MB).

**Option B: Wait for next event**
Display "No character detected" until the user enters a zone or levels up. Simpler but worse UX.

Recommend Option A with Option B as fallback if the backward scan finds nothing.

## Edge Cases

1. **Multiple PoE installs**: User has both standalone and Steam. Settings stores the explicit path — no ambiguity.
2. **File deleted mid-session**: File handle becomes invalid. Detect via read error, attempt to reopen, reset offset.
3. **PoE not running**: File doesn't change. Watcher is idle. No events emitted. No harm.
4. **Character name collision**: Two characters with the same name on different accounts — not possible in PoE (names are unique per realm).
5. **Alt-tabbing between PoE and overlay**: Window focus events could be used to auto-show/hide overlay, but that's a separate feature.
6. **LOG FILE OPENING marker**: PoE2 writes `***** LOG FILE OPENING *****` on restart. Treat as a file reset — clear character state.

## Build Order

### Step 1: Settings — PoE game path

- Add `poeGamePath` to `GeneralSettings` in store.ts
- Add browse + auto-detect UI in GeneralSettings.tsx
- Tauri command to validate path (check `logs/Client.txt` exists)

### Step 2: Rust log watcher

- `log_watcher.rs`: file tailing with `notify` crate
- Line parser with regex patterns for zone entry, character name
- Emit `character-detected` and `area-entered` Tauri events
- Start watcher on app startup (if path configured), stop on app close
- Backward scan for character recovery

### Step 3: Profile character bindings

- Add `boundCharacters: string[]` to `StoredProfile`
- Settings UI: per-profile character binding editor (text input + list)
- Migration: default to `[]` for existing profiles

### Step 4: Auto-switch logic

- Listen for `character-detected` in App.tsx / SettingsApp.tsx
- Look up which profile has this character in `boundCharacters`
- Set that profile as primary, sync to backend
- Show brief notification in overlay: "Switched to [Profile Name]"

## Not in Scope (for now)

- Trade whisper detection (different feature, different UI)
- AFK mode detection (nice-to-have, not needed for profile switching)
- Map-specific overlay behavior based on area generation events
- PoE2 support (different area names, but same log format — works when we add PoE2 data)

## Reference

- Awakened PoE Trade's `GameLogWatcher.ts`: `_reference/awakened-poe-trade/main/src/host-files/GameLogWatcher.ts` (102 lines)
- Awakened PoE Trade's path detection: `_reference/awakened-poe-trade/main/src/host-files/utils.ts`
- Real log data: `D:\games\PathofExile\logs\Client.txt` (22.2 MB, Mirage 3.28)
