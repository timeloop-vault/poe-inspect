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
    self, ArmourTypeRow, BaseItemTypeRow, ClientStringRow, ItemClassCategoryRow, ItemClassRow,
    ModFamilyRow, ModRow, ModTypeRow, RarityRow, ShieldTypeRow, StatRow, TagRow, WeaponTypeRow,
};

use crate::domain::{self, LOCAL_STAT_NONLOCAL_FALLBACKS};

/// A resolved pseudo stat definition — families expanded to concrete stat_ids.
#[derive(Debug, Clone)]
pub struct ResolvedPseudo {
    /// Pseudo stat ID (e.g., `"pseudo_total_life"`).
    pub id: &'static str,
    /// Display label template (e.g., `"+# total maximum Life"`).
    pub label: &'static str,
    /// Component stat_ids with multipliers and required flags.
    pub components: Vec<ResolvedPseudoComponent>,
}

/// A single stat_id that contributes to a pseudo stat.
#[derive(Debug, Clone)]
pub struct ResolvedPseudoComponent {
    /// GGPK stat ID (e.g., `"base_maximum_life"`).
    pub stat_id: String,
    /// Multiplier (e.g., 0.5 for Strength → Life).
    pub multiplier: f64,
    /// If true, pseudo only shows when this component has a value.
    pub required: bool,
}

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

    // Mod name → indices into self.mods (multiple tiers share a name).
    mods_by_name: HashMap<String, Vec<usize>>,

    // Reverse index: stat_id → indices into self.mods that contain this stat.
    stat_to_mods: HashMap<String, Vec<usize>>,

    // Reverse mapping: stat_id → display templates (built from reverse_index).
    stat_id_to_templates: HashMap<String, Vec<String>>,

    // Reverse mapping: template → all stat_ids (including local equivalents).
    // Built as the inverse of stat_id_to_templates, so it includes local
    // stat_ids that share templates with their non-local counterparts.
    template_to_all_stat_ids: HashMap<String, Vec<String>>,

    // Stat description reverse index (display text → stat IDs + values).
    // Optional because loading it requires the raw stat_descriptions.txt.
    pub reverse_index: Option<ReverseIndex>,

    // Client strings: GGG's master display text table (ItemPopup*, ItemDisplay*, etc.).
    // Indexed by string ID for O(1) lookup.
    client_string_by_id: HashMap<String, usize>,
    client_strings: Vec<ClientStringRow>,

    // Base item type tables — indexed by base item name for lookup.
    // These map base types to their base defence/weapon/shield stats.
    armour_by_base: HashMap<String, ArmourTypeRow>,
    weapon_by_base: HashMap<String, WeaponTypeRow>,
    shield_by_base: HashMap<String, ShieldTypeRow>,

    // ModFamily name → set of stat_ids (from non-unique mods in that family).
    // Used to resolve pseudo stat definitions to concrete stat_ids.
    family_stat_ids: HashMap<String, HashSet<String>>,

    // Resolved pseudo stat definitions: pseudo_id → Vec<(stat_id, multiplier, required)>.
    // Built at load time by resolving PseudoDefinition families to stat_ids.
    resolved_pseudos: Vec<ResolvedPseudo>,
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

        // Mod name → mod indices (multiple tiers share a display name).
        let mut mods_by_name: HashMap<String, Vec<usize>> = HashMap::new();
        for (mod_idx, m) in mods.iter().enumerate() {
            if !m.name.is_empty() {
                mods_by_name
                    .entry(m.name.clone())
                    .or_default()
                    .push(mod_idx);
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

        // Build family_name → set of stat_ids (from non-unique mods).
        let mut family_stat_ids: HashMap<String, HashSet<String>> = HashMap::new();
        for m in &mods {
            // Skip unique mods (generation_type 3) — they don't represent rollable families
            if m.generation_type == 3 {
                continue;
            }
            for &family_fk in &m.families {
                if let Some(family_row) = mod_families.get(family_fk as usize) {
                    let entry = family_stat_ids
                        .entry(family_row.id.clone())
                        .or_default();
                    for stat_fk in m.stat_keys.iter().flatten() {
                        if let Some(stat_row) = stats.get(*stat_fk as usize) {
                            entry.insert(stat_row.id.clone());
                        }
                    }
                }
            }
        }

        // Resolve pseudo definitions: families → concrete stat_ids
        let resolved_pseudos = domain::pseudo_definitions()
            .iter()
            .map(|def| {
                let mut components = Vec::new();
                for comp in def.components {
                    if let Some(stat_ids) = family_stat_ids.get(comp.family) {
                        for sid in stat_ids {
                            components.push(ResolvedPseudoComponent {
                                stat_id: sid.clone(),
                                multiplier: comp.multiplier,
                                required: comp.required,
                            });
                        }
                    }
                }
                ResolvedPseudo {
                    id: def.id,
                    label: def.label,
                    components,
                }
            })
            .collect();

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
            mods_by_name,
            stat_to_mods,
            stat_id_to_templates: HashMap::new(),
            template_to_all_stat_ids: HashMap::new(),
            reverse_index: None,
            client_string_by_id: HashMap::new(),
            client_strings: Vec::new(),
            armour_by_base: HashMap::new(),
            weapon_by_base: HashMap::new(),
            shield_by_base: HashMap::new(),
            family_stat_ids,
            resolved_pseudos,
        }
    }

    /// Set the client strings table (GGG's master display text).
    ///
    /// Called separately from `new()` because the client strings table
    /// is optional (not needed for core item parsing, but enables
    /// data-driven text validation).
    pub fn set_client_strings(&mut self, strings: Vec<ClientStringRow>) {
        self.client_string_by_id = index_by(&strings, |s| s.id.clone());
        self.client_strings = strings;
    }

    /// Look up a client string by its internal ID.
    ///
    /// E.g., `client_string("ItemPopupCorrupted")` → `Some("Corrupted")`
    #[must_use]
    pub fn client_string(&self, id: &str) -> Option<&str> {
        self.client_string_by_id
            .get(id)
            .map(|&i| self.client_strings[i].text.as_str())
    }

    /// Get all client strings matching a prefix.
    ///
    /// E.g., `client_strings_with_prefix("ItemPopup")` returns all status/influence display texts.
    pub fn client_strings_with_prefix(&self, prefix: &str) -> Vec<(&str, &str)> {
        self.client_strings
            .iter()
            .filter(|s| s.id.starts_with(prefix))
            .map(|s| (s.id.as_str(), s.text.as_str()))
            .collect()
    }

    /// Set base item type tables (armour, weapon, shield stats).
    ///
    /// Resolves FK row indices to base type names for indexed lookup.
    pub fn set_base_type_tables(
        &mut self,
        armour_types: Vec<ArmourTypeRow>,
        weapon_types: Vec<WeaponTypeRow>,
        shield_types: Vec<ShieldTypeRow>,
    ) {
        // Resolve FK → base type name
        for at in armour_types {
            if let Some(base) = self.base_item_types.get(at.base_item as usize) {
                self.armour_by_base.insert(base.name.clone(), at);
            }
        }
        for wt in weapon_types {
            if let Some(base) = self.base_item_types.get(wt.base_item as usize) {
                self.weapon_by_base.insert(base.name.clone(), wt);
            }
        }
        for st in shield_types {
            if let Some(base) = self.base_item_types.get(st.base_item as usize) {
                self.shield_by_base.insert(base.name.clone(), st);
            }
        }
    }

    /// Look up base armour/defence values for a base type name.
    #[must_use]
    pub fn base_armour(&self, base_type: &str) -> Option<&ArmourTypeRow> {
        self.armour_by_base.get(base_type)
    }

    /// Look up base weapon stats for a base type name.
    #[must_use]
    pub fn base_weapon(&self, base_type: &str) -> Option<&WeaponTypeRow> {
        self.weapon_by_base.get(base_type)
    }

    /// Look up base shield block chance for a base type name.
    #[must_use]
    pub fn base_shield_block(&self, base_type: &str) -> Option<i32> {
        self.shield_by_base.get(base_type).map(|s| s.block)
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

        // Build inverse: template → all stat_ids (including local equivalents).
        let mut tmpl_to_ids: HashMap<String, Vec<String>> = HashMap::new();
        for (stat_id, templates) in &map {
            for tmpl in templates {
                let entry = tmpl_to_ids.entry(tmpl.clone()).or_default();
                if !entry.contains(stat_id) {
                    entry.push(stat_id.clone());
                }
            }
        }
        self.template_to_all_stat_ids = tmpl_to_ids;

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

    /// Look up an item class by its display name (e.g., "Body Armours", "Boots").
    ///
    /// This is the name that appears in Ctrl+Alt+C `Item Class:` header.
    pub fn item_class_by_name(&self, name: &str) -> Option<&ItemClassRow> {
        self.item_classes.iter().find(|c| c.name == name)
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

    /// Get all resolved pseudo stat definitions.
    ///
    /// Each definition has its component families resolved to concrete stat_ids.
    /// Used by poe-item's resolver to compute pseudo values on items.
    pub fn pseudo_definitions(&self) -> &[ResolvedPseudo] {
        &self.resolved_pseudos
    }

    /// Get the set of stat_ids associated with a mod family.
    pub fn family_stat_ids(&self, family: &str) -> Option<&HashSet<String>> {
        self.family_stat_ids.get(family)
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

    /// Look up mod indices by display name (for direct GGPK mod table access).
    pub fn mods_by_name_indices(&self, name: &str) -> Option<&[usize]> {
        self.mods_by_name.get(name).map(Vec::as_slice)
    }

    /// Find the mod that is eligible for a given base type and item class.
    ///
    /// Uses the GGPK's tag intersection system: a mod is eligible if any of its
    /// `spawn_weight_tags` overlap with the base type's `tags` AND the weight > 0.
    ///
    /// Filters by mod domain (from `item_class_mod_domains`) to exclude mods
    /// that can't appear on this item type (e.g., monster mods, abyss jewel
    /// mods on equipment).
    ///
    /// When multiple mods share the same display name (e.g., local vs non-local
    /// variants of "Vaporous"), returns the one with the highest total spawn
    /// weight for this base type's tags. This ensures items get the correct
    /// variant (e.g., abyss jewels get `base_evasion_rating` not
    /// `local_base_evasion_rating`).
    pub fn find_eligible_mod(
        &self,
        base_type: &str,
        mod_name: &str,
        item_class: &str,
    ) -> Option<&ModRow> {
        let base = self.base_item_by_name(base_type)?;
        let base_tags: HashSet<u64> = base.tags.iter().copied().collect();
        let mod_indices = self.mods_by_name.get(mod_name)?;
        let valid_domains = crate::domain::item_class_mod_domains(item_class);

        mod_indices
            .iter()
            .filter_map(|&idx| {
                let m = &self.mods[idx];
                if !valid_domains.contains(&m.domain) {
                    return None;
                }
                let total_weight: i32 = m
                    .spawn_weight_tags
                    .iter()
                    .zip(m.spawn_weight_values.iter())
                    .filter(|(tag, weight)| **weight > 0 && base_tags.contains(tag))
                    .map(|(_, weight)| *weight)
                    .sum();
                if total_weight > 0 {
                    Some((m, total_weight))
                } else {
                    None
                }
            })
            .max_by_key(|&(_, weight)| weight)
            .map(|(m, _)| m)
    }

    /// Resolve a mod's `stat_keys` to `stat_id` strings.
    ///
    /// Returns the stat IDs for all non-None `stat_keys` in order.
    pub fn mod_stat_ids(&self, mod_row: &ModRow) -> Vec<String> {
        mod_row
            .stat_keys
            .iter()
            .flatten()
            .filter_map(|&fk| self.stat_id(fk).map(String::from))
            .collect()
    }

    /// Get display templates for a stat ID (e.g., `"base_maximum_life"` → `["+# to maximum Life"]`).
    ///
    /// Requires `set_reverse_index()` to have been called.
    pub fn templates_for_stat(&self, stat_id: &str) -> Option<&[String]> {
        self.stat_id_to_templates.get(stat_id).map(Vec::as_slice)
    }

    /// Get all stat IDs for a display template (including local equivalents).
    ///
    /// E.g., `"+# to maximum Life"` → `["base_maximum_life"]`
    /// E.g., `"# to Armour"` → `["base_physical_damage_reduction_rating", "local_base_physical_damage_reduction_rating"]`
    ///
    /// Requires `set_reverse_index()` to have been called.
    pub fn all_stat_ids_for_template(&self, template: &str) -> Option<&[String]> {
        self.template_to_all_stat_ids
            .get(template)
            .map(Vec::as_slice)
    }

    /// Return all map/area mod templates with their stat IDs.
    ///
    /// Filters the template index for entries where any associated stat ID
    /// starts with `map_`. Used by the map danger assessment settings page.
    ///
    /// Requires `set_reverse_index()` to have been called.
    pub fn map_mod_templates(&self) -> Vec<(&str, &[String])> {
        self.template_to_all_stat_ids
            .iter()
            .filter(|(_, stat_ids)| stat_ids.iter().any(|id| id.starts_with("map_")))
            .map(|(template, stat_ids)| (template.as_str(), stat_ids.as_slice()))
            .collect()
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

            // Use template_to_all_stat_ids which includes local equivalents,
            // falling back to the reverse index if the template map is empty.
            let stat_ids = self
                .template_to_all_stat_ids
                .get(&template)
                .cloned()
                .unwrap_or_else(|| ri.stat_ids_for_template(&template).unwrap_or_default());

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

                    results.push(StatSuggestion {
                        template: template.clone(),
                        stat_ids: stat_ids.clone(),
                        kind: StatSuggestionKind::Hybrid {
                            mod_name: m.name.clone(),
                            generation_type: m.generation_type,
                            other_templates,
                            other_stat_ids,
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
///
/// # Errors
///
/// Returns `LoadError` if any datc64 file is missing or fails to parse.
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

    // Load base type tables if available (optional — enables DPS/defence calculations)
    let armour_types = load_table(dat_dir, "armourtypes", tables::extract_armour_types)
        .unwrap_or_default();
    let weapon_types = load_table(dat_dir, "weapontypes", tables::extract_weapon_types)
        .unwrap_or_default();
    let shield_types = load_table(dat_dir, "shieldtypes", tables::extract_shield_types)
        .unwrap_or_default();
    if !armour_types.is_empty() || !weapon_types.is_empty() || !shield_types.is_empty() {
        tracing::info!(
            armour = armour_types.len(),
            weapon = weapon_types.len(),
            shield = shield_types.len(),
            "Base type tables loaded"
        );
        gd.set_base_type_tables(armour_types, weapon_types, shield_types);
    }

    // Load client strings if available (optional — enables data-driven text validation)
    match load_table(dat_dir, "clientstrings", tables::extract_client_strings) {
        Ok(strings) => {
            tracing::info!(count = strings.len(), "Client strings loaded");
            gd.set_client_strings(strings);
        }
        Err(e) => {
            tracing::debug!("No client strings: {e}");
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

/// Format a `stat_id` as a human-readable display string (last-resort fallback).
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
