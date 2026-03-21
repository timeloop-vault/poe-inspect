use poe_data::GameData;
use poe_trade::{TradeClient, TradeStatsIndex, TradeStatsResponse};
use tauri::Manager;
use tauri_plugin_store::StoreExt;

/// Trade API client + stats index, managed as Tauri state.
///
/// Uses `tokio::sync::Mutex` because `TradeClient` methods take `&mut self`
/// and are async (held across `.await` points). The `RwLock` lets multiple
/// commands read the index concurrently while only `refresh` writes it.
pub(crate) struct TradeState {
    pub client: tokio::sync::Mutex<TradeClient>,
    pub index: tokio::sync::RwLock<Option<TradeStatsIndex>>,
    pub filter_index: tokio::sync::RwLock<Option<poe_trade::filter_schema::FilterIndex>>,
}

/// Return the path for the cached trade stats JSON.
pub(crate) fn trade_stats_cache_path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|dir| dir.join("trade_stats.json"))
}

pub(crate) fn trade_filters_cache_path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|dir| dir.join("trade_filters.json"))
}

/// Try to load the trade stats index from disk cache.
pub(crate) fn load_cached_trade_index(
    app: &tauri::AppHandle,
    gd: &GameData,
) -> Option<TradeStatsIndex> {
    let path = trade_stats_cache_path(app)?;
    let data = std::fs::read_to_string(&path).ok()?;
    let response: TradeStatsResponse = serde_json::from_str(&data).ok()?;
    let result = TradeStatsIndex::from_response(&response, gd);
    eprintln!(
        "[trade] Loaded cached index: {}/{} matched",
        result.matched,
        result.matched + result.unmatched,
    );
    Some(result.index)
}

/// Try to load the trade filter index from disk cache.
pub(crate) fn load_cached_filter_index(
    app: &tauri::AppHandle,
) -> Option<poe_trade::filter_schema::FilterIndex> {
    let path = trade_filters_cache_path(app)?;
    let response = poe_trade::filter_schema::FilterIndex::load_response(&path).ok()?;
    let index = poe_trade::filter_schema::FilterIndex::from_response(&response);
    eprintln!(
        "[trade] Loaded cached filter index: {} filters",
        index.filter_count()
    );
    Some(index)
}

/// Initialize trade state: load cached index + POESESSID if available.
pub(crate) fn init(app: &tauri::AppHandle, gd: &GameData) {
    let cached_index = load_cached_trade_index(app, gd);
    let mut client = TradeClient::new();

    // Restore POESESSID from settings store
    if let Some(sessid) = app
        .store("settings.json")
        .ok()
        .and_then(|store| {
            store.get("trade").and_then(|v| {
                v.get("poesessid")
                    .and_then(|v| v.as_str().map(String::from))
            })
        })
        .filter(|s| !s.is_empty())
    {
        client.set_session_id(Some(sessid));
        eprintln!("[trade] POESESSID loaded from settings");
    }

    let cached_filter_index = load_cached_filter_index(app);
    app.manage(TradeState {
        client: tokio::sync::Mutex::new(client),
        index: tokio::sync::RwLock::new(cached_index),
        filter_index: tokio::sync::RwLock::new(cached_filter_index),
    });
}
