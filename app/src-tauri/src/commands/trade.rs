use crate::game_data::GameDataState;
use crate::trade_state::{trade_filters_cache_path, trade_stats_cache_path, TradeState};
use poe_trade::{LeagueList, TradeQueryConfig, TradeStatsIndex};

/// Preview a trade query without executing it (no HTTP, no rate limit cost).
///
/// Returns the full `QueryBuildResult` including `mapped_stats` so the
/// frontend can populate the "Edit Search" UI with checkboxes and value inputs.
#[tauri::command]
pub(crate) async fn preview_trade_query(
    item_text: String,
    config: TradeQueryConfig,
    filter_config: Option<poe_trade::TradeFilterConfig>,
    gd: tauri::State<'_, GameDataState>,
    trade: tauri::State<'_, TradeState>,
) -> Result<poe_trade::QueryBuildResult, String> {
    let gd = &gd.0;

    let raw = poe_item::parse(&item_text).map_err(|e| format!("Parse error: {e}"))?;
    let resolved = poe_item::resolve(&raw, gd);

    let index_guard = trade.index.read().await;
    let index = index_guard
        .as_ref()
        .ok_or("Trade stats index not loaded — call refresh_trade_stats first")?;

    Ok(poe_trade::build_query(
        &resolved,
        index,
        &config,
        filter_config.as_ref(),
    ))
}

/// Get the schema-driven trade edit schema for an item.
///
/// Returns `TradeEditSchema` with all applicable filter groups, per-stat
/// schemas, and type scope options. The frontend renders this generically.
#[tauri::command]
pub(crate) async fn get_trade_edit_schema(
    item_text: String,
    config: TradeQueryConfig,
    gd: tauri::State<'_, GameDataState>,
    trade: tauri::State<'_, TradeState>,
) -> Result<poe_trade::filter_schema::TradeEditSchema, String> {
    let gd = &gd.0;

    let raw = poe_item::parse(&item_text).map_err(|e| format!("Parse error: {e}"))?;
    let resolved = poe_item::resolve(&raw, gd);

    let index_guard = trade.index.read().await;
    let stats_index = index_guard
        .as_ref()
        .ok_or("Trade stats index not loaded — call refresh_trade_stats first")?;

    let filter_guard = trade.filter_index.read().await;
    let filter_index = filter_guard
        .as_ref()
        .ok_or("Trade filter index not loaded — call refresh_trade_stats first")?;

    Ok(poe_trade::filter_schema::trade_edit_schema(
        &resolved,
        filter_index,
        stats_index,
        &config,
        gd,
    ))
}

/// Full price check: parse item -> build query -> search -> fetch prices.
///
/// Returns prices from the cheapest listings, or an error string.
#[tauri::command]
pub(crate) async fn price_check(
    item_text: String,
    config: TradeQueryConfig,
    filter_config: Option<poe_trade::TradeFilterConfig>,
    gd: tauri::State<'_, GameDataState>,
    trade: tauri::State<'_, TradeState>,
) -> Result<poe_trade::PriceCheckResult, String> {
    let gd = &gd.0;

    let raw = poe_item::parse(&item_text).map_err(|e| format!("Parse error: {e}"))?;
    let resolved = poe_item::resolve(&raw, gd);

    let index_guard = trade.index.read().await;
    let index = index_guard
        .as_ref()
        .ok_or("Trade stats index not loaded — call refresh_trade_stats first")?;

    let query_result = poe_trade::build_query(&resolved, index, &config, filter_config.as_ref());
    // Release the read lock before acquiring the client mutex.
    drop(index_guard);

    let mut client = trade.client.lock().await;
    client
        .price_check(&query_result.body, &config)
        .await
        .map_err(|e| e.to_string())
}

/// Build a trade query and return the trade site URL (no fetch).
///
/// Useful for "open on trade site" without waiting for price results.
#[tauri::command]
pub(crate) async fn trade_search_url(
    item_text: String,
    config: TradeQueryConfig,
    filter_config: Option<poe_trade::TradeFilterConfig>,
    gd: tauri::State<'_, GameDataState>,
    trade: tauri::State<'_, TradeState>,
) -> Result<String, String> {
    let gd = &gd.0;

    let raw = poe_item::parse(&item_text).map_err(|e| format!("Parse error: {e}"))?;
    let resolved = poe_item::resolve(&raw, gd);

    let index_guard = trade.index.read().await;
    let index = index_guard
        .as_ref()
        .ok_or("Trade stats index not loaded — call refresh_trade_stats first")?;

    let query_result = poe_trade::build_query(&resolved, index, &config, filter_config.as_ref());
    drop(index_guard);

    let mut client = trade.client.lock().await;
    let search = client
        .search(&query_result.body, &config.league)
        .await
        .map_err(|e| e.to_string())?;

    Ok(poe_trade::query::trade_url(&config.league, &search.id))
}

/// Fetch live trade stats from the API, rebuild the index, and cache to disk.
///
/// Returns the number of stats mapped (GGPK -> trade).
#[tauri::command]
pub(crate) async fn refresh_trade_stats(
    app: tauri::AppHandle,
    gd: tauri::State<'_, GameDataState>,
    trade: tauri::State<'_, TradeState>,
) -> Result<u32, String> {
    let mut client = trade.client.lock().await;
    let stats_response = client.fetch_stats().await.map_err(|e| e.to_string())?;

    // Also fetch filters.json (schema for structural trade filters).
    let filters_response = client.fetch_filters().await;
    drop(client);

    // Cache raw stats response to disk.
    if let Some(cache_path) = trade_stats_cache_path(&app) {
        if let Ok(json) = serde_json::to_string(&stats_response) {
            let _ = std::fs::create_dir_all(cache_path.parent().unwrap());
            if let Err(e) = std::fs::write(&cache_path, json) {
                eprintln!("Failed to cache trade stats: {e}");
            }
        }
    }

    // Cache raw filters response to disk.
    if let Ok(ref filters_resp) = filters_response {
        if let Some(cache_path) = trade_filters_cache_path(&app) {
            let _ = poe_trade::filter_schema::FilterIndex::save_response(filters_resp, &cache_path);
        }
    }

    let result = TradeStatsIndex::from_response(&stats_response, &gd.0);
    let matched = result.matched;

    eprintln!(
        "[trade] Refreshed index: {}/{} matched",
        result.matched,
        result.matched + result.unmatched,
    );

    *trade.index.write().await = Some(result.index);

    // Update filter index if fetched successfully.
    if let Ok(filters_resp) = filters_response {
        let filter_index = poe_trade::filter_schema::FilterIndex::from_response(&filters_resp);
        eprintln!(
            "[trade] Refreshed filter index: {} filters",
            filter_index.filter_count()
        );
        *trade.filter_index.write().await = Some(filter_index);
    }

    Ok(matched)
}

/// Open a URL in the user's default browser.
#[tauri::command]
pub(crate) fn open_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| format!("Failed to open URL: {e}"))
}

/// Set the POESESSID cookie on the trade client.
///
/// Enables "online only" filtering. Pass empty string to clear.
#[tauri::command]
pub(crate) async fn set_trade_session(
    poesessid: String,
    trade: tauri::State<'_, TradeState>,
) -> Result<(), String> {
    let mut client = trade.client.lock().await;
    if poesessid.is_empty() {
        client.set_session_id(None);
    } else {
        client.set_session_id(Some(poesessid));
    }
    Ok(())
}

/// Get the current trade stats index status.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TradeIndexStatus {
    loaded: bool,
    stat_count: usize,
    mapped_count: usize,
}

#[tauri::command]
pub(crate) async fn get_trade_index_status(
    trade: tauri::State<'_, TradeState>,
) -> Result<TradeIndexStatus, String> {
    let guard = trade.index.read().await;
    match guard.as_ref() {
        Some(index) => Ok(TradeIndexStatus {
            loaded: true,
            stat_count: index.len(),
            mapped_count: index.mapped_stat_count(),
        }),
        None => Ok(TradeIndexStatus {
            loaded: false,
            stat_count: 0,
            mapped_count: 0,
        }),
    }
}

/// Return the valid listing status options for trade searches.
#[tauri::command]
pub(crate) fn get_listing_statuses() -> Vec<poe_trade::ListingStatus> {
    poe_trade::listing_statuses()
}

/// Fetch the list of active leagues from GGG.
#[tauri::command]
pub(crate) async fn fetch_leagues(
    trade: tauri::State<'_, TradeState>,
) -> Result<LeagueList, String> {
    let mut client = trade.client.lock().await;
    client.fetch_leagues().await.map_err(|e| e.to_string())
}
