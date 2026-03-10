//! Trade query builder: converts a `ResolvedItem` into a trade API search body.
//!
//! Pure logic — no HTTP. Takes item data, the stats index, and user configuration,
//! produces a serializable `TradeSearchBody` ready for POST to
//! `/api/trade/search/{league}`.

use poe_item::types::{ModDisplayType, Rarity, ResolvedItem, ResolvedMod, ResolvedStatLine};
use serde::Serialize;

use crate::types::{
    MappedStat, TradeFilterConfig, TradeQueryConfig, TradeStatsIndex, TypeSearchScope,
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
    pub corrupted: Option<OptionFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identified: Option<OptionFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fractured_item: Option<OptionFilter>,
}

// ── Builder ─────────────────────────────────────────────────────────────────

/// Build a trade search query from a resolved item.
///
/// Maps item mods to trade stat filters using the stats index,
/// applies value relaxation, and sets appropriate item filters.
///
/// When `filter_config` is `None`, uses default behavior (all stats included,
/// exact base type). When `Some`, respects per-stat overrides and base type
/// specificity from the "Edit Search" UI.
pub fn build_query(
    item: &ResolvedItem,
    index: &TradeStatsIndex,
    config: &TradeQueryConfig,
    filter_config: Option<&TradeFilterConfig>,
) -> QueryBuildResult {
    let mut filters = Vec::new();
    let mut stats_mapped = 0u32;
    let mut stats_total = 0u32;
    let mut unmapped_stats = Vec::new();
    let mut mapped_stats = Vec::new();
    let mut flat_index = 0u32;

    // Process all mods (enchants → implicits → explicits).
    for resolved_mod in item.all_mods() {
        let category = mod_trade_category(resolved_mod);

        for stat_line in &resolved_mod.stat_lines {
            if stat_line.is_reminder {
                continue;
            }
            stats_total += 1;

            // Try to map this stat to a trade ID.
            let trade_id = resolve_trade_id(stat_line, category, index);
            let computed_min = compute_filter_value(stat_line, config).and_then(|fv| fv.min);

            // Check user overrides for this stat.
            let user_override = filter_config.and_then(|fc| {
                fc.stat_overrides
                    .iter()
                    .find(|o| o.stat_index == flat_index)
            });

            let enabled = user_override.is_none_or(|o| o.enabled);
            let included = enabled && trade_id.is_some();

            if included {
                if let Some(ref tid) = trade_id {
                    // Use override min if provided, otherwise relaxation-computed.
                    let min_value = user_override.and_then(|o| o.min_override).or(computed_min);

                    let value = min_value.map(|min| FilterValue {
                        min: Some(min),
                        max: None,
                    });

                    filters.push(StatFilter {
                        id: tid.clone(),
                        value,
                        disabled: None,
                    });
                    stats_mapped += 1;
                }
            } else if trade_id.is_none() {
                unmapped_stats.push(stat_line.display_text.clone());
            }

            mapped_stats.push(MappedStat {
                stat_index: flat_index,
                trade_id,
                display_text: stat_line.display_text.clone(),
                computed_min,
                included,
            });

            flat_index += 1;
        }
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

    let type_scope = filter_config.map_or(TypeSearchScope::BaseType, |fc| fc.type_scope);
    let query_filters = build_item_filters(item, config, type_scope);

    let body = TradeSearchBody {
        query: TradeQuery {
            status: if config.online_only {
                Some(StatusFilter {
                    option: "online".to_string(),
                })
            } else {
                None
            },
            name: match item.header.rarity {
                Rarity::Unique => item.header.name.clone(),
                _ => None,
            },
            base_type: match type_scope {
                TypeSearchScope::BaseType => Some(item.header.base_type.clone()),
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
    }
}

/// Construct the trade site URL for a completed search.
#[must_use]
pub fn trade_url(league: &str, search_id: &str) -> String {
    format!("https://www.pathofexile.com/trade/search/{league}/{search_id}")
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
    };
    poe_data::domain::mod_trade_category(display_type, m.is_fractured)
}

/// Resolve a stat line to its full trade stat ID (e.g., `"explicit.stat_3299347043"`).
///
/// Returns `None` if the stat can't be mapped.
fn resolve_trade_id(
    stat_line: &ResolvedStatLine,
    category: &str,
    index: &TradeStatsIndex,
) -> Option<String> {
    let stat_ids = stat_line.stat_ids.as_ref()?;
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

/// Build item-level filters (type, misc).
fn build_item_filters(
    item: &ResolvedItem,
    config: &TradeQueryConfig,
    type_scope: TypeSearchScope,
) -> Option<QueryFilters> {
    let type_filters = build_type_filters(item, type_scope);
    let misc_filters = build_misc_filters(item, config);

    if type_filters.is_none() && misc_filters.is_none() {
        return None;
    }

    Some(QueryFilters {
        type_filters,
        misc_filters,
    })
}

fn build_type_filters(item: &ResolvedItem, type_scope: TypeSearchScope) -> Option<TypeFilters> {
    let rarity = match item.header.rarity {
        Rarity::Rare | Rarity::Magic | Rarity::Normal => Some(OptionFilter {
            option: "nonunique".to_string(),
        }),
        _ => None,
    };

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

fn build_misc_filters(item: &ResolvedItem, _config: &TradeQueryConfig) -> Option<MiscFilters> {
    let corrupted = if item.is_corrupted {
        Some(OptionFilter {
            option: "true".to_string(),
        })
    } else {
        None
    };

    let identified = if item.is_unidentified {
        Some(OptionFilter {
            option: "false".to_string(),
        })
    } else {
        None
    };

    let fractured_item = if item.is_fractured {
        Some(OptionFilter {
            option: "true".to_string(),
        })
    } else {
        None
    };

    if corrupted.is_none() && identified.is_none() && fractured_item.is_none() {
        return None;
    }

    Some(MiscFilters {
        filters: MiscFilterValues {
            corrupted,
            identified,
            fractured_item,
        },
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
            unclassified_sections: vec![],
        }
    }

    /// Build a minimal trade stats index that maps our test stat IDs.
    fn test_index() -> TradeStatsIndex {
        use std::collections::HashMap;
        let mut ggpk_to_trade = HashMap::new();
        ggpk_to_trade.insert("base_maximum_life".to_string(), 3299347043u64);
        ggpk_to_trade.insert("base_cold_damage_resistance_pct".to_string(), 4220027924u64);
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
                },
                StatFilterOverride {
                    stat_index: 1,
                    enabled: false,
                    min_override: None,
                },
            ],
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
            }],
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
}
