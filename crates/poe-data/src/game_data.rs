//! Central game data store with indexed lookup tables.
//!
//! Loads the 7 datc64 tables extracted by poe-dat and builds id-based
//! indexes for fast access. FK row indices can be resolved to strings
//! via helper methods.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use poe_dat::dat_reader::DatFile;
use poe_dat::stat_desc::ReverseIndex;
use poe_dat::tables::{
    self, BaseItemTypeRow, ItemClassCategoryRow, ItemClassRow, ModFamilyRow, ModRow, ModTypeRow,
    RarityRow, StatRow, TagRow,
};

use crate::domain::LOCAL_STAT_NONLOCAL_FALLBACKS;

// ── GameData ────────────────────────────────────────────────────────────────

/// All game data tables with pre-built indexes.
///
/// Intended to be built once and shared via `Arc<GameData>`.
pub struct GameData {
    // Raw tables (poe-dat row structs, no reshaping)
    pub stats: Vec<StatRow>,
    pub tags: Vec<TagRow>,
    pub item_classes: Vec<ItemClassRow>,
    pub item_class_categories: Vec<ItemClassCategoryRow>,
    pub base_item_types: Vec<BaseItemTypeRow>,
    pub mod_families: Vec<ModFamilyRow>,
    pub mod_types: Vec<ModTypeRow>,
    pub mods: Vec<ModRow>,
    pub rarities: Vec<RarityRow>,

    // Indexes: id string → row index in the corresponding Vec
    stat_by_id: HashMap<String, usize>,
    tag_by_id: HashMap<String, usize>,
    mod_by_id: HashMap<String, usize>,
    item_class_by_id: HashMap<String, usize>,
    base_item_by_name: HashMap<String, usize>,
    rarity_by_id: HashMap<String, usize>,
    item_class_category_by_id: HashMap<String, usize>,

    // Pre-computed tier counts: (family_fk, generation_type) → number of tiers
    family_tier_counts: HashMap<(u64, u32), u32>,

    // Reverse index: stat_id → indices into self.mods that contain this stat.
    stat_to_mods: HashMap<String, Vec<usize>>,

    // Reverse mapping: stat_id → display templates (built from reverse_index).
    stat_id_to_templates: HashMap<String, Vec<String>>,

    // Stat description reverse index (display text → stat IDs + values).
    // Optional because loading it requires the raw stat_descriptions.txt.
    pub reverse_index: Option<ReverseIndex>,
}

impl GameData {
    /// Construct `GameData` from pre-loaded table rows.
    ///
    /// Builds all id-based indexes automatically. `reverse_index` is set to `None`;
    /// call `set_reverse_index()` separately if needed.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        stats: Vec<StatRow>,
        tags: Vec<TagRow>,
        item_classes: Vec<ItemClassRow>,
        item_class_categories: Vec<ItemClassCategoryRow>,
        base_item_types: Vec<BaseItemTypeRow>,
        mod_families: Vec<ModFamilyRow>,
        mod_types: Vec<ModTypeRow>,
        mods: Vec<ModRow>,
        rarities: Vec<RarityRow>,
    ) -> Self {
        let stat_by_id = index_by(&stats, |s| s.id.clone());
        let tag_by_id = index_by(&tags, |t| t.id.clone());
        let mod_by_id = index_by(&mods, |m| m.id.clone());
        let item_class_by_id = index_by(&item_classes, |c| c.id.clone());
        let base_item_by_name = index_by(&base_item_types, |b| b.name.clone());
        let rarity_by_id = index_by(&rarities, |r| r.id.clone());
        let item_class_category_by_id = index_by(&item_class_categories, |c| c.id.clone());

        // Pre-compute tier counts per (family, generation_type) group.
        // Mods sharing the same primary family + gen type form a tier chain.
        let mut family_tier_counts: HashMap<(u64, u32), u32> = HashMap::new();
        for m in &mods {
            if let Some(&family_fk) = m.families.first() {
                *family_tier_counts
                    .entry((family_fk, m.generation_type))
                    .or_insert(0) += 1;
            }
        }

        // Reverse index: stat_id → mod indices containing that stat.
        let mut stat_to_mods: HashMap<String, Vec<usize>> = HashMap::new();
        for (mod_idx, m) in mods.iter().enumerate() {
            for stat_fk in m.stat_keys.iter().flatten() {
                if let Some(stat_row) = stats.get(*stat_fk as usize) {
                    stat_to_mods
                        .entry(stat_row.id.clone())
                        .or_default()
                        .push(mod_idx);
                }
            }
        }

        Self {
            stats,
            tags,
            item_classes,
            item_class_categories,
            base_item_types,
            mod_families,
            mod_types,
            mods,
            rarities,
            stat_by_id,
            tag_by_id,
            mod_by_id,
            item_class_by_id,
            base_item_by_name,
            rarity_by_id,
            item_class_category_by_id,
            family_tier_counts,
            stat_to_mods,
            stat_id_to_templates: HashMap::new(),
            reverse_index: None,
        }
    }

    /// Set the stat description reverse index.
    ///
    /// Also builds the `stat_id → templates` reverse mapping used by
    /// `stat_suggestions_for_query()`.
    pub fn set_reverse_index(&mut self, ri: ReverseIndex) {
        // Build stat_id → display templates mapping.
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for template in ri.template_keys() {
            if let Some(stat_ids) = ri.stat_ids_for_template(&template) {
                for sid in stat_ids {
                    map.entry(sid).or_default().push(template.clone());
                }
            }
        }

        // Fallback for local stats: local stats on armour/weapons (flat
        // armour, evasion, energy shield) don't have entries in
        // stat_descriptions.txt — PoE renders them as base properties.
        // For hybrid suggestion display, reuse the non-local template.
        for stat in &self.stats {
            if !stat.is_local || map.contains_key(&stat.id) {
                continue;
            }
            // Try 1: strip "local_" prefix → look up non-local equivalent.
            if let Some(stripped) = stat.id.strip_prefix("local_") {
                if let Some(templates) = map.get(stripped).cloned() {
                    map.insert(stat.id.clone(), templates);
                    continue;
                }
            }
            // Try 2: hardcoded fallbacks for non-obvious mappings.
            for &(local_id, nonlocal_id) in LOCAL_STAT_NONLOCAL_FALLBACKS {
                if stat.id == local_id {
                    if let Some(templates) = map.get(nonlocal_id).cloned() {
                        map.insert(stat.id.clone(), templates);
                    }
                    break;
                }
            }
        }

        self.stat_id_to_templates = map;
        self.reverse_index = Some(ri);
    }

    // ── Lookup by string id ─────────────────────────────────────────────

    pub fn stat(&self, id: &str) -> Option<&StatRow> {
        self.stat_by_id.get(id).map(|&i| &self.stats[i])
    }

    pub fn tag(&self, id: &str) -> Option<&TagRow> {
        self.tag_by_id.get(id).map(|&i| &self.tags[i])
    }

    pub fn mod_by_id(&self, id: &str) -> Option<&ModRow> {
        self.mod_by_id.get(id).map(|&i| &self.mods[i])
    }

    pub fn item_class(&self, id: &str) -> Option<&ItemClassRow> {
        self.item_class_by_id
            .get(id)
            .map(|&i| &self.item_classes[i])
    }

    pub fn base_item_by_name(&self, name: &str) -> Option<&BaseItemTypeRow> {
        self.base_item_by_name
            .get(name)
            .map(|&i| &self.base_item_types[i])
    }

    // ── FK resolution (row index → string) ──────────────────────────────

    /// Resolve a stat FK (row index) to the stat's string ID.
    pub fn stat_id(&self, fk: u64) -> Option<&str> {
        self.stats.get(fk as usize).map(|s| s.id.as_str())
    }

    /// Resolve a tag FK (row index) to the tag's string ID.
    pub fn tag_id(&self, fk: u64) -> Option<&str> {
        self.tags.get(fk as usize).map(|t| t.id.as_str())
    }

    /// Resolve a mod family FK (row index) to the family's string ID.
    pub fn mod_family_id(&self, fk: u64) -> Option<&str> {
        self.mod_families.get(fk as usize).map(|f| f.id.as_str())
    }

    /// Resolve a mod type FK (row index) to the type's display name.
    pub fn mod_type_name(&self, fk: u64) -> Option<&str> {
        self.mod_types.get(fk as usize).map(|t| t.name.as_str())
    }

    /// Resolve an item class FK (row index) to the class row.
    pub fn item_class_by_index(&self, fk: u64) -> Option<&ItemClassRow> {
        self.item_classes.get(fk as usize)
    }

    pub fn rarity(&self, id: &str) -> Option<&RarityRow> {
        self.rarity_by_id.get(id).map(|&i| &self.rarities[i])
    }

    pub fn item_class_category(&self, id: &str) -> Option<&ItemClassCategoryRow> {
        self.item_class_category_by_id
            .get(id)
            .map(|&i| &self.item_class_categories[i])
    }

    /// Resolve an item class category FK (row index) to the category row.
    pub fn item_class_category_by_index(&self, fk: u64) -> Option<&ItemClassCategoryRow> {
        self.item_class_categories.get(fk as usize)
    }

    /// Get the total number of tiers for a mod (by display name).
    ///
    /// Looks up the mod in the Mods table, finds its family + generation type,
    /// then returns how many mods share that family/gen combo (= tier count).
    pub fn tier_count_for_mod(&self, mod_name: &str) -> Option<u32> {
        let mod_row = self.mods.iter().find(|m| m.name == mod_name)?;
        let family_fk = *mod_row.families.first()?;
        self.family_tier_counts
            .get(&(family_fk, mod_row.generation_type))
            .copied()
    }

    /// Get the max prefix count for a given rarity ID (e.g., "Rare" → 3).
    pub fn max_prefixes(&self, rarity_id: &str) -> Option<i32> {
        self.rarity(rarity_id).map(|r| r.max_prefix)
    }

    /// Get the max suffix count for a given rarity ID (e.g., "Rare" → 3).
    pub fn max_suffixes(&self, rarity_id: &str) -> Option<i32> {
        self.rarity(rarity_id).map(|r| r.max_suffix)
    }

    /// Get display templates for a stat ID (e.g., `"base_maximum_life"` → `["+# to maximum Life"]`).
    ///
    /// Requires `set_reverse_index()` to have been called.
    pub fn templates_for_stat(&self, stat_id: &str) -> Option<&[String]> {
        self.stat_id_to_templates.get(stat_id).map(Vec::as_slice)
    }

    /// Return stat suggestions matching a text query, including hybrid mod combos.
    ///
    /// For each matching template, returns a `Single` suggestion. For each stat
    /// in those templates, also returns `Hybrid` suggestions for prefix/suffix
    /// mods that combine that stat with other stats.
    ///
    /// Requires `set_reverse_index()` to have been called.
    pub fn stat_suggestions_for_query(&self, query: &str) -> Vec<StatSuggestion> {
        let Some(ri) = &self.reverse_index else {
            return Vec::new();
        };

        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        // Dedup hybrids by (sorted stat_id combo, generation_type).
        let mut seen_hybrids: HashSet<(String, u32)> = HashSet::new();

        for template in ri.template_keys() {
            if !template.to_lowercase().contains(&query_lower) {
                continue;
            }

            let stat_ids = ri.stat_ids_for_template(&template).unwrap_or_default();

            // Single-stat suggestion (always included).
            results.push(StatSuggestion {
                template: template.clone(),
                stat_ids: stat_ids.clone(),
                kind: StatSuggestionKind::Single,
            });

            // Find hybrid mods containing any of this template's stats.
            let template_stat_set: HashSet<&str> = stat_ids.iter().map(String::as_str).collect();

            for stat_id in &stat_ids {
                let Some(mod_indices) = self.stat_to_mods.get(stat_id.as_str()) else {
                    continue;
                };

                for &mod_idx in mod_indices {
                    let m = &self.mods[mod_idx];

                    // Only rollable affixes with a display name.
                    if m.name.is_empty() {
                        continue;
                    }
                    if m.generation_type != 1 && m.generation_type != 2 {
                        continue;
                    }

                    // Resolve all stat IDs for this mod.
                    let all_stat_ids: Vec<String> = m
                        .stat_keys
                        .iter()
                        .flatten()
                        .filter_map(|&fk| self.stat_id(fk).map(String::from))
                        .collect();

                    // "Other" stats = those not covered by the searched template.
                    let other_stat_ids: Vec<String> = all_stat_ids
                        .iter()
                        .filter(|s| !template_stat_set.contains(s.as_str()))
                        .cloned()
                        .collect();

                    // Not a hybrid if all stats are already in the template.
                    if other_stat_ids.is_empty() {
                        continue;
                    }

                    // Dedup: same stat combo + affix type = same hybrid option.
                    let mut dedup_ids = all_stat_ids.clone();
                    dedup_ids.sort();
                    let dedup_key = (dedup_ids.join(","), m.generation_type);
                    if !seen_hybrids.insert(dedup_key) {
                        continue;
                    }

                    let other_templates: Vec<String> = other_stat_ids
                        .iter()
                        .map(|sid| {
                            self.stat_id_to_templates
                                .get(sid)
                                .and_then(|ts| ts.first().cloned())
                                .unwrap_or_else(|| format_stat_id_as_display(sid))
                        })
                        .collect();

                    // Resolve other_stat_ids to canonical reverse index stat_ids.
                    // The Mods table uses local stat_ids (e.g., local_base_physical_damage_reduction_rating)
                    // but the reverse index (and item resolver) uses non-local equivalents
                    // (e.g., base_physical_damage_reduction_rating). Map through templates
                    // to get the stat_ids that items will actually have after resolution.
                    let canonical_other_stat_ids: Vec<String> = other_stat_ids
                        .iter()
                        .map(|sid| {
                            self.stat_id_to_templates
                                .get(sid)
                                .and_then(|ts| ts.first())
                                .and_then(|tmpl| ri.stat_ids_for_template(tmpl))
                                .and_then(|ids| ids.first().cloned())
                                .unwrap_or_else(|| sid.clone())
                        })
                        .collect();

                    results.push(StatSuggestion {
                        template: template.clone(),
                        stat_ids: stat_ids.clone(),
                        kind: StatSuggestionKind::Hybrid {
                            mod_name: m.name.clone(),
                            generation_type: m.generation_type,
                            other_templates,
                            other_stat_ids: canonical_other_stat_ids,
                        },
                    });
                }
            }
        }

        results
    }
}

// ── Loading ─────────────────────────────────────────────────────────────────

/// Load `GameData` from a directory of extracted datc64 files.
///
/// Expects files named `{table}.datc64` (lowercase):
/// stats, tags, itemclasses, baseitemtypes, modfamily, modtype, mods.
pub fn load(dat_dir: &Path) -> Result<GameData, LoadError> {
    let stats = load_table(dat_dir, "stats", tables::extract_stats)?;
    let tags = load_table(dat_dir, "tags", tables::extract_tags)?;
    let item_classes = load_table(dat_dir, "itemclasses", tables::extract_item_classes)?;
    let item_class_categories = load_table(
        dat_dir,
        "itemclasscategories",
        tables::extract_item_class_categories,
    )?;
    let base_item_types = load_table(dat_dir, "baseitemtypes", tables::extract_base_item_types)?;
    let mod_families = load_table(dat_dir, "modfamily", tables::extract_mod_families)?;
    let mod_types = load_table(dat_dir, "modtype", tables::extract_mod_types)?;
    let mods = load_table(dat_dir, "mods", tables::extract_mods)?;
    let rarities = load_table(dat_dir, "rarity", tables::extract_rarity)?;

    tracing::info!(
        stats = stats.len(),
        tags = tags.len(),
        item_classes = item_classes.len(),
        item_class_categories = item_class_categories.len(),
        base_items = base_item_types.len(),
        mods = mods.len(),
        rarities = rarities.len(),
        "GameData loaded"
    );

    let mut gd = GameData::new(
        stats,
        tags,
        item_classes,
        item_class_categories,
        base_item_types,
        mod_families,
        mod_types,
        mods,
        rarities,
    );

    // Load reverse index if available
    let ri_path = dat_dir.join("reverse_index.json");
    match ReverseIndex::load(&ri_path) {
        Ok(ri) => {
            tracing::info!(patterns = ri.len(), "Reverse index loaded");
            gd.set_reverse_index(ri);
        }
        Err(e) => {
            tracing::warn!("No reverse index at {}: {e}", ri_path.display());
        }
    }

    Ok(gd)
}

/// Read a single datc64 file and extract typed rows.
fn load_table<T>(
    dir: &Path,
    name: &str,
    extract: fn(&DatFile) -> Vec<T>,
) -> Result<Vec<T>, LoadError> {
    let path = dir.join(format!("{name}.datc64"));
    let bytes = std::fs::read(&path).map_err(|e| LoadError::Io {
        table: name.to_string(),
        source: e,
    })?;
    let dat = DatFile::from_bytes(bytes).map_err(|e| LoadError::Parse {
        table: name.to_string(),
        source: e,
    })?;
    Ok(extract(&dat))
}

/// Build a `HashMap<String, usize>` from a slice, keyed by a string field.
fn index_by<T, F: Fn(&T) -> String>(items: &[T], key_fn: F) -> HashMap<String, usize> {
    items
        .iter()
        .enumerate()
        .map(|(i, item)| (key_fn(item), i))
        .collect()
}

// ── Stat suggestion types ────────────────────────────────────────────────────

/// A stat suggestion for the stat picker, either a single stat or a hybrid mod combo.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StatSuggestion {
    /// The display template that matched the query (e.g., `"+# to maximum Life"`).
    pub template: String,
    /// All stat IDs for this suggestion.
    pub stat_ids: Vec<String>,
    /// Whether this is a single stat or a hybrid mod combo.
    pub kind: StatSuggestionKind,
}

/// Distinguishes single-stat suggestions from hybrid mod combos.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum StatSuggestionKind {
    /// A single stat template (the current behavior).
    Single,
    /// A hybrid mod that combines the searched stat with other stats.
    Hybrid {
        /// Mod display name (e.g., "Urchin's").
        mod_name: String,
        /// Generation type: 1 = prefix, 2 = suffix.
        generation_type: u32,
        /// Display templates for the other stats in the hybrid.
        other_templates: Vec<String>,
        /// Stat IDs for the other stats in the hybrid.
        other_stat_ids: Vec<String>,
    },
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Format a stat_id as a human-readable display string (last-resort fallback).
///
/// Strips common prefixes (`local_`, `base_`), replaces underscores with
/// spaces, and title-cases words. Example:
/// `"local_minimum_added_physical_damage"` → `"Minimum Added Physical Damage"`
fn format_stat_id_as_display(stat_id: &str) -> String {
    let stripped = stat_id
        .strip_prefix("local_")
        .unwrap_or(stat_id)
        .strip_prefix("base_")
        .unwrap_or(stat_id);
    stripped
        .split('_')
        .filter(|w| !w.is_empty() && *w != "+" && *w != "%")
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(c) => {
                    let mut s = c.to_uppercase().to_string();
                    s.extend(chars);
                    s
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("failed to read {table}.datc64")]
    Io {
        table: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse {table}.datc64")]
    Parse {
        table: String,
        #[source]
        source: poe_dat::dat_reader::DatError,
    },
}
