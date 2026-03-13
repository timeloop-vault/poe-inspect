# Auto-Updater Design

In-app update mechanism using `tauri-plugin-updater` and GitHub Releases.

## How It Works

1. App checks for updates on startup (and optionally via Settings button)
2. Fetches `latest.json` from GitHub Releases endpoint
3. Compares current version with manifest version (semver)
4. If newer: downloads platform-specific binary, verifies cryptographic signature
5. Prompts user → installs → relaunches

## Signing

Every update artifact must be signed. Tauri enforces this — unsigned updates are rejected.

**Key generation:**
```bash
npx tauri signer generate -w ~/.tauri/poe-inspect.key
```

Produces:
- **Private key** → `~/.tauri/poe-inspect.key` (NEVER commit, store as CI secret)
- **Public key** → paste into `tauri.conf.json` `plugins.updater.pubkey`

**CI secrets needed:**
- `TAURI_SIGNING_PRIVATE_KEY` — the private key content
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — passphrase (if set during generation)

## Update Manifest (`latest.json`)

Generated automatically by `tauri-apps/tauri-action` when `includeUpdaterJson: true`.

Format:
```json
{
  "version": "0.2.0",
  "notes": "Release notes here",
  "pub_date": "2026-03-15T12:00:00Z",
  "platforms": {
    "darwin-aarch64": {
      "signature": "<base64 signature>",
      "url": "https://github.com/.../releases/download/v0.2.0/PoE.Inspect.app.tar.gz"
    },
    "windows-x86_64": {
      "signature": "<base64 signature>",
      "url": "https://github.com/.../releases/download/v0.2.0/PoE.Inspect_0.2.0_x64-setup.nsis.zip"
    },
    "linux-x86_64": {
      "signature": "<base64 signature>",
      "url": "https://github.com/.../releases/download/v0.2.0/poe-inspect_0.2.0_amd64.AppImage.tar.gz"
    }
  }
}
```

The `tauri-action` uploads this as a release asset. The app's endpoint URL points to it.

## Endpoint

The updater endpoint in `tauri.conf.json`:
```json
"plugins": {
  "updater": {
    "pubkey": "<public key>",
    "endpoints": [
      "https://github.com/OWNER/poe-inspect/releases/latest/download/latest.json"
    ]
  }
}
```

GitHub Releases `latest` URL always resolves to the most recent non-prerelease, non-draft release. No server-side logic needed.

## Update Path Considerations

### Version Format
- Strict semver: `MAJOR.MINOR.PATCH` (e.g., `0.1.0`, `0.2.0`, `1.0.0`)
- Three places to keep in sync: `tauri.conf.json`, `app/src-tauri/Cargo.toml`, `app/package.json`
- CI validates all three match before building

### Data Migration
When updating, user data (settings, profiles) persists in the app data directory:
- Windows: `%APPDATA%/com.poe-inspect.app/`
- macOS: `~/Library/Application Support/com.poe-inspect.app/`
- Linux: `~/.config/com.poe-inspect.app/`

**Forward-compatible format rules:**
- New fields get defaults (existing profiles keep working)
- Never remove fields without a migration step
- Profile JSON schema should be versioned (add `schemaVersion` field)
- Store migration logic runs on app startup, before any data access

### Breaking Changes
If a release changes the profile format in a non-backward-compatible way:
1. Bump `schemaVersion` in profile JSON
2. Add migration function: `migrateProfile(old) → new`
3. Migration runs automatically on first launch after update
4. Keep one version of backward compat (N-1 only, not N-2)

## Implementation Steps

### Step 1: Generate signing keys
```bash
npx tauri signer generate -w ~/.tauri/poe-inspect.key
```
Add public key to `tauri.conf.json`. Add private key to GitHub repo secrets.

### Step 2: Add updater plugin
**Rust:** `cargo add tauri-plugin-updater` in `app/src-tauri/`
**TypeScript:** `npm add @tauri-apps/plugin-updater` in `app/`
**Tauri config:** Add `plugins.updater` section with pubkey + endpoint

### Step 3: Wire up update check
- On app startup: silent check, show notification if update available
- In Settings > General: "Check for Updates" button
- Update dialog: show version + release notes, "Install" / "Later" buttons

### Step 4: Add schemaVersion to profiles
- Add `schemaVersion: 1` to `StoredProfile`
- Migration runner in `store.ts` that upgrades on load
- Future-proofs all profile format changes

## Not in Scope (for now)

- Delta updates (full binary replacement is fine for our size)
- Multiple update channels (beta/stable) — single channel for now
- Silent/forced updates — always prompt the user
- Rollback mechanism — user can manually install older version from GitHub Releases
