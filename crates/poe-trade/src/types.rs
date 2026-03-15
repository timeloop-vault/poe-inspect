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
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Price {
    pub amount: f64,
    pub currency: String,
}

/// Summary of a price check.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
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

// ── Leagues ─────────────────────────────────────────────────────────────────

/// A league from the GGG leagues API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct League {
    /// League ID used in trade API paths (e.g., `"Mirage"`, `"Standard"`).
    pub id: String,
    /// Whether this is a private league (detected by `(PLnnnn)` suffix).
    pub private: bool,
}

/// Grouped league list returned to the frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LeagueList {
    /// Public leagues (challenge, standard, hardcore, etc.).
    pub leagues: Vec<League>,
    /// Private leagues (user-created, `(PLnnnn)` prefix).
    pub private_leagues: Vec<League>,
}

/// Raw league entry from `GET /api/leagues`.
#[derive(Debug, Deserialize)]
pub(crate) struct ApiLeague {
    pub id: String,
    #[serde(default)]
    pub rules: Vec<ApiLeagueRule>,
}

/// A rule attached to a league (e.g., `"NoParties"` for SSF).
#[derive(Debug, Deserialize)]
pub(crate) struct ApiLeagueRule {
    pub id: String,
}

// ── Listing Status ──────────────────────────────────────────────────────────

/// A valid trade listing status option.
/// Matches the GGG trade site's status dropdown.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ListingStatus {
    /// API value sent in the query body (e.g., `"available"`).
    pub id: String,
    /// Human-readable label for the UI (e.g., `"Instant Buyout and In Person"`).
    pub label: String,
}

/// Returns all valid listing status options for the trade search API.
/// The first entry is the recommended default.
#[must_use]
pub fn listing_statuses() -> Vec<ListingStatus> {
    vec![
        ListingStatus {
            id: "available".into(),
            label: "Instant Buyout and In Person".into(),
        },
        ListingStatus {
            id: "securable".into(),
            label: "Instant Buyout".into(),
        },
        ListingStatus {
            id: "online".into(),
            label: "In Person (Online)".into(),
        },
        ListingStatus {
            id: "any".into(),
            label: "Any".into(),
        },
    ]
}

// ── Query Config ────────────────────────────────────────────────────────────

/// Configuration for trade query construction.
///
/// League is required — there is no default since it changes every ~3 months.
/// The app must provide this from user settings or auto-detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TradeQueryConfig {
    /// League name (e.g., `"Mirage"`). Required — set by app from user config.
    pub league: String,
    /// Value relaxation factor (0.0–1.0). Default: 0.85 (search for 85%+ of actual value).
    pub value_relaxation: f64,
    /// Whether to use pseudo stats where available.
    pub use_pseudo_stats: bool,
    /// Listing status filter for trade searches.
    /// Values match the GGG trade site dropdown:
    /// - `"available"` — Instant Buyout and In Person (trade site default)
    /// - `"securable"` — Instant Buyout only
    /// - `"online"` — In Person (Online)
    /// - `"any"` — Any (including offline)
    pub listing_status: String,
}

impl TradeQueryConfig {
    /// Create a new config for the given league with sensible defaults.
    #[must_use]
    pub fn new(league: impl Into<String>) -> Self {
        Self {
            league: league.into(),
            value_relaxation: 0.85,
            use_pseudo_stats: false,
            listing_status: "available".into(),
        }
    }
}

// ── Trade Filter Config (Edit Search mode) ─────────────────────────────────

/// User's filter overrides for a trade search.
///
/// Sent from the frontend when in "Edit Search" mode. When `None`, the query
/// builder uses default behavior (all stats included, exact base type, etc.).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TradeFilterConfig {
    /// How specific the type filter should be (base type / item class / any).
    pub type_scope: TypeSearchScope,
    /// Per-stat overrides, indexed by flat stat position
    /// (order: enchants → implicits → explicits, skipping reminder text).
    pub stat_overrides: Vec<StatFilterOverride>,
    /// Whether to include a minimum-links filter.
    /// Default `false` — auto-include only for 5L/6L items.
    #[serde(default)]
    pub min_links_enabled: bool,
    /// Minimum link count override (only used when `min_links_enabled` is true).
    /// `None` = use the item's actual max link group size.
    #[serde(default)]
    pub min_links: Option<u32>,
    /// Whether to include a quality filter.
    #[serde(default)]
    pub quality_enabled: bool,
    /// Minimum quality override (only used when `quality_enabled` is true).
    /// `None` = use the item's actual quality value.
    #[serde(default)]
    pub quality_min: Option<u32>,
    /// Rarity filter override. `None` = use default ("nonunique" for rares).
    /// `"any"` = remove rarity restriction.
    #[serde(default)]
    pub rarity_override: Option<String>,
    /// Whether to include an item-level minimum filter.
    #[serde(default)]
    pub ilvl_enabled: bool,
    /// Minimum item level (only used when `ilvl_enabled` is true).
    /// `None` = use the item's actual item level.
    #[serde(default)]
    pub ilvl_min: Option<u32>,
    /// Override for the corrupted misc filter.
    /// `None` = default (include if item is corrupted).
    /// `Some(false)` = omit. `Some(true)` = force on.
    #[serde(default)]
    pub corrupted_override: Option<bool>,
    /// Override for the fractured misc filter.
    /// `None` = default (include if item is fractured).
    /// `Some(false)` = omit. `Some(true)` = force on.
    #[serde(default)]
    pub fractured_override: Option<bool>,
}

/// How specific the type filter should be in a trade search.
///
/// Matches the GGPK hierarchy: `BaseItemTypes` (e.g., "Demon's Horn")
/// → `ItemClasses` (e.g., "Wands") → no restriction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TypeSearchScope {
    /// Filter by exact base item type (GGPK `BaseItemTypes`, e.g., "Demon's Horn").
    /// Sets `query.type` to the base type string.
    #[default]
    BaseType,
    /// Filter by item class only (GGPK `ItemClasses`, e.g., "Wands" → trade category `"weapon.wand"`).
    /// Omits `query.type`, sets `filters.type_filters.filters.category`.
    ItemClass,
    /// No type restriction — search across all item types.
    Any,
}

/// Override for a single stat line in the trade search.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StatFilterOverride {
    /// Flat index into the item's non-reminder stat lines.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub stat_index: u32,
    /// Whether this stat is included in the search.
    pub enabled: bool,
    /// Min value override. `None` = use relaxation-computed default.
    pub min_override: Option<f64>,
    /// Max value override. `None` = no max constraint.
    #[serde(default)]
    pub max_override: Option<f64>,
}

// ── Socket Info ────────────────────────────────────────────────────────────

/// Parsed socket data from the item's socket string (e.g., `"B-B-B B"`).
///
/// Returned in `QueryBuildResult` so the frontend can populate
/// socket filter controls in the "Edit Search" UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SocketInfo {
    /// Total number of sockets.
    pub total: u32,
    /// Size of the largest linked group.
    pub max_link: u32,
    /// Red socket count.
    pub red: u32,
    /// Green socket count.
    pub green: u32,
    /// Blue socket count.
    pub blue: u32,
    /// White socket count.
    pub white: u32,
}

// ── Mapped Stat Info (returned from query builder) ─────────────────────────

/// Info about a stat that was considered during query building.
///
/// Returned in `QueryBuildResult` so the frontend can populate
/// the "Edit Search" UI with checkboxes and value inputs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MappedStat {
    /// Flat index (position in enchants → implicits → explicits, skipping reminders).
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub stat_index: u32,
    /// Trade stat ID if mapped (e.g., `"explicit.stat_3299347043"`).
    pub trade_id: Option<String>,
    /// Display text for UI label.
    pub display_text: String,
    /// Relaxation-computed min value (default for the input field).
    pub computed_min: Option<f64>,
    /// Whether this stat was included in the final query.
    pub included: bool,
}
