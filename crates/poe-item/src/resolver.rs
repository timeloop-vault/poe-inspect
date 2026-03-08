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
    Header, ModGroup, Rarity, RawItem, ResolvedHeader, ResolvedItem, ResolvedMod,
    ResolvedStatLine, Section, ValueRange,
};

/// Regex matching value range annotations: `32(25-40)`, `-9(-25-50)`, `1(10--10)`.
static VALUE_RANGE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(-?\d+)\((-?\d+)-(-?\d+)\)").unwrap());

/// Regex matching type suffixes appended by Ctrl+Alt+C format.
static SUFFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\s+\((implicit|crafted|enchant|fractured)\)$").unwrap()
});

/// Resolve a [`RawItem`] into a [`ResolvedItem`] using game data.
///
/// Flattens sections into typed fields, parses value ranges, strips
/// display suffixes, and resolves stat IDs when a `ReverseIndex` is available.
pub fn resolve(raw: &RawItem, game_data: &GameData) -> ResolvedItem {
    let header = resolve_header(&raw.header, game_data);

    let mut item_level = None;
    let mut monster_level = None;
    let mut talisman_tier = None;
    let mut requirements = Vec::new();
    let mut sockets = None;
    let mut experience = None;
    let mut mods = Vec::new();
    let mut influences = Vec::new();
    let mut statuses = Vec::new();
    let mut properties = Vec::new();

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
                    mods.push(resolve_mod(group, game_data));
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
            Section::Generic(lines) => properties.push(lines.clone()),
        }
    }

    ResolvedItem {
        header,
        item_level,
        monster_level,
        talisman_tier,
        requirements,
        sockets,
        experience,
        mods,
        influences,
        statuses,
        properties,
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
            let base_type = extract_magic_base_type(&header.name1, game_data)
                .unwrap_or_else(|| header.name1.clone());
            ResolvedHeader {
                item_class: header.item_class.clone(),
                rarity: header.rarity,
                name: None,
                base_type,
            }
        }
        // Normal, Gem, Currency, Unknown — name1 is the base type
        _ => ResolvedHeader {
            item_class: header.item_class.clone(),
            rarity: header.rarity,
            name: None,
            base_type: header.name1.clone(),
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

fn resolve_mod(group: &ModGroup, game_data: &GameData) -> ResolvedMod {
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

    ResolvedMod {
        header: group.header.clone(),
        stat_lines,
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

    let values = parse_value_ranges(raw_text);
    let display_text = build_display_text(raw_text);

    // Only attempt stat ID resolution for non-reminder lines
    let (stat_ids, stat_values) = if is_reminder {
        (None, None)
    } else if let Some(ri) = &game_data.reverse_index {
        if let Some(m) = ri.lookup(&display_text) {
            (Some(m.stat_ids), Some(m.values))
        } else {
            tracing::debug!(display_text, "stat line did not match any reverse index entry");
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
    stripped.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_range() {
        let ranges = parse_value_ranges("+32(25-40) to maximum Life");
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0], ValueRange { current: 32, min: 25, max: 40 });
    }

    #[test]
    fn parse_negative_current() {
        let ranges = parse_value_ranges("-9(-25-50)% to Cold Resistance");
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0], ValueRange { current: -9, min: -25, max: 50 });
    }

    #[test]
    fn parse_negative_max() {
        // Ventor's Gamble: "1(10--10)% reduced Quantity of Items found"
        let ranges = parse_value_ranges("1(10--10)% reduced Quantity of Items found");
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0], ValueRange { current: 1, min: 10, max: -10 });
    }

    #[test]
    fn parse_two_ranges_adds() {
        let ranges = parse_value_ranges("Adds 18(14-20) to 33(29-33) Fire Damage");
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0], ValueRange { current: 18, min: 14, max: 20 });
        assert_eq!(ranges[1], ValueRange { current: 33, min: 29, max: 33 });
    }

    #[test]
    fn parse_no_ranges() {
        let ranges = parse_value_ranges("+22% Chance to Block Attack Damage while wielding a Staff");
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
