//! Trade query builder: converts a `ResolvedItem` into a trade API search body.
//!
//! Pure logic — no HTTP. Takes item data, the stats index, and user configuration,
//! produces a serializable `TradeSearchBody` ready for POST to
//! `/api/trade/search/{league}`.

use std::collections::HashSet;

use poe_data::domain::pseudo_definitions;
use poe_item::types::{
    ModDisplayType, ModTierKind, Rarity, ResolvedItem, ResolvedMod, ResolvedStatLine, SocketInfo,
};
use serde::Serialize;

use crate::types::{
    MappedStat, StatFilterOverride, TradeFilterConfig, TradeQueryConfig, TradeSearchDefaults,
    TradeStatsIndex, TypeSearchScope,
};

// ── Result ──────────────────────────────────────────────────────────────────

/// Result of building a trade query, with mapping diagnostics.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct QueryBuildResult {
    /// Serializable POST body for `/api/trade/search/{league}`.
    pub body: TradeSearchBody,
    /// Number of stat lines successfully mapped to trade filters.
    pub stats_mapped: u32,
    /// Total stat lines considered (excluding reminder text).
    pub stats_total: u32,
    /// Display text of stat lines that couldn't be mapped.
    pub unmapped_stats: Vec<String>,
    /// Per-stat mapping info for the "Edit Search" UI.
    ///
    /// One entry per non-reminder stat line (flat index order:
    /// enchants → implicits → explicits). Tells the frontend which
    /// stats are mappable, their default min values, and whether
    /// they were included in the final query.
    pub mapped_stats: Vec<MappedStat>,
    /// Parsed socket info (total count, max link, colors).
    /// `None` if the item has no sockets section.
    pub socket_info: Option<SocketInfo>,
    /// Item quality percentage (e.g., 20 for `"+20%"`).
    /// `None` if the item has no quality property.
    pub quality: Option<u32>,
}

// ── Trade search body ───────────────────────────────────────────────────────

/// POST body for `/api/trade/search/{league}`.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TradeSearchBody {
    pub query: TradeQuery,
    pub sort: TradeSort,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TradeQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<StatusFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub base_type: Option<String>,
    pub stats: Vec<StatGroup>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<QueryFilters>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StatusFilter {
    pub option: String,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StatGroup {
    #[serde(rename = "type")]
    pub group_type: StatGroupType,
    pub filters: Vec<StatFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<FilterValue>,
}

/// How stat filters within a group are combined.
#[derive(Debug, Clone, Copy, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "lowercase")]
pub enum StatGroupType {
    And,
    Count,
    Not,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StatFilter {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<FilterValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FilterValue {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TradeSort {
    pub price: String,
}

// ── Item-level filters ──────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct QueryFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_filters: Option<TypeFilters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub misc_filters: Option<MiscFilters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socket_filters: Option<SocketFilters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weapon_filters: Option<WeaponFilters>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SocketFilters {
    pub filters: SocketFilterValues,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SocketFilterValues {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<IntFilterValue>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WeaponFilters {
    pub filters: WeaponFilterValues,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WeaponFilterValues {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdps: Option<FilterValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edps: Option<FilterValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dps: Option<FilterValue>,
}

/// Integer-valued filter range (trade API requires integers for links/sockets).
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct IntFilterValue {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TypeFilters {
    pub filters: TypeFilterValues,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TypeFilterValues {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<OptionFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rarity: Option<OptionFilter>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OptionFilter {
    pub option: String,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MiscFilters {
    pub filters: MiscFilterValues,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MiscFilterValues {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ilvl: Option<FilterValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corrupted: Option<OptionFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identified: Option<OptionFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fractured_item: Option<OptionFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<FilterValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutated: Option<OptionFilter>,
}

// ── Builder ─────────────────────────────────────────────────────────────────

/// Build a trade search query from a resolved item.
///
/// Maps item mods to trade stat filters using the stats index,
/// applies value relaxation, and sets appropriate item filters.
///
/// When `filter_config` is `None`, uses smart auto-selection based on
/// `config.search_defaults` (pseudo preference, tier threshold, stat cap).
/// When `Some`, respects per-stat overrides from the "Edit Search" UI.
pub fn build_query(
    item: &ResolvedItem,
    index: &TradeStatsIndex,
    config: &TradeQueryConfig,
    filter_config: Option<&TradeFilterConfig>,
) -> QueryBuildResult {
    let defaults = &config.search_defaults;
    let is_unique = item.header.rarity == Rarity::Unique;

    // Build pseudo coverage: explicit stat_ids fully covered by active pseudos.
    let covered = if defaults.prefer_pseudos && !is_unique {
        pseudo_covered_stat_ids(item)
    } else {
        HashSet::new()
    };

    // ── Pass 1: Collect all stats with metadata ──────────────────────────
    let mut candidates: Vec<StatCandidate> = Vec::new();
    let mut flat_index = 0u32;

    for resolved_mod in item.all_mods() {
        let category = mod_trade_category(resolved_mod);

        for stat_line in &resolved_mod.stat_lines {
            if stat_line.is_reminder {
                continue;
            }

            let trade_id = resolve_trade_id(stat_line, category, index);
            let computed_min = compute_filter_value(stat_line, config).and_then(|fv| fv.min);

            let user_override = filter_config.and_then(|fc| {
                fc.stat_overrides
                    .iter()
                    .find(|o| o.stat_index == flat_index)
            });

            // Determine auto-selection eligibility (only matters without user override).
            let covered_by_pseudo = defaults.prefer_pseudos
                && resolved_mod.display_type != ModDisplayType::Pseudo
                && stat_line.stat_ids.as_ref().is_some_and(|ids| {
                    !ids.is_empty() && ids.iter().all(|id| covered.contains(id.as_str()))
                });

            let excluded_by_tier = defaults.tier_threshold.is_some_and(|threshold| {
                resolved_mod
                    .header
                    .tier
                    .as_ref()
                    .is_some_and(|t| t.number() > threshold)
            });

            let excluded_by_crafted =
                !defaults.include_crafted && resolved_mod.display_type == ModDisplayType::Crafted;

            let auto_eligible = trade_id.is_some()
                && !covered_by_pseudo
                && !excluded_by_tier
                && !excluded_by_crafted;

            let priority = stat_priority(
                resolved_mod.display_type,
                resolved_mod.is_fractured,
                resolved_mod.header.tier.as_ref(),
            );

            let pseudo_id = if resolved_mod.display_type == ModDisplayType::Pseudo {
                stat_line
                    .stat_ids
                    .as_ref()
                    .and_then(|ids| ids.first().cloned())
            } else {
                None
            };

            candidates.push(StatCandidate {
                flat_index,
                trade_id,
                computed_min,
                user_override: user_override.cloned(),
                display_text: stat_line.display_text.clone(),
                auto_eligible,
                priority,
                pseudo_id,
            });

            flat_index += 1;
        }
    }

    // ── Pass 2: Auto-select within cap ───────────────────────────────────
    let auto_selected = if is_unique {
        // Unique items: the name is the primary filter. Don't auto-select
        // stats — mods are fixed, and including roll ranges over-constrains.
        // Users can manually enable specific stats via Edit Search.
        HashSet::new()
    } else {
        auto_select_stats(&candidates, defaults)
    };

    // ── Pass 3: Build filters and mapped_stats ───────────────────────────
    let mut filters = Vec::new();
    let mut stats_mapped = 0u32;
    let stats_total = candidates.len() as u32;
    let mut unmapped_stats = Vec::new();
    let mut mapped_stats = Vec::new();

    for c in &candidates {
        let included = if let Some(ref ov) = c.user_override {
            ov.enabled && c.trade_id.is_some()
        } else {
            auto_selected.contains(&c.flat_index)
        };

        if included {
            if let Some(ref tid) = c.trade_id {
                let min_value = c
                    .user_override
                    .as_ref()
                    .and_then(|o| o.min_override)
                    .or(c.computed_min);
                let max_value = c.user_override.as_ref().and_then(|o| o.max_override);

                let value = if min_value.is_some() || max_value.is_some() {
                    Some(FilterValue {
                        min: min_value,
                        max: max_value,
                    })
                } else {
                    None
                };

                filters.push(StatFilter {
                    id: tid.clone(),
                    value,
                    disabled: None,
                });
                stats_mapped += 1;
            }
        } else if c.trade_id.is_none() {
            unmapped_stats.push(c.display_text.clone());
        }

        mapped_stats.push(MappedStat {
            stat_index: c.flat_index,
            trade_id: c.trade_id.clone(),
            display_text: c.display_text.clone(),
            computed_min: c.computed_min,
            included,
        });
    }

    // Single AND group with all stat filters.
    let stats = if filters.is_empty() {
        vec![]
    } else {
        vec![StatGroup {
            group_type: StatGroupType::And,
            filters,
            value: None,
        }]
    };

    // Socket info and quality from item (pre-parsed by poe-item)
    let socket_info = item.socket_info.as_ref();
    let quality = extract_quality(item);

    let type_scope = filter_config.map_or(TypeSearchScope::BaseType, |fc| fc.type_scope);
    let query_filters = build_item_filters(
        item,
        config,
        type_scope,
        socket_info,
        quality,
        filter_config,
    );

    let body = TradeSearchBody {
        query: TradeQuery {
            status: if config.listing_status == "any" {
                None
            } else {
                Some(StatusFilter {
                    option: config.listing_status.clone(),
                })
            },
            name: match item.header.rarity {
                Rarity::Unique => item
                    .header
                    .name
                    .as_deref()
                    .map(|n| poe_data::domain::strip_league_prefix(n).to_string()),
                _ => None,
            },
            base_type: match type_scope {
                TypeSearchScope::BaseType => Some(
                    poe_data::domain::strip_base_type_prefix(&item.header.base_type).to_string(),
                ),
                TypeSearchScope::ItemClass | TypeSearchScope::Any => None,
            },
            stats,
            filters: query_filters,
        },
        sort: TradeSort {
            price: "asc".to_string(),
        },
    };

    QueryBuildResult {
        body,
        stats_mapped,
        stats_total,
        unmapped_stats,
        mapped_stats,
        socket_info: socket_info.cloned(),
        quality,
    }
}

/// Construct the trade site URL for a completed search.
#[must_use]
pub fn trade_url(league: &str, search_id: &str) -> String {
    format!("https://www.pathofexile.com/trade/search/{league}/{search_id}")
}

// ── Auto-selection helpers ─────────────────────────────────────────────────

/// Internal data for a stat candidate during auto-selection.
struct StatCandidate {
    flat_index: u32,
    trade_id: Option<String>,
    computed_min: Option<f64>,
    user_override: Option<StatFilterOverride>,
    display_text: String,
    /// Eligible for auto-inclusion (mappable, not covered/excluded).
    auto_eligible: bool,
    /// Priority score (lower = selected first).
    priority: u32,
    /// Pseudo stat ID (e.g., `"pseudo_total_life"`), if this is a pseudo stat.
    pseudo_id: Option<String>,
}

/// Compute the set of explicit `stat_ids` fully covered by active pseudo stats.
///
/// A `stat_id` is "covered" if it appears in a `PseudoComponent` with
/// `multiplier >= 1.0` for a pseudo that actually exists on this item.
fn pseudo_covered_stat_ids(item: &ResolvedItem) -> HashSet<String> {
    // Collect pseudo IDs that are actually present on the item.
    let active_pseudo_ids: HashSet<&str> = item
        .pseudo_mods
        .iter()
        .flat_map(|pm| pm.stat_lines.iter())
        .filter_map(|sl| sl.stat_ids.as_ref())
        .filter_map(|ids| ids.first())
        .map(String::as_str)
        .collect();

    let mut covered = HashSet::new();
    for defn in pseudo_definitions() {
        if !active_pseudo_ids.contains(defn.id) {
            continue;
        }
        for comp in defn.components {
            if comp.multiplier >= 1.0 {
                for &sid in comp.stat_ids {
                    covered.insert(sid.to_string());
                }
            }
        }
    }
    covered
}

/// Compute the priority score for a stat. Lower = more important = selected first.
///
/// Priority groups:
/// - 100: Pseudo stats (broadest signal)
/// - 200: Fractured explicits (permanent, define the item)
/// - 300+tier: Explicits by tier (T1=301, no tier=310)
/// - 400: Enchants
/// - 500: Implicits
/// - 600: Crafted mods (replaceable)
/// - 700: Unique mods
fn stat_priority(
    display_type: ModDisplayType,
    is_fractured: bool,
    tier: Option<&ModTierKind>,
) -> u32 {
    match display_type {
        ModDisplayType::Pseudo => 100,
        _ if is_fractured => 200,
        ModDisplayType::Prefix | ModDisplayType::Suffix => {
            let tier_offset = tier.map_or(10, |t| t.number().min(10));
            300 + tier_offset
        }
        ModDisplayType::Enchant => 400,
        ModDisplayType::Implicit => 500,
        ModDisplayType::Crafted => 600,
        ModDisplayType::Unique => 700,
    }
}

/// Select which stats to auto-include based on smart defaults.
///
/// Returns the set of `flat_index` values that should be auto-checked.
/// Only considers candidates without user overrides. When a broader pseudo
/// is eligible (e.g., total resistance), narrower pseudos (e.g., cold res)
/// are suppressed via `pseudo_subsumes()` — regardless of iteration order.
fn auto_select_stats(candidates: &[StatCandidate], defaults: &TradeSearchDefaults) -> HashSet<u32> {
    let mut eligible: Vec<&StatCandidate> = candidates
        .iter()
        .filter(|c| c.auto_eligible && c.user_override.is_none())
        .collect();
    eligible.sort_by_key(|c| c.priority);

    // Pre-compute which pseudos are suppressed by broader pseudos in the
    // eligible set. This ensures order-independent deduplication — even if
    // "cold resistance" appears before "total resistance" in iteration order,
    // it's still suppressed because total resistance is in the eligible set.
    let mut suppressed_pseudos: HashSet<&str> = HashSet::new();
    for c in &eligible {
        if let Some(ref pid) = c.pseudo_id {
            for &subsumed in poe_data::domain::pseudo_subsumes(pid) {
                suppressed_pseudos.insert(subsumed);
            }
        }
    }

    let mut selected = HashSet::new();
    let mut count = 0u32;

    for c in &eligible {
        if count >= defaults.max_stat_filters {
            break;
        }

        // Skip pseudos suppressed by a broader pseudo.
        if let Some(ref pid) = c.pseudo_id {
            if suppressed_pseudos.contains(pid.as_str()) {
                continue;
            }
        }

        selected.insert(c.flat_index);
        count += 1;
    }

    selected
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Determine the trade API category prefix for a mod.
///
/// Delegates to `poe_data::domain::mod_trade_category()` — the mapping
/// from mod types to trade API categories is `PoE` domain knowledge.
fn mod_trade_category(m: &ResolvedMod) -> &'static str {
    let display_type = match m.display_type {
        ModDisplayType::Prefix => "prefix",
        ModDisplayType::Suffix => "suffix",
        ModDisplayType::Implicit => "implicit",
        ModDisplayType::Crafted => "crafted",
        ModDisplayType::Enchant => "enchant",
        ModDisplayType::Unique => "unique",
        ModDisplayType::Pseudo => "pseudo",
    };
    poe_data::domain::mod_trade_category(display_type, m.is_fractured)
}

/// Resolve a stat line to its full trade stat ID (e.g., `"explicit.stat_3299347043"`).
///
/// Returns `None` if the stat can't be mapped.
///
/// Pseudo mods use a different format: their `stat_ids` are already the trade
/// pseudo IDs (e.g., `"pseudo_total_life"` → `"pseudo.pseudo_total_life"`).
fn resolve_trade_id(
    stat_line: &ResolvedStatLine,
    category: &str,
    index: &TradeStatsIndex,
) -> Option<String> {
    let stat_ids = stat_line.stat_ids.as_ref()?;

    // Pseudo mods carry their trade ID directly — no stat_number lookup needed.
    // DPS pseudos are excluded: they use weapon_filters, not pseudo stat IDs.
    if category == "pseudo" {
        let pseudo_id = stat_ids.first()?;
        if poe_data::domain::is_dps_pseudo(pseudo_id) {
            return None;
        }
        return Some(format!("pseudo.{pseudo_id}"));
    }

    let trade_num = stat_ids
        .iter()
        .find_map(|sid| index.trade_stat_number(sid))?;
    Some(format!("{category}.stat_{trade_num}"))
}

/// Compute the filter value with relaxation applied.
///
/// For single-value stats, uses the display value directly.
/// For multi-value stats (e.g., "Adds # to # Damage"), uses the average.
/// Handles negative values correctly (relaxation widens the search range
/// in the appropriate direction).
fn compute_filter_value(
    stat_line: &ResolvedStatLine,
    config: &TradeQueryConfig,
) -> Option<FilterValue> {
    if stat_line.values.is_empty() {
        return None;
    }

    let raw_value = if stat_line.values.len() == 1 {
        stat_line.values[0].current as f64
    } else {
        // Multi-value: average (e.g., "Adds 11 to 24" → 17.5).
        let sum: f64 = stat_line.values.iter().map(|v| v.current as f64).sum();
        sum / stat_line.values.len() as f64
    };

    // Relaxation: allow some variation from the actual value.
    // Positive stats: min = value * factor (search for similar-or-better).
    // Negative stats: min = value * (2 - factor) (allow slightly worse penalty).
    let relaxed = if raw_value >= 0.0 {
        (raw_value * config.value_relaxation).floor()
    } else {
        (raw_value * (2.0 - config.value_relaxation)).floor()
    };

    Some(FilterValue {
        min: Some(relaxed),
        max: None,
    })
}

/// Build item-level filters (type, misc, sockets, weapon DPS).
fn build_item_filters(
    item: &ResolvedItem,
    config: &TradeQueryConfig,
    type_scope: TypeSearchScope,
    socket_info: Option<&SocketInfo>,
    quality: Option<u32>,
    filter_config: Option<&TradeFilterConfig>,
) -> Option<QueryFilters> {
    let type_filters = build_type_filters(item, type_scope, filter_config);
    let misc_filters = build_misc_filters(item, config, quality, filter_config);
    let socket_filters = build_socket_filters(socket_info, filter_config);
    let weapon_filters = build_weapon_filters(item, config);

    if type_filters.is_none()
        && misc_filters.is_none()
        && socket_filters.is_none()
        && weapon_filters.is_none()
    {
        return None;
    }

    Some(QueryFilters {
        type_filters,
        misc_filters,
        socket_filters,
        weapon_filters,
    })
}

fn build_type_filters(
    item: &ResolvedItem,
    type_scope: TypeSearchScope,
    filter_config: Option<&TradeFilterConfig>,
) -> Option<TypeFilters> {
    let default_rarity = match item.header.rarity {
        Rarity::Rare | Rarity::Magic | Rarity::Normal => Some("nonunique".to_string()),
        _ => None,
    };
    // Override: "any" removes the rarity filter entirely.
    let rarity_option = match filter_config.and_then(|fc| fc.rarity_override.as_deref()) {
        Some("any") => None,
        Some(other) => Some(other.to_string()),
        None => default_rarity,
    };
    let rarity = rarity_option.map(|opt| OptionFilter { option: opt });

    // Category filter: always set for Exact/Category modes, omitted for Any.
    let category = match type_scope {
        TypeSearchScope::Any => None,
        TypeSearchScope::BaseType | TypeSearchScope::ItemClass => {
            poe_data::domain::item_class_trade_category(&item.header.item_class).map(|id| {
                OptionFilter {
                    option: id.to_string(),
                }
            })
        }
    };

    if rarity.is_none() && category.is_none() {
        return None;
    }

    Some(TypeFilters {
        filters: TypeFilterValues { category, rarity },
    })
}

fn build_misc_filters(
    item: &ResolvedItem,
    _config: &TradeQueryConfig,
    quality: Option<u32>,
    filter_config: Option<&TradeFilterConfig>,
) -> Option<MiscFilters> {
    // Item level: only when user explicitly enables in edit mode.
    let ilvl = match filter_config {
        Some(fc) if fc.ilvl_enabled => {
            let min = fc.ilvl_min.or(item.item_level).map(f64::from);
            min.map(|m| FilterValue {
                min: Some(m),
                max: None,
            })
        }
        _ => None,
    };

    // Corrupted: default = include if item is corrupted. Override can force on/off.
    let corrupted = match filter_config.and_then(|fc| fc.corrupted_override) {
        Some(true) => Some(OptionFilter {
            option: "true".to_string(),
        }),
        Some(false) => None,
        None => {
            if item.is_corrupted {
                Some(OptionFilter {
                    option: "true".to_string(),
                })
            } else {
                None
            }
        }
    };

    let identified = if item.is_unidentified {
        Some(OptionFilter {
            option: "false".to_string(),
        })
    } else {
        None
    };

    // Fractured: default = include if item is fractured. Override can force on/off.
    let fractured_item = match filter_config.and_then(|fc| fc.fractured_override) {
        Some(true) => Some(OptionFilter {
            option: "true".to_string(),
        }),
        Some(false) => None,
        None => {
            if item.is_fractured {
                Some(OptionFilter {
                    option: "true".to_string(),
                })
            } else {
                None
            }
        }
    };

    // Quality filter: only included when user explicitly enables it in edit mode.
    let quality_filter = match filter_config {
        Some(fc) if fc.quality_enabled => {
            let min = fc.quality_min.or(quality).map(f64::from);
            min.map(|m| FilterValue {
                min: Some(m),
                max: None,
            })
        }
        _ => None,
    };

    // Mutated (Foulborn): auto-include if item name has a league mechanic prefix.
    let mutated = item
        .header
        .name
        .as_deref()
        .filter(|n| poe_data::domain::has_league_prefix(n))
        .map(|_| OptionFilter {
            option: "true".to_string(),
        });

    if ilvl.is_none()
        && corrupted.is_none()
        && identified.is_none()
        && fractured_item.is_none()
        && quality_filter.is_none()
        && mutated.is_none()
    {
        return None;
    }

    Some(MiscFilters {
        filters: MiscFilterValues {
            ilvl,
            corrupted,
            identified,
            fractured_item,
            quality: quality_filter,
            mutated,
        },
    })
}

/// Extract the numeric quality value from an item's properties.
///
/// Public for use by `filter_schema::trade_edit_schema()`.
///
/// Looks for a property named `"Quality"` and parses its value
/// (e.g., `"+26%"` → `26`, `"+20% (augmented)"` → `20`).
pub fn extract_quality(item: &ResolvedItem) -> Option<u32> {
    item.properties.iter().find_map(|p| {
        if p.name == "Quality" {
            // Strip leading +, trailing %, and any " (augmented)" suffix.
            let num_str = p.value.trim_start_matches('+').split('%').next()?.trim();
            num_str.parse().ok()
        } else {
            None
        }
    })
}

/// Build socket filters (links).
///
/// Default behavior (no filter config): include a min-links filter only for
/// 5-link or 6-link items (link count significantly affects price).
///
/// With filter config: respect the user's `min_links_enabled` and `min_links` overrides.
fn build_socket_filters(
    socket_info: Option<&SocketInfo>,
    filter_config: Option<&TradeFilterConfig>,
) -> Option<SocketFilters> {
    let info = socket_info?;

    let (enabled, min_links) = match filter_config {
        Some(fc) => (fc.min_links_enabled, fc.min_links.unwrap_or(info.max_link)),
        None => {
            // Auto: only include for 5L+ items
            if info.max_link >= 5 {
                (true, info.max_link)
            } else {
                (false, 0)
            }
        }
    };

    if !enabled || min_links == 0 {
        return None;
    }

    Some(SocketFilters {
        filters: SocketFilterValues {
            links: Some(IntFilterValue {
                min: Some(min_links),
                max: None,
            }),
        },
    })
}

/// Build weapon DPS filters from computed DPS pseudo mods.
///
/// Maps DPS pseudo stat IDs → trade API `weapon_filters` keys (pdps/edps/dps).
/// Returns `None` for non-weapons or items with no DPS pseudos.
fn build_weapon_filters(item: &ResolvedItem, config: &TradeQueryConfig) -> Option<WeaponFilters> {
    let mut pdps = None;
    let mut edps = None;
    let mut dps = None;

    for pseudo in &item.pseudo_mods {
        for sl in &pseudo.stat_lines {
            let Some(stat_ids) = &sl.stat_ids else {
                continue;
            };
            let Some(pseudo_id) = stat_ids.first() else {
                continue;
            };
            let Some(filter_key) = poe_data::domain::dps_weapon_filter(pseudo_id) else {
                continue;
            };
            if sl.values.is_empty() {
                continue;
            }

            let raw_value = sl.values[0].current as f64;
            let relaxed = (raw_value * config.value_relaxation).floor();
            let fv = FilterValue {
                min: Some(relaxed),
                max: None,
            };

            match filter_key {
                "pdps" => pdps = Some(fv),
                "edps" => edps = Some(fv),
                "dps" => dps = Some(fv),
                _ => {}
            }
        }
    }

    if pdps.is_none() && edps.is_none() && dps.is_none() {
        return None;
    }

    Some(WeaponFilters {
        filters: WeaponFilterValues { pdps, edps, dps },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mod_category_prefix_suffix() {
        let m = ResolvedMod {
            header: poe_item::types::ModHeader {
                source: poe_item::types::ModSource::Regular,
                slot: poe_item::types::ModSlot::Prefix,
                influence_tier: None,
                name: None,
                tier: None,
                tags: vec![],
            },
            stat_lines: vec![],
            is_fractured: false,
            display_type: ModDisplayType::Prefix,
        };
        assert_eq!(mod_trade_category(&m), "explicit");
    }

    #[test]
    fn mod_category_fractured_overrides() {
        let m = ResolvedMod {
            header: poe_item::types::ModHeader {
                source: poe_item::types::ModSource::Regular,
                slot: poe_item::types::ModSlot::Prefix,
                influence_tier: None,
                name: None,
                tier: None,
                tags: vec![],
            },
            stat_lines: vec![],
            is_fractured: true,
            display_type: ModDisplayType::Prefix,
        };
        assert_eq!(mod_trade_category(&m), "fractured");
    }

    #[test]
    fn mod_category_crafted() {
        let m = ResolvedMod {
            header: poe_item::types::ModHeader {
                source: poe_item::types::ModSource::MasterCrafted,
                slot: poe_item::types::ModSlot::Prefix,
                influence_tier: None,
                name: None,
                tier: None,
                tags: vec![],
            },
            stat_lines: vec![],
            is_fractured: false,
            display_type: ModDisplayType::Crafted,
        };
        assert_eq!(mod_trade_category(&m), "crafted");
    }

    #[test]
    fn relaxation_positive_value() {
        let config = TradeQueryConfig::new("Mirage");
        let stat_line = ResolvedStatLine {
            raw_text: String::new(),
            display_text: String::new(),
            values: vec![poe_item::types::ValueRange {
                current: 139,
                min: 130,
                max: 144,
            }],
            stat_ids: None,
            stat_values: None,
            is_reminder: false,
            is_unscalable: false,
        };
        let fv = compute_filter_value(&stat_line, &config).unwrap();
        // 139 * 0.85 = 118.15 → floor = 118
        assert_eq!(fv.min, Some(118.0));
        assert_eq!(fv.max, None);
    }

    #[test]
    fn relaxation_negative_value() {
        let config = TradeQueryConfig::new("Mirage");
        let stat_line = ResolvedStatLine {
            raw_text: String::new(),
            display_text: String::new(),
            values: vec![poe_item::types::ValueRange {
                current: -30,
                min: -30,
                max: -30,
            }],
            stat_ids: None,
            stat_values: None,
            is_reminder: false,
            is_unscalable: false,
        };
        let fv = compute_filter_value(&stat_line, &config).unwrap();
        // -30 * (2 - 0.85) = -30 * 1.15 = -34.5 → floor = -35
        assert_eq!(fv.min, Some(-35.0));
        assert_eq!(fv.max, None);
    }

    #[test]
    fn relaxation_multi_value_averages() {
        let config = TradeQueryConfig::new("Mirage");
        let stat_line = ResolvedStatLine {
            raw_text: String::new(),
            display_text: String::new(),
            values: vec![
                poe_item::types::ValueRange {
                    current: 11,
                    min: 11,
                    max: 15,
                },
                poe_item::types::ValueRange {
                    current: 24,
                    min: 23,
                    max: 26,
                },
            ],
            stat_ids: None,
            stat_values: None,
            is_reminder: false,
            is_unscalable: false,
        };
        let fv = compute_filter_value(&stat_line, &config).unwrap();
        // average = (11 + 24) / 2 = 17.5, relaxed = 17.5 * 0.85 = 14.875 → floor = 14
        assert_eq!(fv.min, Some(14.0));
    }

    #[test]
    fn boolean_stat_no_value() {
        let config = TradeQueryConfig::new("Mirage");
        let stat_line = ResolvedStatLine {
            raw_text: "Hits can't be Evaded".to_string(),
            display_text: "Hits can't be Evaded".to_string(),
            values: vec![],
            stat_ids: None,
            stat_values: None,
            is_reminder: false,
            is_unscalable: false,
        };
        assert!(compute_filter_value(&stat_line, &config).is_none());
    }

    // ── Filter override tests ──────────────────────────────────────────────

    use crate::types::{StatFilterOverride, TradeFilterConfig};

    /// Build a minimal item for filter tests.
    fn test_item() -> ResolvedItem {
        use poe_item::types::*;
        ResolvedItem {
            header: ResolvedHeader {
                rarity: Rarity::Rare,
                name: Some("Test Item".to_string()),
                base_type: "Demon's Horn".to_string(),
                item_class: "Wands".to_string(),
            },
            properties: vec![],
            requirements: vec![],
            sockets: None,
            socket_info: None,
            quality: None,
            item_level: Some(83),
            enchants: vec![],
            implicits: vec![],
            explicits: vec![
                ResolvedMod {
                    header: ModHeader {
                        source: ModSource::Regular,
                        slot: ModSlot::Prefix,
                        influence_tier: None,
                        name: None,
                        tier: None,
                        tags: vec![],
                    },
                    stat_lines: vec![ResolvedStatLine {
                        raw_text: "+100 to maximum Life".to_string(),
                        display_text: "+100 to maximum Life".to_string(),
                        values: vec![ValueRange {
                            current: 100,
                            min: 80,
                            max: 109,
                        }],
                        stat_ids: Some(vec!["base_maximum_life".to_string()]),
                        stat_values: None,
                        is_reminder: false,
                        is_unscalable: false,
                    }],
                    is_fractured: false,
                    display_type: ModDisplayType::Prefix,
                },
                ResolvedMod {
                    header: ModHeader {
                        source: ModSource::Regular,
                        slot: ModSlot::Suffix,
                        influence_tier: None,
                        name: None,
                        tier: None,
                        tags: vec![],
                    },
                    stat_lines: vec![ResolvedStatLine {
                        raw_text: "+40% to Cold Resistance".to_string(),
                        display_text: "+40% to Cold Resistance".to_string(),
                        values: vec![ValueRange {
                            current: 40,
                            min: 36,
                            max: 41,
                        }],
                        stat_ids: Some(vec!["base_cold_damage_resistance_pct".to_string()]),
                        stat_values: None,
                        is_reminder: false,
                        is_unscalable: false,
                    }],
                    is_fractured: false,
                    display_type: ModDisplayType::Suffix,
                },
            ],
            gem_data: None,
            influences: vec![],
            statuses: vec![],
            description: None,
            flavor_text: None,
            is_corrupted: false,
            is_unidentified: false,
            is_fractured: false,
            monster_level: None,
            talisman_tier: None,
            experience: None,
            note: None,
            pseudo_mods: vec![],
            unclassified_sections: vec![],
        }
    }

    /// Build a minimal trade stats index that maps our test stat IDs.
    fn test_index() -> TradeStatsIndex {
        use std::collections::HashMap;
        let mut ggpk_to_trade = HashMap::new();
        ggpk_to_trade.insert("base_maximum_life".to_string(), 3_299_347_043_u64);
        ggpk_to_trade.insert(
            "base_cold_damage_resistance_pct".to_string(),
            4_220_027_924_u64,
        );
        TradeStatsIndex {
            by_template: HashMap::new(),
            by_trade_id: HashMap::new(),
            ggpk_to_trade,
            trade_to_ggpk: HashMap::new(),
        }
    }

    #[test]
    fn none_filter_config_includes_all_stats() {
        let item = test_item();
        let index = test_index();
        let config = TradeQueryConfig::new("Mirage");
        let result = build_query(&item, &index, &config, None);

        assert_eq!(result.stats_mapped, 2);
        assert_eq!(result.mapped_stats.len(), 2);
        assert!(result.mapped_stats[0].included);
        assert!(result.mapped_stats[1].included);
        assert!(result.mapped_stats[0].trade_id.is_some());
        assert!(result.mapped_stats[1].trade_id.is_some());
    }

    #[test]
    fn filter_disables_stat() {
        let item = test_item();
        let index = test_index();
        let config = TradeQueryConfig::new("Mirage");
        let fc = TradeFilterConfig {
            type_scope: TypeSearchScope::BaseType,
            stat_overrides: vec![
                StatFilterOverride {
                    stat_index: 0,
                    enabled: true,
                    min_override: None,
                    max_override: None,
                },
                StatFilterOverride {
                    stat_index: 1,
                    enabled: false,
                    min_override: None,
                    max_override: None,
                },
            ],
            min_links_enabled: false,
            min_links: None,
            quality_enabled: false,
            quality_min: None,
            ..TradeFilterConfig::default()
        };
        let result = build_query(&item, &index, &config, Some(&fc));

        assert_eq!(
            result.stats_mapped, 1,
            "only one stat should be in the query"
        );
        assert!(result.mapped_stats[0].included);
        assert!(!result.mapped_stats[1].included);
        // The query should have 1 stat filter
        assert_eq!(result.body.query.stats[0].filters.len(), 1);
    }

    #[test]
    fn filter_overrides_min_value() {
        let item = test_item();
        let index = test_index();
        let config = TradeQueryConfig::new("Mirage");
        let fc = TradeFilterConfig {
            type_scope: TypeSearchScope::BaseType,
            stat_overrides: vec![StatFilterOverride {
                stat_index: 0,
                enabled: true,
                min_override: Some(50.0),
                max_override: None,
            }],
            min_links_enabled: false,
            min_links: None,
            quality_enabled: false,
            quality_min: None,
            ..TradeFilterConfig::default()
        };
        let result = build_query(&item, &index, &config, Some(&fc));

        let filter = &result.body.query.stats[0].filters[0];
        assert_eq!(filter.value.as_ref().unwrap().min, Some(50.0));
    }

    #[test]
    fn type_scope_base_type() {
        let item = test_item();
        let index = test_index();
        let config = TradeQueryConfig::new("Mirage");
        let fc = TradeFilterConfig {
            type_scope: TypeSearchScope::BaseType,
            stat_overrides: vec![],
            min_links_enabled: false,
            min_links: None,
            quality_enabled: false,
            quality_min: None,
            ..TradeFilterConfig::default()
        };
        let result = build_query(&item, &index, &config, Some(&fc));

        assert_eq!(result.body.query.base_type.as_deref(), Some("Demon's Horn"));
        let cat = result
            .body
            .query
            .filters
            .as_ref()
            .and_then(|f| f.type_filters.as_ref())
            .and_then(|tf| tf.filters.category.as_ref());
        assert_eq!(cat.unwrap().option, "weapon.wand");
    }

    #[test]
    fn type_scope_item_class_omits_base() {
        let item = test_item();
        let index = test_index();
        let config = TradeQueryConfig::new("Mirage");
        let fc = TradeFilterConfig {
            type_scope: TypeSearchScope::ItemClass,
            stat_overrides: vec![],
            min_links_enabled: false,
            min_links: None,
            quality_enabled: false,
            quality_min: None,
            ..TradeFilterConfig::default()
        };
        let result = build_query(&item, &index, &config, Some(&fc));

        // Base type should be omitted
        assert!(result.body.query.base_type.is_none());
        // But category filter should still be set
        let cat = result
            .body
            .query
            .filters
            .as_ref()
            .and_then(|f| f.type_filters.as_ref())
            .and_then(|tf| tf.filters.category.as_ref());
        assert_eq!(cat.unwrap().option, "weapon.wand");
    }

    #[test]
    fn type_scope_any_omits_both() {
        let item = test_item();
        let index = test_index();
        let config = TradeQueryConfig::new("Mirage");
        let fc = TradeFilterConfig {
            type_scope: TypeSearchScope::Any,
            stat_overrides: vec![],
            min_links_enabled: false,
            min_links: None,
            quality_enabled: false,
            quality_min: None,
            ..TradeFilterConfig::default()
        };
        let result = build_query(&item, &index, &config, Some(&fc));

        assert!(result.body.query.base_type.is_none());
        // Category should be None, but rarity still set → type_filters exists
        let type_filters = result
            .body
            .query
            .filters
            .as_ref()
            .and_then(|f| f.type_filters.as_ref());
        assert!(type_filters.is_some());
        assert!(type_filters.unwrap().filters.category.is_none());
    }

    #[test]
    fn mapped_stats_have_computed_min() {
        let item = test_item();
        let index = test_index();
        let config = TradeQueryConfig::new("Mirage");
        let result = build_query(&item, &index, &config, None);

        // Life stat: 100 * 0.85 = 85.0
        assert_eq!(result.mapped_stats[0].computed_min, Some(85.0));
        assert_eq!(result.mapped_stats[0].display_text, "+100 to maximum Life");
        // Cold res: 40 * 0.85 = 34.0
        assert_eq!(result.mapped_stats[1].computed_min, Some(34.0));
    }

    // ── Socket filter tests ───────────────────────────────────────────────

    #[test]
    fn no_sockets_no_filter() {
        let item = test_item(); // sockets: None
        let index = test_index();
        let config = TradeQueryConfig::new("Mirage");
        let result = build_query(&item, &index, &config, None);

        assert!(result.socket_info.is_none());
        // No socket filters in query
        let sf = result
            .body
            .query
            .filters
            .as_ref()
            .and_then(|f| f.socket_filters.as_ref());
        assert!(sf.is_none());
    }

    #[test]
    fn five_link_auto_includes_link_filter() {
        let mut item = test_item();
        item.sockets = Some("R-R-G-G-B".to_string());
        item.socket_info = Some(SocketInfo {
            total: 5,
            max_link: 5,
            red: 2,
            green: 2,
            blue: 1,
            white: 0,
        });
        let index = test_index();
        let config = TradeQueryConfig::new("Mirage");
        let result = build_query(&item, &index, &config, None);

        let info = result.socket_info.as_ref().unwrap();
        assert_eq!(info.total, 5);
        assert_eq!(info.max_link, 5);

        let sf = result
            .body
            .query
            .filters
            .as_ref()
            .and_then(|f| f.socket_filters.as_ref())
            .unwrap();
        assert_eq!(sf.filters.links.as_ref().unwrap().min, Some(5));
    }

    #[test]
    fn four_link_no_auto_filter() {
        let mut item = test_item();
        item.sockets = Some("R-R-G-G B".to_string());
        item.socket_info = Some(SocketInfo {
            total: 5,
            max_link: 4,
            red: 2,
            green: 2,
            blue: 1,
            white: 0,
        });
        let index = test_index();
        let config = TradeQueryConfig::new("Mirage");
        let result = build_query(&item, &index, &config, None);

        let info = result.socket_info.as_ref().unwrap();
        assert_eq!(info.max_link, 4);

        // No auto link filter for < 5L
        let sf = result
            .body
            .query
            .filters
            .as_ref()
            .and_then(|f| f.socket_filters.as_ref());
        assert!(sf.is_none());
    }

    #[test]
    fn edit_mode_enables_link_filter() {
        let mut item = test_item();
        item.sockets = Some("R-R-G B".to_string()); // 3-link
        item.socket_info = Some(SocketInfo {
            total: 4,
            max_link: 3,
            red: 2,
            green: 1,
            blue: 1,
            white: 0,
        });
        let index = test_index();
        let config = TradeQueryConfig::new("Mirage");
        let fc = TradeFilterConfig {
            type_scope: TypeSearchScope::BaseType,
            stat_overrides: vec![],
            min_links_enabled: true,
            min_links: Some(3),
            quality_enabled: false,
            quality_min: None,
            ..TradeFilterConfig::default()
        };
        let result = build_query(&item, &index, &config, Some(&fc));

        let sf = result
            .body
            .query
            .filters
            .as_ref()
            .and_then(|f| f.socket_filters.as_ref())
            .unwrap();
        assert_eq!(sf.filters.links.as_ref().unwrap().min, Some(3));
    }

    #[test]
    fn edit_mode_disables_link_filter() {
        let mut item = test_item();
        item.sockets = Some("R-R-G-G-B-B".to_string()); // 6-link
        item.socket_info = Some(SocketInfo {
            total: 6,
            max_link: 6,
            red: 2,
            green: 2,
            blue: 2,
            white: 0,
        });
        let index = test_index();
        let config = TradeQueryConfig::new("Mirage");
        let fc = TradeFilterConfig {
            type_scope: TypeSearchScope::BaseType,
            stat_overrides: vec![],
            min_links_enabled: false,
            min_links: None,
            quality_enabled: false,
            quality_min: None,
            ..TradeFilterConfig::default()
        };
        let result = build_query(&item, &index, &config, Some(&fc));

        // User disabled link filter even though 6L would auto-include
        let sf = result
            .body
            .query
            .filters
            .as_ref()
            .and_then(|f| f.socket_filters.as_ref());
        assert!(sf.is_none());
    }

    // ── Quality tests ─────────────────────────────────────────────────────

    #[test]
    fn extract_quality_from_properties() {
        let mut item = test_item();
        item.properties = vec![poe_item::types::ItemProperty {
            name: "Quality".to_string(),
            value: "+20%".to_string(),
            augmented: true,
            synthetic: false,
        }];
        let index = test_index();
        let config = TradeQueryConfig::new("Mirage");
        let result = build_query(&item, &index, &config, None);

        assert_eq!(result.quality, Some(20));
        // Default: no quality filter in the query
        let qf = result
            .body
            .query
            .filters
            .as_ref()
            .and_then(|f| f.misc_filters.as_ref())
            .and_then(|mf| mf.filters.quality.as_ref());
        assert!(qf.is_none());
    }

    #[test]
    fn edit_mode_enables_quality_filter() {
        let mut item = test_item();
        item.properties = vec![poe_item::types::ItemProperty {
            name: "Quality".to_string(),
            value: "+20%".to_string(),
            augmented: true,
            synthetic: false,
        }];
        let index = test_index();
        let config = TradeQueryConfig::new("Mirage");
        let fc = TradeFilterConfig {
            type_scope: TypeSearchScope::BaseType,
            stat_overrides: vec![],
            min_links_enabled: false,
            min_links: None,
            quality_enabled: true,
            quality_min: None, // use item's actual quality
            ..TradeFilterConfig::default()
        };
        let result = build_query(&item, &index, &config, Some(&fc));

        let qf = result
            .body
            .query
            .filters
            .as_ref()
            .and_then(|f| f.misc_filters.as_ref())
            .and_then(|mf| mf.filters.quality.as_ref())
            .unwrap();
        assert_eq!(qf.min, Some(20.0));
    }

    #[test]
    fn edit_mode_quality_custom_min() {
        let mut item = test_item();
        item.properties = vec![poe_item::types::ItemProperty {
            name: "Quality".to_string(),
            value: "+20%".to_string(),
            augmented: true,
            synthetic: false,
        }];
        let index = test_index();
        let config = TradeQueryConfig::new("Mirage");
        let fc = TradeFilterConfig {
            type_scope: TypeSearchScope::BaseType,
            stat_overrides: vec![],
            min_links_enabled: false,
            min_links: None,
            quality_enabled: true,
            quality_min: Some(15),
            ..TradeFilterConfig::default()
        };
        let result = build_query(&item, &index, &config, Some(&fc));

        let qf = result
            .body
            .query
            .filters
            .as_ref()
            .and_then(|f| f.misc_filters.as_ref())
            .and_then(|mf| mf.filters.quality.as_ref())
            .unwrap();
        assert_eq!(qf.min, Some(15.0));
    }

    #[test]
    fn pseudo_mods_mapped_to_trade_ids() {
        use poe_item::types::*;
        let mut item = test_item();
        item.pseudo_mods = vec![
            ResolvedMod {
                header: ModHeader {
                    source: ModSource::Computed,
                    slot: ModSlot::Pseudo,
                    influence_tier: None,
                    name: None,
                    tier: None,
                    tags: vec![],
                },
                stat_lines: vec![ResolvedStatLine {
                    raw_text: "(Pseudo) +# total maximum Life".to_string(),
                    display_text: "(Pseudo) +142 total maximum Life".to_string(),
                    values: vec![ValueRange {
                        current: 142,
                        min: 0,
                        max: 0,
                    }],
                    stat_ids: Some(vec!["pseudo_total_life".to_string()]),
                    stat_values: None,
                    is_reminder: false,
                    is_unscalable: false,
                }],
                is_fractured: false,
                display_type: ModDisplayType::Pseudo,
            },
            ResolvedMod {
                header: ModHeader {
                    source: ModSource::Computed,
                    slot: ModSlot::Pseudo,
                    influence_tier: None,
                    name: None,
                    tier: None,
                    tags: vec![],
                },
                stat_lines: vec![ResolvedStatLine {
                    raw_text: "(Pseudo) +#% total to Fire Resistance".to_string(),
                    display_text: "(Pseudo) +45% total to Fire Resistance".to_string(),
                    values: vec![ValueRange {
                        current: 45,
                        min: 0,
                        max: 0,
                    }],
                    stat_ids: Some(vec!["pseudo_total_fire_resistance".to_string()]),
                    stat_values: None,
                    is_reminder: false,
                    is_unscalable: false,
                }],
                is_fractured: false,
                display_type: ModDisplayType::Pseudo,
            },
        ];

        let index = test_index();
        let config = TradeQueryConfig::new("Mirage");
        let result = build_query(&item, &index, &config, None);

        // 2 explicit + 2 pseudo = 4 total stats
        assert_eq!(result.stats_total, 4);
        // Life explicit excluded (covered by pseudo_total_life at 1.0 multiplier).
        // Cold res explicit included (stat_id doesn't match pseudo component).
        // 2 pseudos included. Total mapped = 3.
        assert_eq!(result.stats_mapped, 3);

        // Check pseudo trade IDs are correctly formatted
        let stat_group = &result.body.query.stats[0];
        let trade_ids: Vec<&str> = stat_group.filters.iter().map(|f| f.id.as_str()).collect();
        assert!(
            trade_ids.contains(&"pseudo.pseudo_total_life"),
            "expected pseudo.pseudo_total_life in {trade_ids:?}"
        );
        assert!(
            trade_ids.contains(&"pseudo.pseudo_total_fire_resistance"),
            "expected pseudo.pseudo_total_fire_resistance in {trade_ids:?}"
        );
        // Life explicit should NOT be in the query (covered by pseudo)
        assert!(
            !trade_ids.iter().any(|id| id.contains("3299347043")),
            "life explicit should be excluded by pseudo coverage"
        );
    }

    // ── Auto-selection tests ─────────────────────────────────────────────

    #[test]
    fn stat_priority_ordering() {
        use poe_item::types::*;

        let pseudo = stat_priority(ModDisplayType::Pseudo, false, None);
        let fractured = stat_priority(ModDisplayType::Prefix, true, None);
        let t1 = stat_priority(ModDisplayType::Prefix, false, Some(&ModTierKind::Tier(1)));
        let t3 = stat_priority(ModDisplayType::Suffix, false, Some(&ModTierKind::Tier(3)));
        let no_tier = stat_priority(ModDisplayType::Prefix, false, None);
        let enchant = stat_priority(ModDisplayType::Enchant, false, None);
        let implicit = stat_priority(ModDisplayType::Implicit, false, None);
        let crafted = stat_priority(ModDisplayType::Crafted, false, None);

        assert!(pseudo < fractured, "pseudo should beat fractured");
        assert!(fractured < t1, "fractured should beat T1");
        assert!(t1 < t3, "T1 should beat T3");
        assert!(t3 < no_tier, "T3 should beat no-tier");
        assert!(no_tier < enchant, "explicit should beat enchant");
        assert!(enchant < implicit, "enchant should beat implicit");
        assert!(implicit < crafted, "implicit should beat crafted");
    }

    #[test]
    fn max_stat_filters_caps_auto_selection() {
        let index = test_index();
        let mut config = TradeQueryConfig::new("Mirage");
        config.search_defaults.max_stat_filters = 1;
        config.search_defaults.prefer_pseudos = false;

        let item = test_item();
        let result = build_query(&item, &index, &config, None);

        // With cap=1 and prefer_pseudos=false, only the highest-priority stat is included.
        assert_eq!(result.stats_mapped, 1);
    }

    #[test]
    fn prefer_pseudos_off_includes_all_mappable() {
        use poe_item::types::*;
        let mut item = test_item();
        item.pseudo_mods = vec![ResolvedMod {
            header: ModHeader {
                source: ModSource::Computed,
                slot: ModSlot::Pseudo,
                influence_tier: None,
                name: None,
                tier: None,
                tags: vec![],
            },
            stat_lines: vec![ResolvedStatLine {
                raw_text: "(Pseudo) +# total maximum Life".to_string(),
                display_text: "(Pseudo) +142 total maximum Life".to_string(),
                values: vec![ValueRange {
                    current: 142,
                    min: 0,
                    max: 0,
                }],
                stat_ids: Some(vec!["pseudo_total_life".to_string()]),
                stat_values: None,
                is_reminder: false,
                is_unscalable: false,
            }],
            is_fractured: false,
            display_type: ModDisplayType::Pseudo,
        }];

        let index = test_index();
        let mut config = TradeQueryConfig::new("Mirage");
        config.search_defaults.prefer_pseudos = false;
        config.search_defaults.max_stat_filters = 99;

        let result = build_query(&item, &index, &config, None);

        // With prefer_pseudos=false: life explicit NOT excluded, so all 3 map
        assert_eq!(result.stats_mapped, 3);
    }

    #[test]
    fn crafted_excluded_by_default() {
        use poe_item::types::*;
        let mut item = test_item();
        // Replace second explicit with a crafted mod
        item.explicits[1] = ResolvedMod {
            header: ModHeader {
                source: ModSource::MasterCrafted,
                slot: ModSlot::Suffix,
                influence_tier: None,
                name: None,
                tier: None,
                tags: vec![],
            },
            stat_lines: vec![ResolvedStatLine {
                raw_text: "+40% to Cold Resistance".to_string(),
                display_text: "+40% to Cold Resistance".to_string(),
                values: vec![ValueRange {
                    current: 40,
                    min: 36,
                    max: 41,
                }],
                stat_ids: Some(vec!["base_cold_damage_resistance_pct".to_string()]),
                stat_values: None,
                is_reminder: false,
                is_unscalable: false,
            }],
            is_fractured: false,
            display_type: ModDisplayType::Crafted,
        };

        let index = test_index();
        let mut config = TradeQueryConfig::new("Mirage");
        config.search_defaults.include_crafted = false;
        config.search_defaults.prefer_pseudos = false;
        config.search_defaults.max_stat_filters = 99;

        let result = build_query(&item, &index, &config, None);

        // Crafted mod excluded by default, only life explicit included
        assert_eq!(result.stats_mapped, 1);

        // With crafted enabled, both included
        config.search_defaults.include_crafted = true;
        let result2 = build_query(&item, &index, &config, None);
        assert_eq!(result2.stats_mapped, 2);
    }

    #[test]
    fn user_override_wins_over_auto_selection() {
        use crate::types::StatFilterOverride;
        let index = test_index();
        let mut config = TradeQueryConfig::new("Mirage");
        config.search_defaults.max_stat_filters = 0; // Exclude everything

        let item = test_item();

        // Without overrides: nothing auto-selected
        let result = build_query(&item, &index, &config, None);
        assert_eq!(result.stats_mapped, 0);

        // With user override enabling stat 0: it's included despite cap=0
        let filter_config = TradeFilterConfig {
            stat_overrides: vec![StatFilterOverride {
                stat_index: 0,
                enabled: true,
                min_override: None,
                max_override: None,
            }],
            ..TradeFilterConfig::default()
        };
        let result2 = build_query(&item, &index, &config, Some(&filter_config));
        assert_eq!(result2.stats_mapped, 1);
    }

    #[test]
    fn unique_items_no_auto_stats() {
        use poe_item::types::*;
        let mut item = test_item();
        item.header.rarity = Rarity::Unique;

        let index = test_index();
        let config = TradeQueryConfig::new("Mirage");

        let result = build_query(&item, &index, &config, None);

        // Unique items: name is the filter, no stats auto-selected
        assert_eq!(result.stats_mapped, 0);
        // All stats still reported in mapped_stats for Edit Search UI
        assert_eq!(result.mapped_stats.len(), 2);
    }

    #[test]
    fn tier_threshold_excludes_low_tiers() {
        use poe_item::types::*;
        let mut item = test_item();
        // Set tier info on explicits
        item.explicits[0].header.tier = Some(ModTierKind::Tier(1)); // T1 life
        item.explicits[1].header.tier = Some(ModTierKind::Tier(5)); // T5 cold res

        let index = test_index();
        let mut config = TradeQueryConfig::new("Mirage");
        config.search_defaults.tier_threshold = Some(3); // Only T1-T3
        config.search_defaults.prefer_pseudos = false;
        config.search_defaults.max_stat_filters = 99;

        let result = build_query(&item, &index, &config, None);

        // T5 cold res excluded, only T1 life included
        assert_eq!(result.stats_mapped, 1);
    }
}
