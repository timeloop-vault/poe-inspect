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
    ModSource, Rarity, RawItem, ResolvedHeader, ResolvedItem, ResolvedMod, ResolvedStatLine,
    Section, StatusKind, VaalGemData, ValueRange,
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
                        | ModSlot::EaterOfWorldsImplicit => {
                            implicits.push(resolved);
                        }
                        ModSlot::Enchant => {
                            // Shouldn't appear from grammar (enchants come from generic sections),
                            // but handle for completeness.
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
            Section::Generic(lines) => generic_sections.push(lines.clone()),
        }
    }

    // For gems, extract structured data from generic sections before classification
    let gem_data = if header.rarity == Rarity::Gem {
        Some(extract_gem_data(&generic_sections))
    } else {
        None
    };

    // For gems, generic sections are consumed by extract_gem_data — pass empty
    let sections_to_classify = if gem_data.is_some() {
        &[][..]
    } else {
        &generic_sections[..]
    };

    // Classify generic sections into properties, enchants, description, flavor text, etc.
    let classified = classify_generic_sections(sections_to_classify, header.rarity);

    // Build enchant mods from detected enchant lines
    let enchants: Vec<ResolvedMod> = classified
        .enchant_lines
        .iter()
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

    ResolvedItem {
        header,
        item_level,
        monster_level,
        talisman_tier,
        requirements,
        sockets,
        experience,
        properties: classified.properties,
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
        unclassified_sections: classified.unclassified,
    }
}

// ── Header resolution ───────────────────────────────────────────────────────

fn resolve_header(header: &Header, game_data: &GameData) -> ResolvedHeader {
    match header.rarity {
        Rarity::Rare | Rarity::Unique => ResolvedHeader {
            item_class: header.item_class.clone(),
            rarity: header.rarity,
            name: Some(header.name1.clone()),
            base_type: header.name2.clone().unwrap_or_default(),
        },
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
    if let Some(mod_name) = &group.header.name {
        if let Some(mod_row) = game_data.find_eligible_mod(base_type, mod_name, item_class) {
            let real_stat_ids = game_data.mod_stat_ids(mod_row);
            apply_confirmed_stat_ids(&mut stat_lines, &real_stat_ids, game_data);
        }
    }

    // Detect fractured from raw text suffix "(fractured)" on any stat line
    let is_fractured = group
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
fn apply_confirmed_stat_ids(
    stat_lines: &mut [ResolvedStatLine],
    real_stat_ids: &[String],
    game_data: &GameData,
) {
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
            let ri_templates = game_data.templates_for_stat(ri_id);

            for real_id in real_stat_ids {
                if real_id == ri_id {
                    // Already correct, no replacement needed.
                    break;
                }
                let real_templates = game_data.templates_for_stat(real_id);
                if let (Some(ri_t), Some(real_t)) = (ri_templates, real_templates) {
                    if ri_t.iter().any(|t| real_t.contains(t)) {
                        confirmed[i].clone_from(real_id);
                        changed = true;
                        break;
                    }
                }
            }
        }

        if changed {
            sl.stat_ids = Some(confirmed);
        }
    }
}

// ── Generic section classification ─────────────────────────────────────────

struct ClassifiedSections {
    properties: Vec<ItemProperty>,
    enchant_lines: Vec<String>,
    description: Option<String>,
    flavor_text: Option<String>,
    unclassified: Vec<Vec<String>>,
}

/// Known prefixes for GGG usage instructions (not flavor text, not descriptions).
const USAGE_PREFIXES: &[&str] = &[
    "Right click",
    "Place into",
    "Travel to",
    "Can be used",
    "This is a Support Gem",
    "Shift click to unstack",
];

/// Classify generic sections by content analysis.
///
/// Each section is independently classified as one of:
/// - **Properties**: all lines contain `": "` (e.g., "Armour: 890 (augmented)")
/// - **Enchants**: all lines end with `(enchant)`
/// - **Usage instructions**: starts with known GGG instruction prefix
/// - **Flavor text**: poetic/lore text (no colons, not instructions, not enchants)
/// - **Description**: item effect text (currency effects, scarab effects, etc.)
/// - **Unclassified**: anything else
fn classify_generic_sections(sections: &[Vec<String>], rarity: Rarity) -> ClassifiedSections {
    let mut properties = Vec::new();
    let mut enchant_lines = Vec::new();
    let mut description: Option<String> = None;
    let mut flavor_text = None;
    let mut unclassified = Vec::new();

    for section in sections {
        if section.is_empty() {
            continue;
        }

        let classification = classify_single_section(section, rarity);
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

fn classify_single_section(lines: &[String], rarity: Rarity) -> SectionKind {
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

    // Starts with known usage instruction prefix → drop
    if USAGE_PREFIXES.iter().any(|p| non_empty[0].starts_with(p)) {
        return SectionKind::UsageInstructions;
    }

    // Check if this is a property section (majority of lines have ": ")
    let colon_count = non_empty.iter().filter(|l| l.contains(": ")).count();
    if colon_count > 0 && colon_count == non_empty.len() {
        // All lines are property-like
        let props = parse_property_lines(lines);
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

fn parse_property_lines(lines: &[String]) -> Vec<ItemProperty> {
    let mut props = Vec::new();
    for line in lines {
        if line.is_empty() {
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
            });
        }
    }
    props
}

// ── Gem data extraction ──────────────────────────────────────────────────────

/// Extract structured gem data from generic sections.
///
/// Gem section order:
/// 1. Tags + properties (first line = comma tags, rest = Key: Value)
/// 2. Description (single paragraph, no colons)
/// 3. Stats + quality effects (stat lines, blank, "Additional Effects From Quality:", quality lines)
/// 4. [Vaal only] Vaal name separator → repeats 1,3 for Vaal variant
fn extract_gem_data(sections: &[Vec<String>]) -> GemData {
    let mut iter = sections.iter();

    // Section 1: Tags + gem properties
    let (tags, _gem_props) = iter
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

    GemData {
        tags,
        description,
        stats,
        quality_stats,
        vaal,
    }
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
    // Peek at the next section — if it's a single line (Vaal skill name), consume it
    let name_section = iter.next()?;
    if name_section.len() != 1 || name_section[0].is_empty() {
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
