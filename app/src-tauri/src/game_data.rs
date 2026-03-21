use std::sync::Arc;

use poe_data::GameData;

/// Shared game data, loaded once at startup.
pub(crate) struct GameDataState(pub Arc<GameData>);

/// Load game data from extracted datc64 files.
///
/// Looks for data in these locations (first match wins):
/// 1. `POE_DATA_DIR` environment variable
/// 2. `data/` directory next to the executable (Windows/Linux release)
/// 3. `../Resources/data/` relative to executable (macOS .app bundle)
/// 4. Repo path via `CARGO_MANIFEST_DIR` (dev builds)
/// 5. `%TEMP%/poe-dat/` (dev fallback — same dir used by poe-data tests)
///
/// Returns empty GameData if no data directory is found (overlay still works,
/// just without stat resolution or open affix detection).
pub(crate) fn load_game_data() -> GameData {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));

    let candidates = [
        std::env::var("POE_DATA_DIR")
            .ok()
            .map(std::path::PathBuf::from),
        // Windows/Linux: data/ next to executable
        exe_dir.as_ref().map(|d| d.join("data")),
        // macOS .app bundle: Contents/MacOS/../Resources/data/
        exe_dir.as_ref().map(|d| d.join("../Resources/data")),
        // Dev: committed game data in the repo
        Some(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/poe-data/data"),
        ),
        Some(std::env::temp_dir().join("poe-dat")),
    ];

    for candidate in candidates.iter().flatten() {
        if candidate.join("stats.datc64").exists() {
            match poe_data::load(candidate) {
                Ok(gd) => {
                    eprintln!("Loaded game data from {}", candidate.display());
                    return gd;
                }
                Err(e) => {
                    eprintln!("Failed to load game data from {}: {e}", candidate.display());
                }
            }
        }
    }

    eprintln!("No game data found — running without stat resolution");
    GameData::new(
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    )
}
