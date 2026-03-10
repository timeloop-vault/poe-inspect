//! Types for trade API data and responses.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Trade Stats API Response ────────────────────────────────────────────────

/// Raw response from `GET /api/trade/data/stats`.
#[derive(Debug, Deserialize, Serialize)]
pub struct TradeStatsResponse {
    pub result: Vec<TradeStatCategory>,
}

/// A category of stats (e.g., "Explicit", "Pseudo").
#[derive(Debug, Deserialize, Serialize)]
pub struct TradeStatCategory {
    pub label: String,
    pub entries: Vec<TradeStatEntry>,
}

/// A single stat from the trade API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TradeStatEntry {
    /// Full trade stat ID (e.g., `"explicit.stat_3299347043"`).
    pub id: String,
    /// Display text with `#` placeholders (e.g., `"+# to maximum Life"`).
    pub text: String,
    /// Category type (e.g., `"explicit"`, `"pseudo"`).
    #[serde(rename = "type")]
    pub stat_type: String,
    /// Optional dropdown options (for non-numeric stats).
    #[serde(default)]
    pub option: Option<TradeStatOptions>,
}

/// Dropdown options for option-type stats.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TradeStatOptions {
    pub options: Vec<TradeStatOption>,
}

/// A single dropdown option.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TradeStatOption {
    pub id: u32,
    pub text: String,
}

// ── Trade Stats Index ───────────────────────────────────────────────────────

/// Indexed lookup for trade stats, built from the API response.
///
/// Provides bidirectional mapping between GGPK stat IDs (e.g., `"base_maximum_life"`)
/// and trade stat numbers (e.g., `3299347043`).
#[derive(Debug)]
pub struct TradeStatsIndex {
    /// Normalized template text → trade stat entries.
    /// Multiple entries per template (same stat across explicit/implicit/fractured/etc.).
    pub(crate) by_template: HashMap<String, Vec<TradeStatEntry>>,
    /// Full trade stat ID → entry.
    pub(crate) by_trade_id: HashMap<String, TradeStatEntry>,
    /// GGPK stat ID → trade stat number (the numeric portion, e.g., `3299347043`).
    pub(crate) ggpk_to_trade: HashMap<String, u64>,
    /// Trade stat number → GGPK stat IDs (reverse direction).
    pub(crate) trade_to_ggpk: HashMap<u64, Vec<String>>,
}

// ── Search Types ────────────────────────────────────────────────────────────

/// Result of a trade search.
#[derive(Debug)]
pub struct SearchResult {
    /// Search ID (used for fetch requests and trade URL).
    pub id: String,
    /// Total matching listings.
    pub total: u32,
    /// First batch of listing IDs.
    pub listing_ids: Vec<String>,
}

/// A price on a listing.
#[derive(Debug, Clone, Serialize)]
pub struct Price {
    pub amount: f64,
    pub currency: String,
}

/// Summary of a price check.
#[derive(Debug, Serialize)]
pub struct PriceCheckResult {
    /// Search ID for constructing trade URL.
    pub search_id: String,
    /// Total matching listings.
    pub total: u32,
    /// Prices from the first few listings.
    pub prices: Vec<Price>,
    /// Trade site URL for full results.
    pub trade_url: String,
}

// ── Query Config ────────────────────────────────────────────────────────────

/// Configuration for trade query construction.
///
/// League is required — there is no default since it changes every ~3 months.
/// The app must provide this from user settings or auto-detection.
#[derive(Debug, Clone)]
pub struct TradeQueryConfig {
    /// League name (e.g., `"Mirage"`). Required — set by app from user config.
    pub league: String,
    /// Value relaxation factor (0.0–1.0). Default: 0.85 (search for 85%+ of actual value).
    pub value_relaxation: f64,
    /// Whether to use pseudo stats where available.
    pub use_pseudo_stats: bool,
    /// Whether to restrict to online listings.
    pub online_only: bool,
}

impl TradeQueryConfig {
    /// Create a new config for the given league with sensible defaults.
    #[must_use]
    pub fn new(league: impl Into<String>) -> Self {
        Self {
            league: league.into(),
            value_relaxation: 0.85,
            use_pseudo_stats: false,
            online_only: true,
        }
    }
}
