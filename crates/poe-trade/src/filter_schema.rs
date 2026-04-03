//! Trade filter schema: parses GGG's `/data/filters` response and projects
//! items onto the filter space to produce a `TradeEditSchema`.
//!
//! The filter schema is fetched once and cached. Per-item projection
//! (`trade_edit_schema()`) is called on each inspect to determine which
//! filters are applicable and what defaults to use.

use std::collections::HashMap;

use poe_item::types::{Rarity, ResolvedItem};
use serde::{Deserialize, Serialize};

use crate::types::{TradeQueryConfig, TradeStatsIndex, TypeSearchScope};

// ── Raw API response types (deserialization) ────────────────────────────────

/// Raw response from `/api/trade/data/filters`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TradeFiltersResponse {
    pub result: Vec<RawFilterGroup>,
}

/// A group of related filters (e.g., "Miscellaneous", "Weapon Filters").
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RawFilterGroup {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub hidden: bool,
    pub filters: Vec<RawFilterDef>,
}

/// A single filter definition from the API.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RawFilterDef {
    pub id: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub tip: Option<String>,
    #[serde(default)]
    pub min_max: bool,
    #[serde(default)]
    pub option: Option<RawFilterOption>,
    #[serde(default)]
    pub full_span: bool,
    /// Whether this is a socket-type filter (R/G/B/W color inputs + min/max).
    #[serde(default)]
    pub sockets: bool,
}

/// Option definition — either a fixed list of choices or a dynamic autocomplete.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RawFilterOption {
    #[serde(default)]
    pub options: Vec<RawOptionEntry>,
}

/// A single choice within an option filter.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RawOptionEntry {
    /// `null` means "Any" (no filter). Otherwise a string value.
    #[serde(default, deserialize_with = "deserialize_option_id")]
    pub id: Option<String>,
    pub text: String,
}

/// Custom deserializer: normalizes `null`, strings, numbers, booleans → `Option<String>`.
fn deserialize_option_id<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    Ok(value.and_then(|v| match v {
        serde_json::Value::Null => None,
        serde_json::Value::String(s) => Some(s),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        _ => Some(v.to_string()),
    }))
}

// ── Parsed filter index ─────────────────────────────────────────────────────

/// Indexed filter schema, built from the raw API response.
///
/// Analogous to `TradeStatsIndex` for stats.
pub struct FilterIndex {
    groups: Vec<RawFilterGroup>,
    /// Maps `(group_id, filter_id)` → `(group_index, filter_index)`.
    by_id: HashMap<(String, String), (usize, usize)>,
}

impl FilterIndex {
    /// Build from a raw API response.
    #[must_use]
    pub fn from_response(response: &TradeFiltersResponse) -> Self {
        let groups = response.result.clone();
        let mut by_id = HashMap::new();
        for (gi, group) in groups.iter().enumerate() {
            for (fi, filter) in group.filters.iter().enumerate() {
                by_id.insert((group.id.clone(), filter.id.clone()), (gi, fi));
            }
        }
        Self { groups, by_id }
    }

    /// Look up a filter definition by group and filter ID.
    #[must_use]
    pub fn filter_def(&self, group_id: &str, filter_id: &str) -> Option<&RawFilterDef> {
        let &(gi, fi) = self
            .by_id
            .get(&(group_id.to_string(), filter_id.to_string()))?;
        Some(&self.groups[gi].filters[fi])
    }

    /// All filter groups.
    #[must_use]
    pub fn groups(&self) -> &[RawFilterGroup] {
        &self.groups
    }

    /// Total number of filters across all groups.
    #[must_use]
    pub fn filter_count(&self) -> usize {
        self.groups.iter().map(|g| g.filters.len()).sum()
    }

    /// Save the raw response for disk caching.
    ///
    /// # Errors
    ///
    /// Returns IO error if the file cannot be written.
    pub fn save_response(
        response: &TradeFiltersResponse,
        path: &std::path::Path,
    ) -> std::io::Result<()> {
        let file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(file);
        serde_json::to_writer(writer, response).map_err(std::io::Error::other)
    }

    /// Load a cached raw response from disk.
    ///
    /// # Errors
    ///
    /// Returns IO error if the file cannot be read.
    pub fn load_response(path: &std::path::Path) -> std::io::Result<TradeFiltersResponse> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        serde_json::from_reader(reader).map_err(std::io::Error::other)
    }
}

// ── Frontend-facing schema types ────────────────────────────────────────────

/// Complete edit schema for one item — everything the frontend needs to
/// render trade search controls. Computed per-inspect by `trade_edit_schema()`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TradeEditSchema {
    /// Type scope options (base type / item class / any).
    pub type_scope: TypeScopeSchema,
    /// Structural filter groups applicable to this item.
    pub filter_groups: Vec<EditFilterGroup>,
    /// Per-stat filters (inline on mod lines in the overlay).
    pub stats: Vec<TradeStatSchema>,
}

/// Type scope options for the search breadcrumb.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TypeScopeSchema {
    pub current: TypeSearchScope,
    pub options: Vec<TypeScopeOption>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TypeScopeOption {
    pub scope: TypeSearchScope,
    pub label: String,
}

/// A group of filters in the edit schema.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EditFilterGroup {
    pub id: String,
    pub title: String,
    pub filters: Vec<EditFilter>,
}

/// A single filter control in the edit schema.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EditFilter {
    /// Filter ID (e.g., "ilvl", "corrupted"). Matches the trade API query path.
    pub id: String,
    /// API path: `"{group_id}.filters.{id}"` for query construction.
    pub group_id: String,
    /// Human-readable label.
    pub text: String,
    /// Optional tooltip.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tip: Option<String>,
    /// What kind of control to render.
    pub kind: EditFilterKind,
    /// Default value from the item (pre-filled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<EditFilterValue>,
    /// Whether this filter should start enabled.
    pub enabled: bool,
}

/// The kind of UI control for a filter.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum EditFilterKind {
    /// Numeric range (min/max inputs).
    Range,
    /// Dropdown with fixed options.
    Option { options: Vec<EditFilterOption> },
    /// Socket-type filter: per-color inputs (R/G/B/W) + total min/max.
    Sockets,
}

/// A single choice in a dropdown filter.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EditFilterOption {
    /// Value to send (null = "Any" / no filter).
    pub id: Option<String>,
    /// Display text.
    pub text: String,
}

/// A default value for a filter.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum EditFilterValue {
    /// For range filters: a numeric value.
    #[serde(rename_all = "camelCase")]
    Range {
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        min: Option<f64>,
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        max: Option<f64>,
    },
    /// For option filters: the selected option ID.
    #[serde(rename_all = "camelCase")]
    Selected { id: Option<String> },
    /// For socket-type filters: per-color counts + total min/max.
    #[serde(rename_all = "camelCase")]
    Sockets {
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        red: Option<u32>,
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        green: Option<u32>,
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        blue: Option<u32>,
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        white: Option<u32>,
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        min: Option<u32>,
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        max: Option<u32>,
    },
}

/// Per-stat filter for a mod line (inline on the overlay).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TradeStatSchema {
    /// Flat index into the item's non-reminder stat lines.
    pub stat_index: u32,
    /// Full trade stat ID (e.g., `"fractured.stat_809229260"`).
    pub trade_id: Option<String>,
    /// Trade category (e.g., `"fractured"`, `"explicit"`, `"implicit"`).
    pub category: String,
    /// Display text for the stat.
    pub display_text: String,
    /// Relaxation-computed min value.
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub computed_min: Option<f64>,
    /// Whether this stat is included in the search by default.
    pub enabled: bool,
}

// ── Schema projection ───────────────────────────────────────────────────────

/// Build a `TradeEditSchema` for a specific item.
///
/// Projects the item's properties onto the filter schema to determine which
/// filters are applicable and what their defaults should be.
#[must_use]
pub fn trade_edit_schema(
    item: &ResolvedItem,
    filter_index: &FilterIndex,
    stats_index: &TradeStatsIndex,
    config: &TradeQueryConfig,
    game_data: &poe_data::GameData,
) -> TradeEditSchema {
    // Type scope
    let item_class = &item.header.item_class;
    let category_label = poe_data::domain::item_class_trade_category(item_class)
        .unwrap_or("any")
        .to_string();
    let type_scope = TypeScopeSchema {
        current: TypeSearchScope::BaseType,
        options: vec![
            TypeScopeOption {
                scope: TypeSearchScope::BaseType,
                label: item.header.base_type.clone(),
            },
            TypeScopeOption {
                scope: TypeSearchScope::ItemClass,
                label: item_class.clone(),
            },
            TypeScopeOption {
                scope: TypeSearchScope::Any,
                label: "Any".to_string(),
            },
        ],
    };
    // Suppress unused variable warning — category_label reserved for future use
    let _ = category_label;

    // Filter groups
    let mut filter_groups = Vec::new();
    for group in filter_index.groups() {
        // Skip groups not relevant to this item
        if !is_group_relevant(&group.id, item, game_data) {
            continue;
        }
        // Skip status_filters (handled by TradeQueryConfig.listing_status)
        if group.id == "status_filters" || group.id == "trade_filters" {
            continue;
        }
        // Skip type_filters — handled by type_scope breadcrumb
        if group.id == "type_filters" {
            // But extract rarity as a filter
            let mut rarity_filters = Vec::new();
            for f in &group.filters {
                if f.id == "rarity" {
                    if let Some(edit_filter) = build_edit_filter(f, &group.id, item) {
                        rarity_filters.push(edit_filter);
                    }
                }
            }
            if !rarity_filters.is_empty() {
                filter_groups.push(EditFilterGroup {
                    id: group.id.clone(),
                    title: group.title.clone().unwrap_or_default(),
                    filters: rarity_filters,
                });
            }
            continue;
        }

        let mut filters = Vec::new();
        for f in &group.filters {
            if let Some(edit_filter) = build_edit_filter(f, &group.id, item) {
                filters.push(edit_filter);
            }
        }
        if !filters.is_empty() {
            filter_groups.push(EditFilterGroup {
                id: group.id.clone(),
                title: group.title.clone().unwrap_or_default(),
                filters,
            });
        }
    }

    // Stats
    let stats = build_stat_schemas(item, stats_index, config);

    TradeEditSchema {
        type_scope,
        filter_groups,
        stats,
    }
}

/// Convert a raw filter definition into an `EditFilter`, with defaults from the item.
fn build_edit_filter(f: &RawFilterDef, group_id: &str, item: &ResolvedItem) -> Option<EditFilter> {
    let is_range = f.min_max;
    let kind = if f.sockets {
        EditFilterKind::Sockets
    } else if is_range {
        EditFilterKind::Range
    } else if let Some(opt) = &f.option {
        if opt.options.is_empty() {
            // Dynamic autocomplete — skip for now
            return None;
        }
        EditFilterKind::Option {
            options: opt
                .options
                .iter()
                .map(|o| EditFilterOption {
                    id: o.id.clone(),
                    text: o.text.clone(),
                })
                .collect(),
        }
    } else {
        return None;
    };

    let filter_text = f.text.as_deref().unwrap_or("");
    let (default_value, enabled) = filter_default(filter_text, &f.id, is_range, item);

    Some(EditFilter {
        id: f.id.clone(),
        group_id: group_id.to_string(),
        text: filter_text.to_string(),
        tip: f.tip.clone(),
        kind,
        default_value,
        enabled,
    })
}

/// Determine whether a filter group is relevant for this item.
///
/// Uses `ItemClasses` capability flags from GGPK when available,
/// falls back to trade category derivation otherwise.
fn is_group_relevant(group_id: &str, item: &ResolvedItem, game_data: &poe_data::GameData) -> bool {
    let class_name = item.header.item_class.as_str();

    // Use GGPK ItemClasses data when available (for future capability checks)
    let _ic = game_data.item_class_by_name(class_name);

    match group_id {
        "status_filters" | "trade_filters" | "type_filters" | "misc_filters" => true,
        "weapon_filters" => {
            // Weapons have category starting with "weapon." in trade API
            poe_data::domain::item_class_trade_category(class_name)
                .is_some_and(|cat| cat.starts_with("weapon."))
        }
        "armour_filters" => {
            // Armour/shields have category starting with "armour." in trade API
            poe_data::domain::item_class_trade_category(class_name)
                .is_some_and(|cat| cat.starts_with("armour."))
        }
        "socket_filters" => item.socket_info.is_some(),
        "req_filters" => !item.requirements.is_empty(),
        "map_filters" => poe_data::domain::is_map_class(class_name),
        "heist_filters" => poe_data::domain::item_class_trade_category(class_name)
            .is_some_and(|cat| cat.starts_with("heist")),
        "sanctum_filters" => poe_data::domain::item_class_trade_category(class_name)
            .is_some_and(|cat| cat.starts_with("sanctum")),
        // ultimatum_filters: legacy league content
        _ => false,
    }
}

/// Trade API uses shorter property names than GGPK item text.
///
/// Trade API convention, not in GGPK (verified 2026-03-15).
/// `ClientStrings` confirms GGG's item text uses the longer forms:
///   `ItemDisplayArmourEvasionRating` = "Evasion Rating"
///   `ItemDisplayShieldBlockChance` = "Chance to Block"
/// The trade `filters.json` uses these shortened forms.
///
/// TODO: Could be auto-generated by comparing `ClientStrings` `ItemDisplay*`
/// text with trade `filters.json` text at runtime. See `docs/data-driven-plan.md`.
const PROPERTY_ALIASES: &[(&str, &str)] = &[
    ("Evasion", "Evasion Rating"),
    ("Block", "Chance to Block"),
    ("Gem Level", "Level"),
    ("Gem Experience %", "Experience"),
];

/// Trade API requirement filter text → poe-item requirement key.
///
/// Trade API convention, not in GGPK (verified 2026-03-15).
/// Items show short forms (Str, Dex, Int), trade filters use full names.
const REQ_ALIASES: &[(&str, &str)] = &[
    ("Level", "Level"),
    ("Strength", "Str"),
    ("Dexterity", "Dex"),
    ("Intelligence", "Int"),
];

/// Extract a default value and enabled state for a filter based on item data.
///
/// Uses text matching against item properties, statuses, and influences.
/// Only a small exception table handles GGG naming inconsistencies and
/// dedicated fields (`item_level`, sockets, rarity, etc.).
fn filter_default(
    filter_text: &str,
    filter_id: &str,
    is_range: bool,
    item: &ResolvedItem,
) -> (Option<EditFilterValue>, bool) {
    // ── 0. Socket-type filters: populate from SocketInfo ──────────────
    if filter_id == "sockets" || filter_id == "links" {
        if let Some(si) = &item.socket_info {
            if filter_id == "sockets" {
                return (
                    Some(EditFilterValue::Sockets {
                        red: Some(si.red),
                        green: Some(si.green),
                        blue: Some(si.blue),
                        white: Some(si.white),
                        min: Some(si.total),
                        max: None,
                    }),
                    false,
                );
            }
            // links
            let enabled = si.max_link >= 5;
            return (
                Some(EditFilterValue::Sockets {
                    red: None,
                    green: None,
                    blue: None,
                    white: None,
                    min: Some(si.max_link),
                    max: None,
                }),
                enabled,
            );
        }
        return (None, false);
    }

    // ── 1. Exception table: trade API conventions that can't be text-matched ──
    //
    // Only 3 entries remain — everything else is matched by property/status text.
    match filter_id {
        "rarity" => {
            // Trade API uses "nonunique" as a rarity option value — not a property value.
            // Trade API convention, not in GGPK (verified 2026-03-15).
            let default = match item.header.rarity {
                Rarity::Rare | Rarity::Magic | Rarity::Normal => Some("nonunique"),
                _ => None,
            };
            return option_default_selected(default, default.is_some());
        }
        "identified" => {
            // Trade API inverts: filter "Identified" = "No" for unidentified items.
            // Trade API convention, not in GGPK (verified 2026-03-15).
            return if item.is_unidentified {
                option_default_selected(Some("false"), true)
            } else {
                (None, false)
            };
        }
        "gem_vaal" => {
            // "Vaal Gem" is a trade filter concept — no GGPK status line for it.
            // Checked: no ItemPopupVaalGem in ClientStrings (verified 2026-03-15).
            let is = item.gem_data.as_ref().is_some_and(|g| g.vaal.is_some());
            return option_default_bool(is);
        }
        _ => {}
    }

    // ── 2. Property name matching (for range filters) ────────────────

    let prop_name = PROPERTY_ALIASES
        .iter()
        .find(|(filter, _)| *filter == filter_text)
        .map_or(filter_text, |(_, prop)| prop);

    if let Some(prop) = item.properties.iter().find(|p| p.name == prop_name) {
        let val = parse_numeric_property_value(&prop.value);
        if let Some(v) = val {
            // Some filters start enabled by default:
            // - Map Tier / Talisman Tier: primary search criteria
            // - Links >= 5: significant for trade value (5L/6L items)
            let enabled = filter_id == "map_tier"
                || filter_id == "talisman_tier"
                || (filter_id == "links" && v >= 5.0);
            return range_default(Some(v), enabled);
        }
    }

    // ── 3. Status matching (for option filters) ──────────────────────
    //
    // Check if the filter text matches a StatusKind on the item.
    if !is_range {
        let has_status = item
            .statuses
            .iter()
            .any(|s| s.as_item_text() == filter_text);
        if has_status {
            return option_default_bool(true);
        }

        // Check influences (uses "X Item" format from parse text)
        let has_influence = item
            .influences
            .iter()
            .any(|i| i.as_item_text() == filter_text);
        if has_influence {
            return option_default_bool(true);
        }

        // Also check convenience bools for filters that don't match text exactly
        // (e.g., "Fractured Item" matches influence, but is_fractured is the bool)
    }

    // ── 4. Requirement matching ──────────────────────────────────────
    if let Some((_, req_key)) = REQ_ALIASES
        .iter()
        .find(|(filter, _)| *filter == filter_text)
    {
        let val = item
            .requirements
            .iter()
            .find(|r| r.key == *req_key)
            .and_then(|r| r.value.parse::<f64>().ok());
        return range_default(val, false);
    }

    (None, false)
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn range_default(value: Option<f64>, enabled: bool) -> (Option<EditFilterValue>, bool) {
    match value {
        Some(v) => (
            Some(EditFilterValue::Range {
                min: Some(v),
                max: None,
            }),
            enabled,
        ),
        None => (None, false),
    }
}

fn option_default_bool(is_set: bool) -> (Option<EditFilterValue>, bool) {
    if is_set {
        option_default_selected(Some("true"), true)
    } else {
        (None, false)
    }
}

fn option_default_selected(id: Option<&str>, enabled: bool) -> (Option<EditFilterValue>, bool) {
    (
        Some(EditFilterValue::Selected {
            id: id.map(String::from),
        }),
        enabled,
    )
}

/// Parse a numeric value from a property value string, handling `+`, `%`, commas.
fn parse_numeric_property_value(value: &str) -> Option<f64> {
    value
        .replace(['+', '%', ','], "")
        .trim()
        .parse::<f64>()
        .ok()
}

/// Build per-stat schemas from the item's mods.
fn build_stat_schemas(
    item: &ResolvedItem,
    stats_index: &TradeStatsIndex,
    config: &TradeQueryConfig,
) -> Vec<TradeStatSchema> {
    use poe_item::types::ModDisplayType;

    let mut schemas = Vec::new();
    let mut flat_index: u32 = 0;

    let mod_groups: Vec<(&poe_item::types::ResolvedMod, &str)> = item
        .enchants
        .iter()
        .chain(item.implicits.iter())
        .chain(item.explicits.iter())
        .map(|m| {
            let display_type = match m.display_type {
                ModDisplayType::Prefix => "prefix",
                ModDisplayType::Suffix => "suffix",
                ModDisplayType::Implicit => "implicit",
                ModDisplayType::Crafted => "crafted",
                ModDisplayType::Enchant => "enchant",
                ModDisplayType::Unique => "unique",
                ModDisplayType::Pseudo => "pseudo",
            };
            let cat = poe_data::domain::mod_trade_category(display_type, m.is_fractured);
            (m, cat)
        })
        .collect();

    for (m, default_category) in &mod_groups {
        for sl in &m.stat_lines {
            if sl.is_reminder {
                continue;
            }

            let trade_id = sl
                .stat_ids
                .as_ref()
                .and_then(|ids| {
                    ids.iter()
                        .find_map(|sid| stats_index.trade_stat_number(sid))
                })
                .map(|num| format!("{default_category}.stat_{num}"));

            let computed_min = if sl.values.is_empty() {
                None
            } else {
                let raw = if sl.values.len() == 1 {
                    sl.values[0].current as f64
                } else {
                    let sum: f64 = sl.values.iter().map(|v| v.current as f64).sum();
                    sum / sl.values.len() as f64
                };
                let relaxed = if raw >= 0.0 {
                    (raw * config.value_relaxation).floor()
                } else {
                    (raw * (2.0 - config.value_relaxation)).ceil()
                };
                Some(relaxed)
            };

            schemas.push(TradeStatSchema {
                stat_index: flat_index,
                trade_id: trade_id.clone(),
                category: (*default_category).to_string(),
                display_text: sl.display_text.clone(),
                computed_min,
                enabled: trade_id.is_some(),
            });

            flat_index += 1;
        }
    }

    schemas
}
