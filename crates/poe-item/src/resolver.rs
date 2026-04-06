//! Pass 2: Data-dependent resolution using `GameData`.
//!
//! Takes a [`RawItem`] from Pass 1 (PEST grammar + tree walker) and produces
//! a [`ResolvedItem`] with:
//! - Value ranges parsed from inline annotations (`+32(25-40)`)
//! - Type suffixes stripped (`(implicit)`, `(crafted)`, etc.)
//! - Magic item base type extracted via game data lookup
//! - Stat IDs resolved via `ReverseIndex` (when available)

use std::sync::LazyLock;

use poe_data::GameData;
use regex::Regex;

use crate::types::{
    GemData, Header, InfluenceKind, ItemProperty, ModDisplayType, ModGroup, ModHeader, ModSlot,
    ModSource, ModTierKind, Rarity, RawItem, RawPropertyLine, ResolvedHeader, ResolvedItem,
    ResolvedMod, ResolvedStatLine, Section, SocketInfo, StatusKind, UniqueCandidate, VaalGemData,
    ValueRange,
};

/// Regex matching value range annotations: `32(25-40)`, `-9(-25-50)`, `1(10--10)`.
static VALUE_RANGE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(-?\d+)\((-?\d+)-(-?\d+)\)").unwrap());

/// Regex matching type suffixes appended by Ctrl+Alt+C format.
static SUFFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s+\((implicit|crafted|enchant|fractured)\)$").unwrap());

/// Suffix appended to stat lines whose values are fixed and cannot be modified.
const UNSCALABLE_SUFFIX: &str = " \u{2014} Unscalable Value";

/// Resolve a [`RawItem`] into a [`ResolvedItem`] using game data.
///
/// Flattens sections into typed fields, parses value ranges, strips
/// display suffixes, resolves stat IDs, splits mods into implicits/explicits,
/// parses properties, and classifies flavor text.
pub fn resolve(raw: &RawItem, game_data: &GameData) -> ResolvedItem {
    let header = resolve_header(&raw.header, game_data);

    let mut item_level = None;
    let mut monster_level = None;
    let mut talisman_tier = None;
    let mut requirements = Vec::new();
    let mut sockets = None;
    let mut experience = None;
    let mut implicits = Vec::new();
    let mut explicits = Vec::new();
    let mut influences = Vec::new();
    let mut statuses = Vec::new();
    let mut note = None;
    let mut grammar_enchant_lines = Vec::new();
    let mut grammar_properties = Vec::new();
    let mut grammar_subheaders = Vec::new();
    let mut generic_sections = Vec::new();

    for section in &raw.sections {
        match section {
            Section::ItemLevel(n) => item_level = Some(*n),
            Section::MonsterLevel(n) => monster_level = Some(*n),
            Section::TalismanTier(n) => talisman_tier = Some(*n),
            Section::Requirements(r) => requirements.clone_from(r),
            Section::Sockets(s) => sockets = Some(s.clone()),
            Section::Experience(e) => experience = Some(e.clone()),
            Section::Modifiers(mod_section) => {
                for group in &mod_section.groups {
                    let resolved =
                        resolve_mod(group, &header.base_type, &header.item_class, game_data);
                    match resolved.header.slot {
                        ModSlot::Implicit
                        | ModSlot::SearingExarchImplicit
                        | ModSlot::EaterOfWorldsImplicit
                        | ModSlot::CorruptionImplicit => {
                            implicits.push(resolved);
                        }
                        ModSlot::Enchant | ModSlot::Pseudo => {
                            // Enchant: shouldn't appear from grammar (enchants come from generic sections).
                            // Pseudo: never appears from grammar (computed after resolution).
                        }
                        ModSlot::Prefix | ModSlot::Suffix | ModSlot::Unique => {
                            explicits.push(resolved);
                        }
                    }
                }
                for &inf in &mod_section.trailing_influences {
                    if !influences.contains(&inf) {
                        influences.push(inf);
                    }
                }
            }
            Section::Influence(infs) => {
                for &inf in infs {
                    if !influences.contains(&inf) {
                        influences.push(inf);
                    }
                }
            }
            Section::Status(s) => statuses.push(*s),
            Section::Note(n) => note = Some(n.clone()),
            Section::Enchants(lines) => {
                // Enchants identified by grammar (lines ending with " (enchant)")
                for line in lines {
                    grammar_enchant_lines.push(line.clone());
                }
            }
            Section::Properties { subheader, lines } => {
                // Properties identified by grammar (Key: Value format).
                // For gems, the sub-header contains comma-separated tags.
                if let Some(sh) = subheader {
                    grammar_subheaders.push(sh.clone());
                }
                for raw in lines {
                    grammar_properties.push(resolve_raw_property(raw));
                }
            }
            Section::Generic(lines) => generic_sections.push(lines.clone()),
        }
    }

    // For gems, extract structured data.
    // If grammar detected a property section with a sub-header (tags), use that.
    // Otherwise fall back to extracting from generic sections.
    let (gem_data, gem_properties) = if header.rarity == Rarity::Gem {
        if let Some(tag_header) = grammar_subheaders.first() {
            // Grammar gave us tags (sub-header) and properties directly.
            // Remaining generic sections are: description, stats+quality.
            let tags: Vec<String> = tag_header
                .split(", ")
                .map(|s| s.trim().to_string())
                .collect();
            let (stats, quality_stats) = generic_sections
                .iter()
                .find(|s| {
                    s.iter()
                        .any(|l| l.starts_with("Additional Effects From Quality"))
                })
                .map_or_else(
                    || {
                        // Last non-empty generic section is stats (no quality marker)
                        generic_sections
                            .last()
                            .map(|s| {
                                (
                                    s.iter().filter(|l| !l.is_empty()).cloned().collect(),
                                    vec![],
                                )
                            })
                            .unwrap_or_default()
                    },
                    |s| split_stats_and_quality(s),
                );
            let description = generic_sections
                .first()
                .map(|s| s.join("\n"))
                .filter(|s| !s.is_empty());
            let vaal = extract_vaal_data(&mut generic_sections.iter().skip(2));
            let data = GemData {
                tags,
                description,
                stats,
                quality_stats,
                vaal,
            };
            (Some(data), grammar_properties.clone())
        } else {
            let (data, props) = extract_gem_data(&generic_sections);
            (Some(data), props)
        }
    } else {
        (None, vec![])
    };

    // For gems, generic sections are consumed by extract_gem_data — pass empty
    let sections_to_classify = if gem_data.is_some() {
        &[][..]
    } else {
        &generic_sections[..]
    };

    // Classify generic sections into properties, enchants, description, flavor text, etc.
    let classified =
        classify_generic_sections(sections_to_classify, header.rarity, &header.item_class);

    // Build enchant mods from both grammar-detected and classifier-detected enchant lines
    let enchants: Vec<ResolvedMod> = grammar_enchant_lines
        .iter()
        .chain(classified.enchant_lines.iter())
        .map(|line| build_enchant_mod(line, game_data))
        .collect();

    // Convenience booleans
    let is_corrupted = statuses.iter().any(|s| matches!(s, StatusKind::Corrupted));
    let is_fractured = influences
        .iter()
        .any(|i| matches!(i, InfluenceKind::Fractured));
    let is_unidentified = statuses
        .iter()
        .any(|s| matches!(s, StatusKind::Unidentified));

    // Pre-compute socket metadata
    let socket_info = sockets.as_deref().map(parse_socket_info);

    // Extract quality from properties (check grammar, classified, and gem properties)
    let quality = grammar_properties
        .iter()
        .chain(classified.properties.iter())
        .chain(gem_properties.iter())
        .find(|p| p.name == "Quality")
        .and_then(|p| {
            p.value
                .trim_start_matches('+')
                .trim_end_matches('%')
                .parse::<u32>()
                .ok()
        });

    // Merge all property sources: grammar-detected, classifier-detected, gem-specific.
    let mut properties = grammar_properties;
    properties.extend(classified.properties);
    // Gem properties are extracted separately (not through classify_generic_sections).
    properties.extend(gem_properties);
    if let Some(ilvl) = item_level {
        properties.push(ItemProperty {
            name: "Item Level".to_string(),
            value: ilvl.to_string(),
            augmented: false,
            synthetic: true,
        });
    }
    // Sockets/Links: names match trade API filter text.
    // "Links" has no GGPK equivalent — it's a trade API concept (max linked group).
    if let Some(ref si) = socket_info {
        properties.push(ItemProperty {
            name: "Sockets".to_string(),
            value: si.total.to_string(),
            augmented: false,
            synthetic: true,
        });
        properties.push(ItemProperty {
            name: "Links".to_string(),
            value: si.max_link.to_string(),
            augmented: false,
            synthetic: true,
        });
    }
    properties.push(ItemProperty {
        name: "Rarity".to_string(),
        value: format!("{:?}", header.rarity),
        augmented: false,
        synthetic: true,
    });
    if let Some(tier) = talisman_tier {
        properties.push(ItemProperty {
            name: "Talisman Tier".to_string(),
            value: tier.to_string(),
            augmented: false,
            synthetic: true,
        });
    }

    // Compute pseudo mods by scanning all mod stat lines against pseudo definitions.
    let mut pseudo_mods = compute_pseudo_stats(&implicits, &explicits, &enchants, game_data);
    // Compute DPS pseudos from weapon properties (Physical/Elemental/Chaos/Total DPS).
    pseudo_mods.extend(compute_dps_pseudos(&properties, game_data));

    // For unidentified uniques, populate possible unique names from game data.
    let unique_candidates = if is_unidentified && header.rarity == Rarity::Unique {
        game_data
            .uniques_for_base_type(&header.base_type)
            .iter()
            .map(|u| UniqueCandidate {
                name: u.name.clone(),
                art: u.art.clone(),
            })
            .collect()
    } else {
        Vec::new()
    };

    ResolvedItem {
        header,
        item_level,
        monster_level,
        talisman_tier,
        requirements,
        sockets,
        socket_info,
        quality,
        experience,
        properties,
        implicits,
        explicits,
        enchants,
        influences,
        statuses,
        is_corrupted,
        is_fractured,
        is_unidentified,
        note,
        description: classified.description,
        flavor_text: classified.flavor_text,
        gem_data,
        pseudo_mods,
        unique_candidates,
        unclassified_sections: classified.unclassified,
    }
}

// ── Header resolution ───────────────────────────────────────────────────────

fn resolve_header(header: &Header, game_data: &GameData) -> ResolvedHeader {
    match header.rarity {
        Rarity::Rare | Rarity::Unique => {
            if let Some(ref base_type) = header.name2 {
                // Identified: name1 is the item name, name2 is the base type.
                ResolvedHeader {
                    item_class: header.item_class.clone(),
                    rarity: header.rarity,
                    name: Some(header.name1.clone()),
                    base_type: base_type.clone(),
                }
            } else {
                // Unidentified: only one name line, which is the base type.
                ResolvedHeader {
                    item_class: header.item_class.clone(),
                    rarity: header.rarity,
                    name: None,
                    base_type: header.name1.clone(),
                }
            }
        }
        Rarity::Magic => {
            // Strip quality prefix before extracting base type from magic name.
            let name = poe_data::domain::strip_quality_prefix(&header.name1);
            let base_type =
                extract_magic_base_type(name, game_data).unwrap_or_else(|| name.to_string());
            ResolvedHeader {
                item_class: header.item_class.clone(),
                rarity: header.rarity,
                name: None,
                base_type,
            }
        }
        // Normal, Gem, Currency, Unknown — name1 is the base type.
        // Strip quality prefix (e.g., "Superior Ezomyte Tower Shield" → "Ezomyte Tower Shield").
        _ => ResolvedHeader {
            item_class: header.item_class.clone(),
            rarity: header.rarity,
            name: None,
            base_type: poe_data::domain::strip_quality_prefix(&header.name1).to_string(),
        },
    }
}

/// Extract the base type from a Magic item's display name.
///
/// Magic items embed the base type in their name: `"Smouldering Foul Staff"`.
/// Finds the longest known base type that is a substring of the name.
fn extract_magic_base_type(name: &str, game_data: &GameData) -> Option<String> {
    let mut best: Option<&str> = None;
    let mut best_len = 0;

    for base in &game_data.base_item_types {
        if !base.name.is_empty() && base.name.len() > best_len && name.contains(&base.name) {
            best = Some(&base.name);
            best_len = base.name.len();
        }
    }

    best.map(String::from)
}

// ── Mod resolution ──────────────────────────────────────────────────────────

fn resolve_mod(
    group: &ModGroup,
    base_type: &str,
    item_class: &str,
    game_data: &GameData,
) -> ResolvedMod {
    let mut stat_lines: Vec<ResolvedStatLine> = group
        .body_lines
        .iter()
        .map(|line| resolve_stat_line(line, game_data))
        .collect();

    // Multi-line stat descriptions: some stats produce two visual lines from
    // one format string with `\n`. Try joining consecutive unresolved lines.
    if game_data.reverse_index.is_some() {
        try_multi_line_resolution(&mut stat_lines, game_data);
    }

    // Base-type-anchored stat_id resolution: the reverse index gives us
    // non-local stat_ids (all it knows from stat_descriptions.txt). The real
    // stat_ids come from the Mods table, confirmed via base type tag
    // intersection. Replace reverse index guesses with confirmed truth.
    //
    // Multiple mods can share a display name (e.g., "Tempered" exists as both
    // local flat phys for weapons and global flat phys for jewelry). Try all
    // eligible candidates — the template matching in apply_confirmed_stat_ids
    // will only succeed for the variant whose stat_ids share display templates
    // with the reverse index stat_ids.
    if let Some(mod_name) = &group.header.name {
        for mod_row in game_data.find_eligible_mods(base_type, mod_name, item_class) {
            let real_stat_ids = game_data.mod_stat_ids(mod_row);
            if apply_confirmed_stat_ids(&mut stat_lines, &real_stat_ids, game_data) {
                break;
            }
        }
    }

    // Detect fractured from either:
    // 1. Header source: { Fractured Prefix Modifier ... }
    // 2. Body line suffix: +19 to Armour (fractured)
    let is_fractured = group.header.source == ModSource::Fractured
        || group
            .body_lines
            .iter()
            .any(|line| line.ends_with("(fractured)"));

    let display_type = ResolvedMod::compute_display_type(group.header.slot, group.header.source);
    ResolvedMod {
        header: group.header.clone(),
        stat_lines,
        is_fractured,
        display_type,
    }
}

/// Replace reverse-index `stat_ids` with confirmed IDs from the Mods table.
///
/// The reverse index gives non-local IDs (e.g., `base_physical_damage_reduction_rating`).
/// The Mods table knows the real IDs (e.g., `local_base_physical_damage_reduction_rating`).
/// For each stat line, if it has reverse-index IDs, find the corresponding real ID
/// from the mod's `stat_keys` and replace it.
///
/// Matching strategy: for each stat line's reverse-index IDs, check if any real ID
/// from the mod shares the same display template. If so, replace with the real one.
///
/// Returns `true` if any `stat_ids` were replaced (indicating this mod candidate
/// was the correct variant).
fn apply_confirmed_stat_ids(
    stat_lines: &mut [ResolvedStatLine],
    real_stat_ids: &[String],
    game_data: &GameData,
) -> bool {
    // Track which real_stat_ids have been consumed (by index) to avoid
    // assigning the same real_id to multiple slots in a multi-value stat
    // (e.g., min and max sharing the same display template).
    let mut used_real: Vec<bool> = vec![false; real_stat_ids.len()];
    let mut any_changed = false;

    for sl in stat_lines.iter_mut() {
        let Some(ri_ids) = &sl.stat_ids else {
            continue;
        };

        let mut confirmed = ri_ids.clone();
        let mut changed = false;

        for (i, ri_id) in ri_ids.iter().enumerate() {
            // Find a real stat_id that maps to the same display template as this
            // reverse-index stat_id. This correctly handles local↔non-local pairs
            // that share display text.
            //
            // Use native_templates_for_stat for the real_id to avoid false matches
            // where local_* stats get fallback templates from attack_* equivalents
            // (e.g., local_minimum_added_fire_damage getting "Adds # to # Fire Damage
            // to Attacks" template). If the real_id has no native template (common for
            // local stats), fall back to full templates — but only if the replacement
            // is a local↔non-local pair (not local replacing attack_*).
            let ri_templates = game_data.templates_for_stat(ri_id);

            for (j, real_id) in real_stat_ids.iter().enumerate() {
                if real_id == ri_id {
                    // Already correct — mark as used so other slots don't grab it.
                    used_real[j] = true;
                    break;
                }
                if used_real[j] {
                    continue;
                }
                // Prefer native templates to avoid false matches from fallbacks.
                // Fall back to full templates only for genuine local↔non-local pairs
                // (where the local stat has no entry in stat_descriptions.txt).
                let real_native = game_data.native_templates_for_stat(real_id);
                let real_templates = if real_native.is_some_and(|t| !t.is_empty()) {
                    real_native
                } else if is_local_nonlocal_pair(real_id, ri_id) {
                    game_data.templates_for_stat(real_id)
                } else {
                    None
                };
                if let (Some(ri_t), Some(real_t)) = (ri_templates, real_templates) {
                    if ri_t.iter().any(|t| real_t.contains(t)) {
                        confirmed[i].clone_from(real_id);
                        used_real[j] = true;
                        changed = true;
                        break;
                    }
                }
            }
        }

        if changed {
            sl.stat_ids = Some(confirmed);
            any_changed = true;
        }
    }
    any_changed
}

/// Check if `real_id` is a `local_*` variant of `ri_id` (stripped or global/base).
///
/// Returns true for pairs like:
/// - `local_minimum_added_physical_damage` ↔ `global_minimum_added_physical_damage`
/// - `local_base_evasion_rating` ↔ `base_evasion_rating`
///
/// Returns false for:
/// - `local_minimum_added_fire_damage` ↔ `attack_minimum_added_fire_damage`
///   (these are different stat categories with different display text)
fn is_local_nonlocal_pair(real_id: &str, ri_id: &str) -> bool {
    let Some(stripped) = real_id.strip_prefix("local_") else {
        return false;
    };
    // Direct match: local_X ↔ X
    if ri_id == stripped {
        return true;
    }
    // Global match: local_X ↔ global_X
    if let Some(ri_stripped) = ri_id.strip_prefix("global_") {
        if stripped == ri_stripped {
            return true;
        }
    }
    false
}

// ── Generic section classification ─────────────────────────────────────────

struct ClassifiedSections {
    properties: Vec<ItemProperty>,
    enchant_lines: Vec<String>,
    description: Option<String>,
    flavor_text: Option<String>,
    unclassified: Vec<Vec<String>>,
}

/// Classify generic sections by content analysis.
///
/// Each section is independently classified as one of:
/// - **Properties**: all lines contain `": "` (e.g., "Armour: 890 (augmented)")
/// - **Enchants**: all lines end with `(enchant)`
/// - **Usage instructions**: starts with known GGG instruction prefix
/// - **Flavor text**: poetic/lore text (no colons, not instructions, not enchants)
/// - **Description**: item effect text (currency effects, scarab effects, etc.)
/// - **Unclassified**: anything else
fn classify_generic_sections(
    sections: &[Vec<String>],
    rarity: Rarity,
    item_class: &str,
) -> ClassifiedSections {
    let mut properties = Vec::new();
    let mut enchant_lines = Vec::new();
    let mut description: Option<String> = None;
    let mut flavor_text = None;
    let mut unclassified = Vec::new();

    for section in sections {
        if section.is_empty() {
            continue;
        }

        let classification = classify_single_section(section, rarity, item_class);
        match classification {
            SectionKind::Properties(props) => properties.extend(props),
            SectionKind::Enchants(lines) => enchant_lines.extend(lines),
            SectionKind::UsageInstructions => {} // Drop — not useful for evaluation
            SectionKind::FlavorText(text) => flavor_text = Some(text),
            SectionKind::Description(text) => {
                // Append if multiple description sections (e.g., essence header + slot table)
                if let Some(existing) = &mut description {
                    existing.push('\n');
                    existing.push_str(&text);
                } else {
                    description = Some(text);
                }
            }
            SectionKind::Unclassified => unclassified.push(section.clone()),
        }
    }

    ClassifiedSections {
        properties,
        enchant_lines,
        description,
        flavor_text,
        unclassified,
    }
}

enum SectionKind {
    Properties(Vec<ItemProperty>),
    Enchants(Vec<String>),
    UsageInstructions,
    FlavorText(String),
    Description(String),
    Unclassified,
}

fn classify_single_section(lines: &[String], rarity: Rarity, item_class: &str) -> SectionKind {
    let non_empty: Vec<&str> = lines
        .iter()
        .map(String::as_str)
        .filter(|l| !l.is_empty())
        .collect();
    if non_empty.is_empty() {
        return SectionKind::Unclassified;
    }

    // All lines end with (enchant) → enchant section
    if non_empty.iter().all(|l| l.ends_with("(enchant)")) {
        return SectionKind::Enchants(non_empty.iter().map(|l| (*l).to_string()).collect());
    }

    // Usage instructions — identified by domain knowledge from poe-data
    if poe_data::domain::is_usage_instruction(non_empty[0]) {
        return SectionKind::UsageInstructions;
    }

    // Check if this is a property section (majority of lines have ": ")
    let colon_count = non_empty.iter().filter(|l| l.contains(": ")).count();
    // Heist skill requirements: "Requires Lockpicking (Level 3)" — no colon but still a property
    let heist_req_count = non_empty
        .iter()
        .filter(|l| l.starts_with("Requires ") && l.contains("(Level "))
        .count();
    let prop_like_count = colon_count + heist_req_count;
    if prop_like_count > 0 && prop_like_count == non_empty.len() {
        // All lines are property-like
        let props = parse_property_lines(lines);
        return SectionKind::Properties(props);
    }

    // Weapon sections: first line is a weapon type sub-header (e.g., "Warstaff",
    // "Two Handed Axe", "Bow") without ": ", followed by property lines that all have ": ".
    // Only weapons have this pattern — armour/accessories start directly with properties.
    if colon_count > 0
        && colon_count == non_empty.len() - 1
        && !non_empty[0].contains(": ")
        && poe_data::domain::is_weapon_class(item_class)
    {
        let property_lines: Vec<String> = lines.iter().skip(1).cloned().collect();
        let props = parse_property_lines(&property_lines);
        return SectionKind::Properties(props);
    }

    // Mixed section: some lines have colons, some don't.
    // For currency/essence: the description header + slot table are mixed.
    // Treat the whole section as description text.
    if colon_count > 0 && rarity == Rarity::Currency {
        return SectionKind::Description(lines.join("\n"));
    }

    // Pure text section (no colons) — could be flavor text or description
    // Flavor text: appears on Unique, DivinationCard, and scarab-like items (Normal Map Fragments)
    // Description: effect text on currency, scarabs, tinctures, etc.
    let text = lines.join("\n");

    // Currency/gem descriptions come before flavor text
    if matches!(rarity, Rarity::Currency | Rarity::Gem) {
        // Currency items: first text section is description, rest are unclassified
        return SectionKind::Description(text);
    }

    // For uniques and div cards: text sections are typically flavor text
    if matches!(rarity, Rarity::Unique | Rarity::DivinationCard) {
        return SectionKind::FlavorText(text);
    }

    // Quoted text is flavor text regardless of rarity (e.g., heist contracts)
    if non_empty[0].starts_with('"') {
        return SectionKind::FlavorText(text);
    }

    // For Normal rarity items that are scarabs/fragments: first text section is description,
    // subsequent ones might be flavor text. Use a heuristic: if it looks like a game effect
    // (long, mechanical language), it's description. If it's short/poetic, it's flavor.
    if rarity == Rarity::Normal {
        // Short poetic text → flavor; longer mechanical text → description
        if non_empty.len() <= 2 && text.len() < 80 {
            return SectionKind::FlavorText(text);
        }
        return SectionKind::Description(text);
    }

    SectionKind::Unclassified
}

/// Convert a grammar-parsed `RawPropertyLine` into an `ItemProperty`.
///
/// Handles the `(augmented)` marker which the grammar captures as part of
/// the value text.
fn resolve_raw_property(raw: &RawPropertyLine) -> ItemProperty {
    let augmented = raw.value.contains("(augmented)");
    let value = raw
        .value
        .replace(" (augmented)", "")
        .replace("(augmented)", "")
        .trim()
        .to_string();
    ItemProperty {
        name: raw.key.clone(),
        value,
        augmented,
        synthetic: false,
    }
}

fn parse_property_lines(lines: &[String]) -> Vec<ItemProperty> {
    let mut props = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        // Heist skill requirements: "Requires Lockpicking (Level 3)" or
        // "Requires Demolition (Level 5 (unmet))"
        if let Some(prop) = parse_heist_requirement(line) {
            props.push(prop);
            continue;
        }
        if let Some((name, rest)) = line.split_once(": ") {
            let augmented = rest.contains("(augmented)");
            let value = rest
                .replace(" (augmented)", "")
                .replace("(augmented)", "")
                .trim()
                .to_string();
            props.push(ItemProperty {
                name: name.to_string(),
                value,
                augmented,
                synthetic: false,
            });
        }
    }
    props
}

/// Parse a heist skill requirement line into an `ItemProperty`.
///
/// Format: `Requires <Skill> (Level <N>)` or `Requires <Skill> (Level <N> (unmet))`
fn parse_heist_requirement(line: &str) -> Option<ItemProperty> {
    let rest = line.strip_prefix("Requires ")?;
    let paren_idx = rest.find(" (Level ")?;
    let skill = &rest[..paren_idx];
    // Extract everything inside the outer parentheses: "Level 3" or "Level 5 (unmet)"
    let level_part = &rest[paren_idx + 2..]; // skip " ("
    let value = level_part.strip_suffix(')')?.to_string();
    Some(ItemProperty {
        name: format!("Requires {skill}"),
        value,
        augmented: false,
        synthetic: false,
    })
}

// ── Gem data extraction ──────────────────────────────────────────────────────

/// Extract structured gem data from generic sections.
///
/// Gem section order:
/// 1. Tags + properties (first line = comma tags, rest = Key: Value)
/// 2. Description (single paragraph, no colons)
/// 3. Stats + quality effects (stat lines, blank, "Additional Effects From Quality:", quality lines)
/// 4. [Vaal only] Vaal name separator → repeats 1,3 for Vaal variant
fn extract_gem_data(sections: &[Vec<String>]) -> (GemData, Vec<ItemProperty>) {
    let mut iter = sections.iter();

    // Section 1: Tags + gem properties
    let (tags, gem_props) = iter
        .next()
        .map(|s| split_gem_tags_and_props(s))
        .unwrap_or_default();

    // Section 2: Description
    let description = iter.next().map(|s| s.join("\n")).filter(|s| !s.is_empty());

    // Section 3: Stats + quality effects
    let (stats, quality_stats) = iter
        .next()
        .map(|s| split_stats_and_quality(s))
        .unwrap_or_default();

    // Check if there's a Vaal variant (next section is a single-line name)
    let vaal = extract_vaal_data(&mut iter);

    (
        GemData {
            tags,
            description,
            stats,
            quality_stats,
            vaal,
        },
        gem_props,
    )
}

/// Split the first gem section into tags (first line) and properties (remaining lines).
fn split_gem_tags_and_props(lines: &[String]) -> (Vec<String>, Vec<ItemProperty>) {
    if lines.is_empty() {
        return (vec![], vec![]);
    }

    // First line is comma-separated tags (no colon)
    let tags: Vec<String> = lines[0].split(", ").map(|s| s.trim().to_string()).collect();

    // Remaining lines are gem properties (Key: Value)
    let props = parse_property_lines(&lines[1..]);

    (tags, props)
}

/// Split a stats section at "Additional Effects From Quality:" marker.
fn split_stats_and_quality(lines: &[String]) -> (Vec<String>, Vec<String>) {
    let quality_marker = lines
        .iter()
        .position(|l| l.starts_with("Additional Effects From Quality"));

    if let Some(pos) = quality_marker {
        // Stats are before the marker (skip trailing blank lines)
        let stats: Vec<String> = lines[..pos]
            .iter()
            .filter(|l| !l.is_empty())
            .cloned()
            .collect();
        // Quality effects are after the marker
        let quality: Vec<String> = lines[pos + 1..]
            .iter()
            .filter(|l| !l.is_empty())
            .cloned()
            .collect();
        (stats, quality)
    } else {
        let stats: Vec<String> = lines.iter().filter(|l| !l.is_empty()).cloned().collect();
        (stats, vec![])
    }
}

/// Try to extract Vaal variant data from remaining sections.
fn extract_vaal_data<'a>(
    iter: &mut impl Iterator<Item = &'a Vec<String>>,
) -> Option<Box<VaalGemData>> {
    // Peek at the next section — if it's a single line starting with "Vaal ", consume it
    let name_section = iter.next()?;
    if name_section.len() != 1
        || name_section[0].is_empty()
        || !name_section[0].starts_with("Vaal ")
    {
        return None; // Not a Vaal separator
    }

    let name = name_section[0].clone();

    // Vaal properties (Souls Per Use, etc.)
    let vaal_props = iter
        .next()
        .map(|s| parse_property_lines(s))
        .unwrap_or_default();

    // Vaal description
    let vaal_desc = iter.next().map(|s| s.join("\n")).filter(|s| !s.is_empty());

    // Vaal stats + quality effects
    let (vaal_stats, vaal_quality) = iter
        .next()
        .map(|s| split_stats_and_quality(s))
        .unwrap_or_default();

    Some(Box::new(VaalGemData {
        name,
        properties: vaal_props,
        description: vaal_desc,
        stats: vaal_stats,
        quality_stats: vaal_quality,
    }))
}

/// Build a synthetic `ResolvedMod` from an enchant line.
fn build_enchant_mod(line: &str, game_data: &GameData) -> ResolvedMod {
    let stat_line = resolve_stat_line(line, game_data);
    ResolvedMod {
        header: ModHeader {
            source: ModSource::Regular,
            slot: ModSlot::Enchant,
            influence_tier: None,
            name: None,
            tier: None,
            tags: vec![],
        },
        stat_lines: vec![stat_line],
        is_fractured: false,
        display_type: ModDisplayType::Enchant,
    }
}

/// Try joining consecutive unresolved non-reminder lines with `\n` and
/// looking them up as a single multi-line stat description.
///
/// When a joined lookup succeeds, the stat IDs and values are placed on the
/// first line; the continuation line keeps `stat_ids: None`.
fn try_multi_line_resolution(lines: &mut [ResolvedStatLine], game_data: &GameData) {
    let Some(ri) = &game_data.reverse_index else {
        return;
    };

    let mut i = 0;
    while i + 1 < lines.len() {
        // Only try joining if both lines are unresolved and non-reminder
        if lines[i].stat_ids.is_none()
            && !lines[i].is_reminder
            && lines[i + 1].stat_ids.is_none()
            && !lines[i + 1].is_reminder
        {
            let joined = format!("{}\n{}", lines[i].display_text, lines[i + 1].display_text);
            if let Some(m) = ri.lookup(&joined) {
                lines[i].stat_ids = Some(m.stat_ids);
                lines[i].stat_values = Some(m.values);
                i += 2;
                continue;
            }
        }
        i += 1;
    }
}

fn resolve_stat_line(raw_text: &str, game_data: &GameData) -> ResolvedStatLine {
    let is_reminder = raw_text.starts_with('(') && raw_text.ends_with(')');
    let is_unscalable = raw_text.ends_with(UNSCALABLE_SUFFIX);

    let values = parse_value_ranges(raw_text);
    let display_text = build_display_text(raw_text);

    // Only attempt stat ID resolution for non-reminder lines
    let (stat_ids, stat_values) = if is_reminder {
        (None, None)
    } else if let Some(ri) = &game_data.reverse_index {
        if let Some(m) = ri.lookup(&display_text) {
            (Some(m.stat_ids), Some(m.values))
        } else {
            tracing::debug!(
                display_text,
                "stat line did not match any reverse index entry"
            );
            (None, None)
        }
    } else {
        (None, None)
    };

    ResolvedStatLine {
        raw_text: raw_text.to_string(),
        display_text,
        values,
        stat_ids,
        stat_values,
        is_reminder,
        is_unscalable,
    }
}

// ── Pure helper functions ───────────────────────────────────────────────────

/// Parse all value range annotations from a stat line.
///
/// Finds patterns like `+32(25-40)` and extracts `ValueRange { current: 32, min: 25, max: 40 }`.
/// Handles negative values: `-9(-25-50)`, `1(10--10)`.
pub(crate) fn parse_value_ranges(text: &str) -> Vec<ValueRange> {
    VALUE_RANGE_RE
        .captures_iter(text)
        .filter_map(|cap| {
            let current = cap[1].parse::<i64>().ok()?;
            let min = cap[2].parse::<i64>().ok()?;
            let max = cap[3].parse::<i64>().ok()?;
            Some(ValueRange { current, min, max })
        })
        .collect()
}

/// Strip range annotations and type suffixes to produce display text.
///
/// - `+32(25-40) to maximum Life` → `+32 to maximum Life`
/// - `15(6-15)% increased Rarity of Items found (implicit)` → `15% increased Rarity of Items found`
/// - `Adds 18(14-20) to 33(29-33) Fire Damage` → `Adds 18 to 33 Fire Damage`
pub(crate) fn build_display_text(text: &str) -> String {
    // Remove range annotations: "32(25-40)" → "32"
    let stripped = VALUE_RANGE_RE.replace_all(text, "$1");
    // Remove type suffixes: " (implicit)", " (crafted)", " (enchant)", " (fractured)"
    let stripped = SUFFIX_RE.replace(&stripped, "");
    // Remove unscalable value annotation: " — Unscalable Value"
    match stripped.strip_suffix(UNSCALABLE_SUFFIX) {
        Some(s) => s.to_string(),
        None => stripped.into_owned(),
    }
}

/// Parse a socket string (e.g., `"R-G-B B"`) into structured metadata.
///
/// Letters are sockets, `-` = linked, ` ` = new group.
fn parse_socket_info(socket_str: &str) -> SocketInfo {
    let mut total: u32 = 0;
    let mut red: u32 = 0;
    let mut green: u32 = 0;
    let mut blue: u32 = 0;
    let mut white: u32 = 0;

    let mut max_link: u32 = 0;
    let mut current: u32 = 0;
    for c in socket_str.chars() {
        if c.is_ascii_alphabetic() {
            total += 1;
            match c {
                'R' => red += 1,
                'G' => green += 1,
                'B' => blue += 1,
                'W' => white += 1,
                _ => {}
            }
            if current == 0 {
                current = 1;
            }
        } else if c == '-' {
            current += 1;
        } else {
            max_link = max_link.max(current);
            current = 0;
        }
    }
    max_link = max_link.max(current);

    SocketInfo {
        total,
        max_link,
        red,
        green,
        blue,
        white,
    }
}

/// Compute pseudo stats by scanning all mod stat lines against resolved pseudo definitions.
///
/// For each pseudo definition, sums values from matching `stat_ids` across all mods,
/// applying multipliers. Skips pseudos where a required component has no match.
/// Returns synthetic `ResolvedMod` entries with `display_type: Pseudo`.
fn compute_pseudo_stats(
    implicits: &[ResolvedMod],
    explicits: &[ResolvedMod],
    enchants: &[ResolvedMod],
    game_data: &GameData,
) -> Vec<ResolvedMod> {
    let definitions = game_data.pseudo_definitions();
    if definitions.is_empty() {
        return Vec::new();
    }

    // Collect all stat lines from all mods into a flat list for scanning
    let all_mods: Vec<&ResolvedMod> = enchants
        .iter()
        .chain(implicits.iter())
        .chain(explicits.iter())
        .collect();

    let mut results = Vec::new();

    for def in definitions {
        if def.components.is_empty() {
            continue;
        }

        let mut total: f64 = 0.0;
        let mut has_required = false;
        let mut any_required_defined = false;
        // Track the worst (highest number) tier among contributing mods.
        // This gives the pseudo an aggregate tier representing the weakest link.
        let mut worst_tier: Option<u32> = None;

        for comp in def.components {
            if comp.required {
                any_required_defined = true;
            }

            let mut comp_value: f64 = 0.0;
            let mut comp_found = false;

            for m in &all_mods {
                for sl in &m.stat_lines {
                    if sl.is_reminder {
                        continue;
                    }
                    let matches = sl.stat_ids.as_ref().is_some_and(|ids| {
                        ids.iter()
                            .any(|id| comp.stat_ids.iter().any(|cid| cid == id))
                    });
                    if matches && !sl.values.is_empty() {
                        // Use the first value (most stats are single-value)
                        comp_value += sl.values[0].current as f64;
                        comp_found = true;
                        // Track this mod's tier for the pseudo aggregate
                        if let Some(tier_num) = m.header.tier.as_ref().map(ModTierKind::number) {
                            worst_tier = Some(worst_tier.map_or(tier_num, |cur| cur.max(tier_num)));
                        }
                    }
                }
            }

            if comp_found {
                total += comp_value * comp.multiplier;
                if comp.required {
                    has_required = true;
                }
            }
        }

        // Skip if any required component was not found
        if any_required_defined && !has_required {
            continue;
        }

        // Only include pseudos with a non-zero value
        if total.abs() > f64::EPSILON {
            // Round to nearest integer for display (pseudo stats are always whole numbers
            // in PoE — life, resistances, attributes are all integer).
            #[allow(clippy::cast_possible_truncation)]
            let value = total.round() as i64;

            // Substitute the computed value into the template label:
            // "(Pseudo) +# total maximum Life" → "(Pseudo) +142 total maximum Life"
            // "(Pseudo) +#% total to Fire Resistance" → "(Pseudo) +45% total to Fire Resistance"
            let display_text = def.label.replacen("#%", &format!("{value}%"), 1).replacen(
                '#',
                &format!("{value}"),
                1,
            );

            results.push(ResolvedMod {
                header: ModHeader {
                    source: ModSource::Computed,
                    slot: ModSlot::Pseudo,
                    influence_tier: None,
                    name: None,
                    tier: worst_tier.map(ModTierKind::Tier),
                    tags: vec![],
                },
                stat_lines: vec![ResolvedStatLine {
                    raw_text: def.label.to_string(),
                    display_text,
                    values: vec![ValueRange {
                        current: value,
                        min: 0,
                        max: 0,
                    }],
                    stat_ids: Some(vec![def.id.to_string()]),
                    stat_values: None,
                    is_reminder: false,
                    is_unscalable: false,
                }],
                is_fractured: false,
                display_type: ModDisplayType::Pseudo,
            });
        }
    }

    results
}

// ── DPS computation ─────────────────────────────────────────────────────────

/// Parse a single `"min-max"` damage string into `(f64, f64)`.
fn parse_damage_value(s: &str) -> Option<(f64, f64)> {
    let (min_s, max_s) = s.trim().split_once('-')?;
    let min = min_s.trim().parse::<f64>().ok()?;
    let max = max_s.trim().parse::<f64>().ok()?;
    Some((min, max))
}

/// Parse a property's comma-separated damage ranges.
///
/// E.g., `"3-6, 7-108"` → `[(3.0, 6.0), (7.0, 108.0)]`
fn parse_damage_ranges(value: &str) -> Vec<(f64, f64)> {
    value
        .split(',')
        .filter_map(|segment| parse_damage_value(segment.trim()))
        .collect()
}

/// Compute DPS pseudo stats from weapon properties.
///
/// Only produces results for weapons (items with an "Attacks per Second" property).
/// Uses final displayed property values — the game pre-computes base + local mods + quality.
fn compute_dps_pseudos(properties: &[ItemProperty], game_data: &GameData) -> Vec<ResolvedMod> {
    // APS is the gate: if it's missing, this isn't a weapon.
    let aps = properties
        .iter()
        .find(|p| p.name == "Attacks per Second")
        .and_then(|p| p.value.parse::<f64>().ok())
        .filter(|&v| v > 0.0);
    let Some(aps) = aps else {
        return Vec::new();
    };

    let phys_avg = properties
        .iter()
        .find(|p| p.name == "Physical Damage")
        .and_then(|p| parse_damage_value(&p.value))
        .map_or(0.0, |(min, max)| f64::midpoint(min, max));

    let ele_avg: f64 = properties
        .iter()
        .find(|p| p.name == "Elemental Damage")
        .map(|p| parse_damage_ranges(&p.value))
        .unwrap_or_default()
        .iter()
        .map(|&(min, max)| f64::midpoint(min, max))
        .sum();

    let chaos_avg = properties
        .iter()
        .find(|p| p.name == "Chaos Damage")
        .and_then(|p| parse_damage_value(&p.value))
        .map_or(0.0, |(min, max)| f64::midpoint(min, max));

    let definitions = game_data.dps_pseudo_definitions();
    let mut results = Vec::new();

    for def in definitions {
        let value = match def.kind {
            poe_data::domain::DpsPseudoKind::Physical => phys_avg * aps,
            poe_data::domain::DpsPseudoKind::Elemental => ele_avg * aps,
            poe_data::domain::DpsPseudoKind::Chaos => chaos_avg * aps,
            poe_data::domain::DpsPseudoKind::Total => (phys_avg + ele_avg + chaos_avg) * aps,
        };

        if value.abs() < f64::EPSILON {
            continue;
        }

        #[allow(clippy::cast_possible_truncation)]
        let rounded = value.round() as i64;
        let display_text = def.label.replacen('#', &format!("{rounded}"), 1);

        results.push(ResolvedMod {
            header: ModHeader {
                source: ModSource::Computed,
                slot: ModSlot::Pseudo,
                influence_tier: None,
                name: None,
                tier: None,
                tags: vec![],
            },
            stat_lines: vec![ResolvedStatLine {
                raw_text: def.label.to_string(),
                display_text,
                values: vec![ValueRange {
                    current: rounded,
                    min: 0,
                    max: 0,
                }],
                stat_ids: Some(vec![def.id.to_string()]),
                stat_values: None,
                is_reminder: false,
                is_unscalable: false,
            }],
            is_fractured: false,
            display_type: ModDisplayType::Pseudo,
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_range() {
        let ranges = parse_value_ranges("+32(25-40) to maximum Life");
        assert_eq!(ranges.len(), 1);
        assert_eq!(
            ranges[0],
            ValueRange {
                current: 32,
                min: 25,
                max: 40
            }
        );
    }

    #[test]
    fn parse_negative_current() {
        let ranges = parse_value_ranges("-9(-25-50)% to Cold Resistance");
        assert_eq!(ranges.len(), 1);
        assert_eq!(
            ranges[0],
            ValueRange {
                current: -9,
                min: -25,
                max: 50
            }
        );
    }

    #[test]
    fn parse_negative_max() {
        // Ventor's Gamble: "1(10--10)% reduced Quantity of Items found"
        let ranges = parse_value_ranges("1(10--10)% reduced Quantity of Items found");
        assert_eq!(ranges.len(), 1);
        assert_eq!(
            ranges[0],
            ValueRange {
                current: 1,
                min: 10,
                max: -10
            }
        );
    }

    #[test]
    fn parse_two_ranges_adds() {
        let ranges = parse_value_ranges("Adds 18(14-20) to 33(29-33) Fire Damage");
        assert_eq!(ranges.len(), 2);
        assert_eq!(
            ranges[0],
            ValueRange {
                current: 18,
                min: 14,
                max: 20
            }
        );
        assert_eq!(
            ranges[1],
            ValueRange {
                current: 33,
                min: 29,
                max: 33
            }
        );
    }

    #[test]
    fn parse_no_ranges() {
        let ranges =
            parse_value_ranges("+22% Chance to Block Attack Damage while wielding a Staff");
        assert!(ranges.is_empty());
    }

    #[test]
    fn display_text_strips_ranges() {
        assert_eq!(
            build_display_text("+32(25-40) to maximum Life"),
            "+32 to maximum Life"
        );
    }

    #[test]
    fn display_text_strips_suffix() {
        assert_eq!(
            build_display_text("15(6-15)% increased Rarity of Items found (implicit)"),
            "15% increased Rarity of Items found"
        );
    }

    #[test]
    fn display_text_strips_crafted_suffix() {
        assert_eq!(
            build_display_text("+10(10-12)% to Cold and Lightning Resistances (crafted)"),
            "+10% to Cold and Lightning Resistances"
        );
    }

    #[test]
    fn display_text_negative_range() {
        assert_eq!(
            build_display_text("1(10--10)% reduced Quantity of Items found"),
            "1% reduced Quantity of Items found"
        );
    }

    #[test]
    fn display_text_adds_two_ranges() {
        assert_eq!(
            build_display_text("Adds 18(14-20) to 33(29-33) Fire Damage"),
            "Adds 18 to 33 Fire Damage"
        );
    }

    #[test]
    fn display_text_no_changes() {
        let text = "+22% Chance to Block Attack Damage while wielding a Staff";
        assert_eq!(build_display_text(text), text);
    }
}
