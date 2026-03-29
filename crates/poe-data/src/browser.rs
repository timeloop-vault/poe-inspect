//! Game data browser — mod pool computation, search, and base type details.
//!
//! Supports the item-centric data explorer: search for items, compute
//! available mod pools, and explore what can roll on a given base type.

use std::collections::{HashMap, HashSet};

use crate::GameData;

// ── Search ──────────────────────────────────────────────────────────────────

/// A search result from the universal search.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SearchResult {
    /// Display name (e.g., "Vaal Regalia").
    pub name: String,
    /// Entity kind for routing to the correct view.
    pub kind: SearchResultKind,
    /// Item class display name (e.g., "Body Armours"), if applicable.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub item_class: Option<String>,
    /// Category from `ItemClassCategories` (e.g., "Armour", "Weapons").
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub category: Option<String>,
}

/// What kind of entity a search result represents.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SearchResultKind {
    Equipment,
    Jewel,
    Flask,
    Gem,
    Currency,
    DivinationCard,
    Map,
    Other,
}

// ── Base Type Detail ────────────────────────────────────────────────────────

/// Full detail for a base item type.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BaseTypeDetail {
    pub name: String,
    pub item_class_id: String,
    pub item_class_name: String,
    pub category: String,
    pub drop_level: i32,
    pub width: i32,
    pub height: i32,
    /// Implicit mod display text (resolved from stat descriptions).
    pub implicits: Vec<String>,
    /// Tag IDs on this base type (including inherited).
    pub tags: Vec<String>,
    /// Base defence values (armour/evasion/ES/ward), if applicable.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub defences: Option<BaseDefences>,
    /// Base weapon values, if applicable.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub weapon: Option<BaseWeapon>,
    /// Base shield block chance, if applicable.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub block: Option<i32>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BaseDefences {
    pub armour_min: i32,
    pub armour_max: i32,
    pub evasion_min: i32,
    pub evasion_max: i32,
    pub es_min: i32,
    pub es_max: i32,
    pub ward_min: i32,
    pub ward_max: i32,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BaseWeapon {
    /// Critical strike chance in hundredths (e.g., 500 = 5.00%).
    pub critical: i32,
    /// Attack speed as ms per attack (e.g., 1500 = 1000/1500 APS).
    pub speed: i32,
    pub damage_min: i32,
    pub damage_max: i32,
    pub range: i32,
}

// ── Mod Pool ────────────────────────────────────────────────────────────────

/// Configuration for computing a mod pool.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ModPoolQuery {
    /// Base type name (e.g., "Vaal Regalia").
    pub base_type: String,
    /// Maximum item level for mod eligibility.
    pub item_level: u32,
    /// Generation types to include (1=prefix, 2=suffix).
    /// If empty, includes both.
    #[cfg_attr(feature = "serde", serde(default))]
    pub generation_types: Vec<u32>,
    /// Mod families already taken (mod IDs of slotted mods).
    /// Mods from these families will be marked as unavailable.
    #[cfg_attr(feature = "serde", serde(default))]
    pub taken_mod_ids: Vec<String>,
}

/// The computed mod pool for a base type.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ModPoolResult {
    /// Mod families grouped by generation type.
    pub prefixes: Vec<ModFamily>,
    pub suffixes: Vec<ModFamily>,
    /// Total available mod count (not taken, weight > 0).
    pub available_prefix_count: u32,
    pub available_suffix_count: u32,
}

/// A family of mods (different tiers of the same effect).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ModFamily {
    /// Family ID from GGPK (e.g., "`IncreasedLife`").
    pub family_id: String,
    /// All tiers in this family, ordered T1 (best) first.
    pub tiers: Vec<ModTier>,
    /// Whether this family is taken (a mod from it is already slotted).
    pub taken: bool,
}

/// A single mod tier within a family.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ModTier {
    /// Internal mod ID (e.g., "`IncreasedLife8`").
    pub mod_id: String,
    /// Display name (e.g., "Tyrannical").
    pub name: String,
    /// Tier number (1 = best).
    pub tier: u32,
    /// Required item level.
    pub required_level: i32,
    /// Whether this tier is eligible at the current item level.
    pub eligible: bool,
    /// Spawn weight for this base type (0 = cannot appear).
    pub spawn_weight: i32,
    /// Stat lines with display text and value ranges.
    pub stats: Vec<ModTierStat>,
    /// Tags on this mod.
    pub tags: Vec<String>,
}

/// A stat line on a mod tier.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ModTierStat {
    /// Stat ID (e.g., `base_maximum_life`).
    pub stat_id: String,
    /// Raw minimum roll value (GGPK).
    pub min: i32,
    /// Raw maximum roll value (GGPK).
    pub max: i32,
    /// Stat template with `#` placeholder (e.g., `"#% increased Physical Damage"`).
    /// Used as the family header label.
    pub stat_template: String,
    /// Formatted display text for the min value with transforms applied
    /// (e.g., `"170% increased Physical Damage"`). Used in tier rows.
    pub display_text: String,
}

// ── GameData browser methods ────────────────────────────────────────────────

impl GameData {
    /// Search for entities by name query.
    ///
    /// Returns up to `limit` results matching the query (case-insensitive prefix + substring).
    /// Prefix matches are ranked higher than substring matches.
    pub fn browser_search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }
        let q = query.to_lowercase();
        let mut prefix_matches = Vec::new();
        let mut substring_matches = Vec::new();

        for base in &self.base_item_types {
            if base.name.is_empty() {
                continue;
            }
            let name_lower = base.name.to_lowercase();
            let is_prefix = name_lower.starts_with(&q);
            let is_substring = !is_prefix && name_lower.contains(&q);
            if !is_prefix && !is_substring {
                continue;
            }

            let (item_class_name, category, kind) = self.classify_base_type(base);

            let result = SearchResult {
                name: base.name.clone(),
                kind,
                item_class: Some(item_class_name),
                category: Some(category),
            };

            if is_prefix {
                prefix_matches.push(result);
            } else {
                substring_matches.push(result);
            }
        }

        // Sort each group alphabetically, then concat (prefix matches first).
        prefix_matches.sort_by(|a, b| a.name.cmp(&b.name));
        substring_matches.sort_by(|a, b| a.name.cmp(&b.name));
        prefix_matches.extend(substring_matches);
        prefix_matches.truncate(limit);
        prefix_matches
    }

    /// Get full detail for a base item type.
    pub fn browser_base_type_detail(&self, name: &str) -> Option<BaseTypeDetail> {
        let base = self.base_item_by_name(name)?;
        let (item_class_name, category, _) = self.classify_base_type(base);

        let item_class_id = base
            .item_class
            .and_then(|fk| self.item_class_by_index(fk))
            .map_or_else(String::new, |c| c.id.clone());

        // Resolve implicit mod display text.
        let implicits = base
            .implicit_mods
            .iter()
            .filter_map(|&mod_fk| {
                let m = self.mods.get(mod_fk as usize)?;
                Some(self.mod_stat_display_lines(m))
            })
            .flatten()
            .collect();

        let tags = base
            .tags
            .iter()
            .filter_map(|&fk| self.tag_id(fk).map(String::from))
            .collect();

        let defences = self.base_armour(&base.name).map(|a| BaseDefences {
            armour_min: a.armour_min,
            armour_max: a.armour_max,
            evasion_min: a.evasion_min,
            evasion_max: a.evasion_max,
            es_min: a.es_min,
            es_max: a.es_max,
            ward_min: a.ward_min,
            ward_max: a.ward_max,
        });

        let weapon = self.base_weapon(&base.name).map(|w| BaseWeapon {
            critical: w.critical,
            speed: w.speed,
            damage_min: w.damage_min,
            damage_max: w.damage_max,
            range: w.range,
        });

        let block = self.base_shield_block(&base.name);

        Some(BaseTypeDetail {
            name: base.name.clone(),
            item_class_id,
            item_class_name,
            category,
            drop_level: base.drop_level,
            width: base.width,
            height: base.height,
            implicits,
            tags,
            defences,
            weapon,
            block,
        })
    }

    /// Compute the available mod pool for a base type.
    pub fn browser_mod_pool(&self, query: &ModPoolQuery) -> Option<ModPoolResult> {
        let base = self.base_item_by_name(&query.base_type)?;
        let base_tags: HashSet<u64> = base.tags.iter().copied().collect();

        let item_class_id = base
            .item_class
            .and_then(|fk| self.item_class_by_index(fk))
            .map_or("", |c| c.id.as_str());
        let valid_domains = crate::domain::item_class_mod_domains(item_class_id);

        // Determine which generation types to include.
        let gen_types: HashSet<u32> = if query.generation_types.is_empty() {
            [1, 2].into_iter().collect() // prefix + suffix
        } else {
            query.generation_types.iter().copied().collect()
        };

        // Find families of taken mods.
        let taken_families: HashSet<u64> = query
            .taken_mod_ids
            .iter()
            .filter_map(|id| {
                let m = self.mod_by_id(id)?;
                m.families.first().copied()
            })
            .collect();

        // Group key: (family, generation_type, sorted stat_keys).
        // The stat_keys ensure pure mods (e.g., "% increased Physical Damage")
        // and hybrid mods (e.g., "% increased Phys + Accuracy") separate even
        // when they share a mod family.
        let mut family_groups: HashMap<(u64, u32, Vec<u64>), Vec<EligibleMod>> = HashMap::new();

        for m in &self.mods {
            // Filter: must be prefix or suffix.
            if !gen_types.contains(&m.generation_type) {
                continue;
            }
            // Filter: domain must match item class.
            if !valid_domains.contains(&m.domain) {
                continue;
            }
            // Filter: skip bench craft mods (domain 9) — separate crafting section.
            if m.domain == 9 {
                continue;
            }
            // Filter: skip essence-only mods (not rollable via normal crafting).
            if m.is_essence_only {
                continue;
            }
            // Filter: must have a family.
            let Some(&family_fk) = m.families.first() else {
                continue;
            };
            // Filter: must have display name (skip unnamed internal mods).
            if m.name.is_empty() {
                continue;
            }
            // Compute spawn weight for this base type's tags.
            let spawn_weight = compute_spawn_weight(m, &base_tags);
            if spawn_weight <= 0 {
                continue;
            }

            #[allow(clippy::cast_possible_wrap)] // item_level ≤ 100
            let ilvl = query.item_level as i32;
            let eligible = m.level <= ilvl && (m.max_level == 0 || m.max_level >= ilvl);

            let stats = self.extract_mod_tier_stats(m);
            let tags = m
                .tags
                .iter()
                .filter_map(|&fk| self.tag_id(fk).map(String::from))
                .collect();

            // Build stat key signature: sorted list of active stat FKs.
            let mut stat_sig: Vec<u64> = m.stat_keys.iter().flatten().copied().collect();
            stat_sig.sort_unstable();

            family_groups
                .entry((family_fk, m.generation_type, stat_sig))
                .or_default()
                .push(EligibleMod {
                    mod_id: m.id.clone(),
                    name: m.name.clone(),
                    level: m.level,
                    spawn_weight,
                    eligible,
                    stats,
                    tags,
                });
        }

        // Build ModFamily results.
        let mut prefixes = Vec::new();
        let mut suffixes = Vec::new();
        let mut available_prefix_count = 0u32;
        let mut available_suffix_count = 0u32;

        for ((family_fk, gen_type, _stat_sig), mut mods) in family_groups {
            // Sort by level descending (T1 = highest level).
            mods.sort_by(|a, b| b.level.cmp(&a.level).then_with(|| a.mod_id.cmp(&b.mod_id)));

            let family_id = self
                .mod_family_id(family_fk)
                .unwrap_or("unknown")
                .to_string();
            let taken = taken_families.contains(&family_fk);

            let tiers: Vec<ModTier> = mods
                .iter()
                .enumerate()
                .map(|(i, em)| ModTier {
                    mod_id: em.mod_id.clone(),
                    name: em.name.clone(),
                    tier: (i + 1) as u32,
                    required_level: em.level,
                    eligible: em.eligible,
                    spawn_weight: em.spawn_weight,
                    stats: em.stats.clone(),
                    tags: em.tags.clone(),
                })
                .collect();

            let has_eligible = tiers.iter().any(|t| t.eligible);
            if !taken && has_eligible {
                if gen_type == 1 {
                    available_prefix_count += 1;
                } else {
                    available_suffix_count += 1;
                }
            }

            let family = ModFamily {
                family_id,
                tiers,
                taken,
            };

            if gen_type == 1 {
                prefixes.push(family);
            } else {
                suffixes.push(family);
            }
        }

        // Sort families alphabetically by their best tier's name.
        prefixes.sort_by(|a, b| {
            let a_name = a.tiers.first().map_or("", |t| &t.name);
            let b_name = b.tiers.first().map_or("", |t| &t.name);
            a_name.cmp(b_name)
        });
        suffixes.sort_by(|a, b| {
            let a_name = a.tiers.first().map_or("", |t| &t.name);
            let b_name = b.tiers.first().map_or("", |t| &t.name);
            a_name.cmp(b_name)
        });

        Some(ModPoolResult {
            prefixes,
            suffixes,
            available_prefix_count,
            available_suffix_count,
        })
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    /// Classify a base type into a search result kind with resolved class/category names.
    fn classify_base_type(
        &self,
        base: &poe_dat::tables::BaseItemTypeRow,
    ) -> (String, String, SearchResultKind) {
        let item_class = base.item_class.and_then(|fk| self.item_class_by_index(fk));
        let item_class_name = item_class.map_or_else(String::new, |c| c.name.clone());
        let item_class_id = item_class.map_or("", |c| c.id.as_str());

        let category = item_class
            .and_then(|c| c.category)
            .and_then(|fk| self.item_class_category_by_index(fk))
            .map_or_else(String::new, |c| c.text.clone());

        let kind = classify_item_class(item_class_id, &item_class_name);

        (item_class_name, category, kind)
    }

    /// Get display text lines for a mod's stats.
    fn mod_stat_display_lines(&self, m: &poe_dat::tables::ModRow) -> Vec<String> {
        let mut lines = Vec::new();
        for (i, stat_fk) in m.stat_keys.iter().enumerate() {
            let Some(&fk) = stat_fk.as_ref() else {
                continue;
            };
            let (min, max) = m.stat_ranges[i];
            if min == 0 && max == 0 {
                continue;
            }
            if let Some(stat_id) = self.stat_id(fk) {
                // Try to get a display template, fall back to stat_id.
                let template = self
                    .templates_for_stat(stat_id)
                    .and_then(|ts| ts.first())
                    .cloned()
                    .unwrap_or_else(|| stat_id.to_string());
                if min == max {
                    lines.push(template.replace('#', &min.to_string()));
                } else {
                    lines.push(template.replace('#', &format!("({min}-{max})")));
                }
            }
        }
        lines
    }

    /// Extract stat info for a mod tier.
    ///
    /// `display_text` is the fully formatted display text for the min value
    /// (e.g., "170% increased Physical Damage") using forward transforms from
    /// the stat description system. Falls back to the raw stat template if
    /// no reverse index is available.
    fn extract_mod_tier_stats(&self, m: &poe_dat::tables::ModRow) -> Vec<ModTierStat> {
        let mut stats = Vec::new();
        let ri = self.reverse_index.as_ref();
        for (i, stat_fk) in m.stat_keys.iter().enumerate() {
            let Some(&fk) = stat_fk.as_ref() else {
                continue;
            };
            let (min, max) = m.stat_ranges[i];
            if min == 0 && max == 0 {
                continue;
            }
            if let Some(stat_id) = self.stat_id(fk) {
                // Stat template: "#% increased Physical Damage" (with # placeholder).
                let stat_template = self
                    .templates_for_stat(stat_id)
                    .and_then(|ts| ts.first())
                    .cloned()
                    .unwrap_or_else(|| stat_id.to_string());

                // Forward-format for the min value (applies negate etc.).
                let display_text = ri
                    .and_then(|ri| ri.format_stat_values(stat_id, &[i64::from(min)]))
                    .unwrap_or_else(|| stat_template.replace('#', &min.to_string()));

                stats.push(ModTierStat {
                    stat_id: stat_id.to_string(),
                    min,
                    max,
                    stat_template,
                    display_text,
                });
            }
        }
        stats
    }

    /// Get the max prefix/suffix count for a given item class and rarity.
    ///
    /// Delegates to `GameData::max_affixes` which handles jewel overrides.
    pub fn browser_affix_limits(&self, item_class: &str, rarity: &str) -> (i32, i32) {
        let (p, s) = self.max_affixes(item_class, rarity);
        (p.unwrap_or(0), s.unwrap_or(0))
    }
}

// ── Internal helpers ────────────────────────────────────────────────────────

/// Intermediate struct for collecting eligible mods before grouping.
struct EligibleMod {
    mod_id: String,
    name: String,
    level: i32,
    spawn_weight: i32,
    eligible: bool,
    stats: Vec<ModTierStat>,
    tags: Vec<String>,
}

/// Compute the spawn weight for a mod on a base type.
///
/// Walks the mod's `spawn_weight_tags` in order. The first tag that matches
/// the base type's tag set determines the weight. This is how the GGPK
/// spawn weight system works — first match wins.
fn compute_spawn_weight(m: &poe_dat::tables::ModRow, base_tags: &HashSet<u64>) -> i32 {
    for (tag_fk, weight) in m.spawn_weight_tags.iter().zip(&m.spawn_weight_values) {
        if base_tags.contains(tag_fk) {
            return *weight;
        }
    }
    0
}

/// Classify an item class into a search result kind.
///
/// Accepts either the GGPK internal ID (e.g., `BodyArmour`) or the display
/// name (e.g., "Body Armours"). Both forms are checked.
fn classify_item_class(item_class_id: &str, item_class_name: &str) -> SearchResultKind {
    // Check by GGPK internal ID first.
    match item_class_id {
        "Jewel" | "AbyssJewel" => return SearchResultKind::Jewel,
        "LifeFlask" | "ManaFlask" | "HybridFlask" | "UtilityFlask" => {
            return SearchResultKind::Flask;
        }
        "SkillGem" | "SupportGem" => return SearchResultKind::Gem,
        "DivinationCard" => return SearchResultKind::DivinationCard,
        "Map" | "MapFragment" => return SearchResultKind::Map,
        "StackableCurrency" | "DelveStackableSocketableCurrency" | "HideoutDoodad" => {
            return SearchResultKind::Currency;
        }
        _ => {}
    }
    // Fall back to trade category mapping (uses display names).
    if crate::domain::item_class_trade_category(item_class_name).is_some() {
        SearchResultKind::Equipment
    } else {
        SearchResultKind::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Load full game data for testing (same as poe-eval's test helper).
    fn full_game_data() -> &'static GameData {
        use std::sync::OnceLock;
        static GD: OnceLock<GameData> = OnceLock::new();
        GD.get_or_init(|| {
            let dat_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
            crate::load(&dat_dir).expect("game data must be available for tests")
        })
    }

    #[test]
    fn search_finds_vaal_regalia() {
        let gd = full_game_data();
        let results = gd.browser_search("Vaal Regalia", 10);
        assert!(
            results.iter().any(|r| r.name == "Vaal Regalia"),
            "Expected Vaal Regalia in results: {results:?}"
        );
        let vr = results.iter().find(|r| r.name == "Vaal Regalia").unwrap();
        assert_eq!(vr.kind, SearchResultKind::Equipment);
        assert_eq!(vr.item_class.as_deref(), Some("Body Armours"));
    }

    #[test]
    fn search_partial_match() {
        let gd = full_game_data();
        let results = gd.browser_search("vaal reg", 10);
        assert!(
            results.iter().any(|r| r.name == "Vaal Regalia"),
            "Partial search should match: {results:?}"
        );
    }

    #[test]
    fn search_classifies_currency() {
        let gd = full_game_data();
        let results = gd.browser_search("Chaos Orb", 10);
        let chaos = results.iter().find(|r| r.name == "Chaos Orb");
        assert!(chaos.is_some(), "Chaos Orb should be found: {results:?}");
        assert_eq!(chaos.unwrap().kind, SearchResultKind::Currency);
    }

    #[test]
    fn base_type_detail_vaal_regalia() {
        let gd = full_game_data();
        let detail = gd
            .browser_base_type_detail("Vaal Regalia")
            .expect("Vaal Regalia should exist");
        // GGPK internal ID — no spaces.
        assert!(
            detail.item_class_id == "BodyArmour" || detail.item_class_id == "Body Armour",
            "Unexpected item_class_id: {}",
            detail.item_class_id
        );
        assert_eq!(detail.item_class_name, "Body Armours");
        assert!(detail.drop_level > 0);
        assert!(
            detail.defences.is_some(),
            "Body armour should have defences"
        );
        assert!(
            detail.weapon.is_none(),
            "Body armour should not be a weapon"
        );
        assert!(!detail.tags.is_empty(), "Should have tags");
    }

    #[test]
    fn base_type_detail_weapon() {
        let gd = full_game_data();
        let detail = gd
            .browser_base_type_detail("Vaal Hatchet")
            .expect("Vaal Hatchet should exist");
        assert!(detail.weapon.is_some(), "Weapon should have weapon stats");
        assert!(detail.defences.is_none(), "Weapon should not have defences");
    }

    #[test]
    fn mod_pool_returns_prefixes_and_suffixes() {
        let gd = full_game_data();
        let result = gd
            .browser_mod_pool(&ModPoolQuery {
                base_type: "Vaal Regalia".to_string(),
                item_level: 86,
                generation_types: vec![],
                taken_mod_ids: vec![],
            })
            .expect("mod pool should compute");
        assert!(!result.prefixes.is_empty(), "Should have prefix families");
        assert!(!result.suffixes.is_empty(), "Should have suffix families");
        assert!(result.available_prefix_count > 0);
        assert!(result.available_suffix_count > 0);
    }

    #[test]
    fn mod_pool_tiers_ordered_by_level() {
        let gd = full_game_data();
        let result = gd
            .browser_mod_pool(&ModPoolQuery {
                base_type: "Vaal Regalia".to_string(),
                item_level: 86,
                generation_types: vec![],
                taken_mod_ids: vec![],
            })
            .unwrap();

        for family in result.prefixes.iter().chain(result.suffixes.iter()) {
            for window in family.tiers.windows(2) {
                assert!(
                    window[0].required_level >= window[1].required_level,
                    "Tiers should be ordered by level desc: {} (lvl {}) before {} (lvl {})",
                    window[0].name,
                    window[0].required_level,
                    window[1].name,
                    window[1].required_level,
                );
            }
        }
    }

    #[test]
    fn mod_pool_taken_families_marked() {
        let gd = full_game_data();
        // Find a mod ID that exists on body armour.
        let first_result = gd
            .browser_mod_pool(&ModPoolQuery {
                base_type: "Vaal Regalia".to_string(),
                item_level: 86,
                generation_types: vec![1], // prefixes only
                taken_mod_ids: vec![],
            })
            .unwrap();

        let first_mod_id = &first_result.prefixes[0].tiers[0].mod_id;

        let result = gd
            .browser_mod_pool(&ModPoolQuery {
                base_type: "Vaal Regalia".to_string(),
                item_level: 86,
                generation_types: vec![1],
                taken_mod_ids: vec![first_mod_id.clone()],
            })
            .unwrap();

        let taken_family = result
            .prefixes
            .iter()
            .find(|f| f.tiers.iter().any(|t| t.mod_id == *first_mod_id));
        assert!(
            taken_family.is_some(),
            "Family with taken mod should still appear"
        );
        assert!(taken_family.unwrap().taken, "Family should be marked taken");
    }

    #[test]
    fn mod_pool_ilvl_filters_tiers() {
        let gd = full_game_data();
        let low_level = gd
            .browser_mod_pool(&ModPoolQuery {
                base_type: "Vaal Regalia".to_string(),
                item_level: 1,
                generation_types: vec![],
                taken_mod_ids: vec![],
            })
            .unwrap();
        let high_level = gd
            .browser_mod_pool(&ModPoolQuery {
                base_type: "Vaal Regalia".to_string(),
                item_level: 86,
                generation_types: vec![],
                taken_mod_ids: vec![],
            })
            .unwrap();

        let low_eligible: usize = low_level
            .prefixes
            .iter()
            .chain(low_level.suffixes.iter())
            .flat_map(|f| &f.tiers)
            .filter(|t| t.eligible)
            .count();
        let high_eligible: usize = high_level
            .prefixes
            .iter()
            .chain(high_level.suffixes.iter())
            .flat_map(|f| &f.tiers)
            .filter(|t| t.eligible)
            .count();

        assert!(
            high_eligible > low_eligible,
            "Higher ilvl should have more eligible tiers: {high_eligible} vs {low_eligible}"
        );
    }

    #[test]
    fn mod_pool_jewel_has_mods() {
        let gd = full_game_data();
        let result = gd
            .browser_mod_pool(&ModPoolQuery {
                base_type: "Cobalt Jewel".to_string(),
                item_level: 86,
                generation_types: vec![],
                taken_mod_ids: vec![],
            })
            .expect("Cobalt Jewel mod pool should compute");
        assert!(
            !result.prefixes.is_empty() || !result.suffixes.is_empty(),
            "Jewel should have mods"
        );
    }

    #[test]
    fn search_classifies_jewels() {
        let gd = full_game_data();
        let results = gd.browser_search("Cobalt Jewel", 10);
        let jewel = results.iter().find(|r| r.name == "Cobalt Jewel");
        assert!(jewel.is_some());
        assert_eq!(jewel.unwrap().kind, SearchResultKind::Jewel);
    }

    #[test]
    fn search_empty_query_returns_nothing() {
        let gd = full_game_data();
        let results = gd.browser_search("", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn mod_pool_excludes_essence_only() {
        let gd = full_game_data();
        let result = gd
            .browser_mod_pool(&ModPoolQuery {
                base_type: "Vaal Regalia".to_string(),
                item_level: 86,
                generation_types: vec![],
                taken_mod_ids: vec![],
            })
            .unwrap();

        // Collect all mod IDs from the pool.
        let pool_mod_ids: Vec<&str> = result
            .prefixes
            .iter()
            .chain(result.suffixes.iter())
            .flat_map(|f| &f.tiers)
            .map(|t| t.mod_id.as_str())
            .collect();

        // Verify none of them are essence-only in the raw mods table.
        for mod_id in &pool_mod_ids {
            if let Some(m) = gd.mod_by_id(mod_id) {
                assert!(
                    !m.is_essence_only,
                    "Essence-only mod {mod_id} should not appear in browser pool"
                );
            }
        }
    }

    #[test]
    fn mod_pool_excludes_bench_crafts() {
        let gd = full_game_data();
        let result = gd
            .browser_mod_pool(&ModPoolQuery {
                base_type: "Vaal Regalia".to_string(),
                item_level: 86,
                generation_types: vec![],
                taken_mod_ids: vec![],
            })
            .unwrap();

        let pool_mod_ids: Vec<&str> = result
            .prefixes
            .iter()
            .chain(result.suffixes.iter())
            .flat_map(|f| &f.tiers)
            .map(|t| t.mod_id.as_str())
            .collect();

        for mod_id in &pool_mod_ids {
            if let Some(m) = gd.mod_by_id(mod_id) {
                assert!(
                    m.domain != 9,
                    "Bench craft mod {mod_id} (domain 9) should not appear in browser pool"
                );
            }
        }
    }

    #[test]
    fn mod_pool_stats_have_display_text() {
        let gd = full_game_data();
        let result = gd
            .browser_mod_pool(&ModPoolQuery {
                base_type: "Vaal Regalia".to_string(),
                item_level: 86,
                generation_types: vec![],
                taken_mod_ids: vec![],
            })
            .unwrap();

        let all_stats: Vec<_> = result
            .prefixes
            .iter()
            .chain(result.suffixes.iter())
            .flat_map(|f| &f.tiers)
            .flat_map(|t| &t.stats)
            .collect();

        // stat_template should have '#' placeholders.
        let has_template = all_stats.iter().any(|s| s.stat_template.contains('#'));
        assert!(has_template, "stat_template should contain '#' placeholder");

        // display_text should be fully formatted (no '#').
        let has_placeholder = all_stats.iter().any(|s| s.display_text.contains('#'));
        assert!(
            !has_placeholder,
            "display_text should be fully formatted, no '#'"
        );
    }

    #[test]
    fn affix_limits_rare_equipment() {
        let gd = full_game_data();
        // Uses display name (same as BaseTypeDetail.itemClassName).
        let (p, s) = gd.browser_affix_limits("Body Armours", "Rare");
        assert_eq!(p, 3, "Rare body armour should have 3 prefixes");
        assert_eq!(s, 3, "Rare body armour should have 3 suffixes");
    }

    #[test]
    fn affix_limits_rare_jewel() {
        let gd = full_game_data();
        let (p, s) = gd.browser_affix_limits("Jewels", "Rare");
        assert_eq!(p, 2, "Rare jewel should have 2 prefixes");
        assert_eq!(s, 2, "Rare jewel should have 2 suffixes");
    }
}
