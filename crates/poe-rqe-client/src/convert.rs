use std::collections::HashMap;

use poe_item::types::{ModDisplayType, Rarity, ResolvedItem, ResolvedMod, ResolvedStatLine};
use poe_rqe::eval::Entry;

/// Convert a `ResolvedItem` (from poe-item) into a flat `Entry` (for poe-rqe matching).
///
/// This is the bridge between the `PoE` domain (parsed items) and the domain-free
/// RQE service (flat key-value maps). Analogous to how `poe-trade::build_query()`
/// converts `ResolvedItem` into the trade API format.
///
/// # Key format
///
/// - Item metadata: `"item_class"`, `"rarity"`, `"base_type"`, `"name"`, `"item_level"`, etc.
/// - Stat lines: `"{source}.{template}"` where source is `implicit`/`explicit`/`enchant`/`crafted`
///   and template is the stat text with numeric values replaced by `#`.
/// - Boolean flags: `"corrupted"`, `"fractured"`, `"unidentified"`
///
/// # Example
///
/// A rare wand with `+32(25-40) to maximum Life` becomes:
/// ```json
/// {
///   "item_class": "Wands",
///   "rarity": "Rare",
///   "base_type": "Driftwood Wand",
///   "item_level": 7,
///   "explicit.+# to maximum Life": 32
/// }
/// ```
#[must_use]
#[allow(clippy::missing_panics_doc, clippy::cast_possible_wrap)]
pub fn item_to_entry(item: &ResolvedItem) -> Entry {
    let mut map: HashMap<String, serde_json::Value> = HashMap::new();

    // --- Header ---
    map.insert(
        "item_class".into(),
        serde_json::json!(item.header.item_class),
    );
    map.insert(
        "rarity".into(),
        serde_json::json!(rarity_str(item.header.rarity)),
    );
    map.insert(
        "rarity_class".into(),
        serde_json::json!(if item.header.rarity == Rarity::Unique {
            "Unique"
        } else {
            "Non-Unique"
        }),
    );
    map.insert("base_type".into(), serde_json::json!(item.header.base_type));
    if let Some(name) = &item.header.name {
        map.insert("name".into(), serde_json::json!(name));
    }

    // --- Item level ---
    if let Some(ilvl) = item.item_level {
        map.insert("item_level".into(), serde_json::json!(i64::from(ilvl)));
    }

    // --- Boolean flags ---
    map.insert("corrupted".into(), serde_json::json!(item.is_corrupted));
    map.insert("fractured".into(), serde_json::json!(item.is_fractured));
    map.insert(
        "unidentified".into(),
        serde_json::json!(item.is_unidentified),
    );

    // --- Influences ---
    for influence in &item.influences {
        let key = format!("influence.{influence}");
        map.insert(key, serde_json::json!(true));
    }
    map.insert(
        "influence_count".into(),
        serde_json::json!(item.influences.len() as i64),
    );

    // --- Sockets ---
    if let Some(sockets) = &item.sockets {
        let socket_count = sockets.chars().filter(|c| c.is_alphabetic()).count();
        // Links: groups separated by spaces, linked sockets joined by -
        let max_link = sockets
            .split(' ')
            .map(|g| g.chars().filter(|c| c.is_alphabetic()).count())
            .max()
            .unwrap_or(0);
        map.insert(
            "socket_count".into(),
            serde_json::json!(socket_count as i64),
        );
        map.insert("max_link".into(), serde_json::json!(max_link as i64));
    }

    // --- Requirements ---
    for req in &item.requirements {
        if req.key == "Level" {
            if let Ok(v) = req.value.parse::<i64>() {
                map.insert("requirement_level".into(), serde_json::json!(v));
            }
        }
    }

    // --- Mods → stat entries ---
    insert_mods(&mut map, &item.implicits, "implicit");
    insert_mods(&mut map, &item.explicits, "explicit");
    insert_mods(&mut map, &item.enchants, "enchant");

    // Mod counts
    map.insert(
        "implicit_count".into(),
        serde_json::json!(item.implicits.len() as i64),
    );
    map.insert(
        "explicit_count".into(),
        serde_json::json!(item.explicits.len() as i64),
    );

    // Deserialize through JSON to get Entry (which wraps HashMap<String, EntryValue>)
    let json = serde_json::to_string(&map).expect("failed to serialize entry map");
    serde_json::from_str(&json).expect("failed to deserialize as Entry")
}

/// Insert stat lines from a list of mods into the entry map.
fn insert_mods(
    map: &mut HashMap<String, serde_json::Value>,
    mods: &[ResolvedMod],
    default_source: &str,
) {
    for m in mods {
        let source = match m.display_type {
            ModDisplayType::Crafted => "crafted",
            _ => default_source,
        };

        for stat_line in &m.stat_lines {
            if stat_line.is_reminder {
                continue;
            }
            insert_stat_line(map, stat_line, source);
        }
    }
}

/// Insert a single stat line into the entry map.
///
/// Uses `stat_ids` when available (one entry per `stat_id` with its corresponding value).
/// Falls back to template text key with first numeric value.
fn insert_stat_line(
    map: &mut HashMap<String, serde_json::Value>,
    stat_line: &ResolvedStatLine,
    source: &str,
) {
    // Strategy 1: Use stat_ids (preferred — stable, language-independent)
    if let (Some(stat_ids), Some(stat_values)) = (&stat_line.stat_ids, &stat_line.stat_values) {
        for (stat_id, value) in stat_ids.iter().zip(stat_values.iter()) {
            let key = format!("{source}.{stat_id}");
            map.insert(key, serde_json::json!(value));
        }
        return;
    }

    // Strategy 2: Use template text (fallback — extract template from display_text)
    if let Some((template, value)) = extract_template(&stat_line.display_text, &stat_line.values) {
        let key = format!("{source}.{template}");
        map.insert(key, serde_json::json!(value));
    }
}

/// Extract a template string and first value from `display_text`.
///
/// Replaces known numeric values with `#` to produce a matchable template.
/// Example: `"+32 to maximum Life"` with values `[{current: 32, ...}]`
///          → `("+# to maximum Life", 32)`
fn extract_template(
    display_text: &str,
    values: &[poe_item::types::ValueRange],
) -> Option<(String, i64)> {
    if values.is_empty() {
        return None;
    }

    let mut template = display_text.to_owned();
    let first_value = values[0].current;

    // Replace each known value with # (in order, first occurrence only)
    for vr in values {
        let val_str = vr.current.to_string();
        if let Some(pos) = template.find(&val_str) {
            template.replace_range(pos..pos + val_str.len(), "#");
        }
    }

    if template.is_empty() {
        return None;
    }

    Some((template, first_value))
}

fn rarity_str(rarity: Rarity) -> &'static str {
    match rarity {
        Rarity::Normal => "Normal",
        Rarity::Magic => "Magic",
        Rarity::Rare => "Rare",
        Rarity::Unique => "Unique",
        Rarity::Gem => "Gem",
        Rarity::Currency => "Currency",
        Rarity::DivinationCard => "Divination Card",
        Rarity::Unknown => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use poe_item::types::*;

    fn make_stat_line(
        display_text: &str,
        values: Vec<ValueRange>,
        stat_ids: Option<Vec<String>>,
        stat_values: Option<Vec<i64>>,
    ) -> ResolvedStatLine {
        ResolvedStatLine {
            raw_text: display_text.to_owned(),
            display_text: display_text.to_owned(),
            values,
            stat_ids,
            stat_values,
            is_reminder: false,
            is_unscalable: false,
        }
    }

    fn make_mod(display_type: ModDisplayType, stat_lines: Vec<ResolvedStatLine>) -> ResolvedMod {
        ResolvedMod {
            header: ModHeader {
                source: ModSource::Regular,
                slot: ModSlot::Prefix,
                influence_tier: None,
                name: None,
                tier: None,
                tags: vec![],
            },
            stat_lines,
            is_fractured: false,
            display_type,
        }
    }

    fn make_item(explicits: Vec<ResolvedMod>) -> ResolvedItem {
        ResolvedItem {
            header: ResolvedHeader {
                item_class: "Boots".into(),
                rarity: Rarity::Rare,
                name: Some("Test Boots".into()),
                base_type: "Titan Greaves".into(),
            },
            item_level: Some(75),
            monster_level: None,
            talisman_tier: None,
            requirements: vec![Requirement {
                key: "Level".into(),
                value: "68".into(),
            }],
            sockets: Some("R-R-G B".into()),
            experience: None,
            properties: vec![],
            implicits: vec![],
            explicits,
            enchants: vec![],
            influences: vec![],
            statuses: vec![],
            is_corrupted: false,
            is_fractured: false,
            is_unidentified: false,
            note: None,
            description: None,
            flavor_text: None,
            gem_data: None,
            unclassified_sections: vec![],
        }
    }

    #[test]
    fn basic_item_metadata() {
        let item = make_item(vec![]);
        let entry = item_to_entry(&item);

        assert_eq!(
            entry.get("item_class"),
            Some(&poe_rqe::eval::EntryValue::String("Boots".into()))
        );
        assert_eq!(
            entry.get("rarity"),
            Some(&poe_rqe::eval::EntryValue::String("Rare".into()))
        );
        assert_eq!(
            entry.get("base_type"),
            Some(&poe_rqe::eval::EntryValue::String("Titan Greaves".into()))
        );
        assert_eq!(
            entry.get("item_level"),
            Some(&poe_rqe::eval::EntryValue::Integer(75))
        );
        assert_eq!(
            entry.get("corrupted"),
            Some(&poe_rqe::eval::EntryValue::Boolean(false))
        );
        assert_eq!(
            entry.get("socket_count"),
            Some(&poe_rqe::eval::EntryValue::Integer(4))
        );
        assert_eq!(
            entry.get("max_link"),
            Some(&poe_rqe::eval::EntryValue::Integer(3))
        );
        assert_eq!(
            entry.get("requirement_level"),
            Some(&poe_rqe::eval::EntryValue::Integer(68))
        );
    }

    #[test]
    fn stat_ids_preferred_over_template() {
        let stat = make_stat_line(
            "+32 to maximum Life",
            vec![ValueRange {
                current: 32,
                min: 25,
                max: 40,
            }],
            Some(vec!["base_maximum_life".into()]),
            Some(vec![32]),
        );
        let m = make_mod(ModDisplayType::Prefix, vec![stat]);
        let item = make_item(vec![m]);
        let entry = item_to_entry(&item);

        // Should use stat_id key, not template
        assert_eq!(
            entry.get("explicit.base_maximum_life"),
            Some(&poe_rqe::eval::EntryValue::Integer(32))
        );
        // Template key should NOT exist
        assert!(entry.get("explicit.+# to maximum Life").is_none());
    }

    #[test]
    fn template_fallback_when_no_stat_ids() {
        let stat = make_stat_line(
            "+32 to maximum Life",
            vec![ValueRange {
                current: 32,
                min: 25,
                max: 40,
            }],
            None,
            None,
        );
        let m = make_mod(ModDisplayType::Prefix, vec![stat]);
        let item = make_item(vec![m]);
        let entry = item_to_entry(&item);

        assert_eq!(
            entry.get("explicit.+# to maximum Life"),
            Some(&poe_rqe::eval::EntryValue::Integer(32))
        );
    }

    #[test]
    fn multi_value_stat_ids() {
        let stat = make_stat_line(
            "Adds 1 to 4 Lightning Damage",
            vec![
                ValueRange {
                    current: 1,
                    min: 1,
                    max: 3,
                },
                ValueRange {
                    current: 4,
                    min: 3,
                    max: 5,
                },
            ],
            Some(vec![
                "attack_minimum_added_lightning_damage".into(),
                "attack_maximum_added_lightning_damage".into(),
            ]),
            Some(vec![1, 4]),
        );
        let m = make_mod(ModDisplayType::Prefix, vec![stat]);
        let item = make_item(vec![m]);
        let entry = item_to_entry(&item);

        assert_eq!(
            entry.get("explicit.attack_minimum_added_lightning_damage"),
            Some(&poe_rqe::eval::EntryValue::Integer(1))
        );
        assert_eq!(
            entry.get("explicit.attack_maximum_added_lightning_damage"),
            Some(&poe_rqe::eval::EntryValue::Integer(4))
        );
    }

    #[test]
    fn reminder_text_excluded() {
        let reminder = ResolvedStatLine {
            raw_text: "(Only Damage from Hits can be Recouped)".into(),
            display_text: "(Only Damage from Hits can be Recouped)".into(),
            values: vec![],
            stat_ids: None,
            stat_values: None,
            is_reminder: true,
            is_unscalable: false,
        };
        let m = make_mod(ModDisplayType::Implicit, vec![reminder]);
        let item = make_item(vec![m]);
        let entry = item_to_entry(&item);

        // Reminder text should not appear as an entry key
        assert!(
            entry
                .get("explicit.(Only Damage from Hits can be Recouped)")
                .is_none()
        );
    }

    #[test]
    fn crafted_source_prefix() {
        let stat = make_stat_line(
            "+22 to Lightning Resistance",
            vec![ValueRange {
                current: 22,
                min: 21,
                max: 28,
            }],
            Some(vec!["base_lightning_damage_resistance_%".into()]),
            Some(vec![22]),
        );
        let m = make_mod(ModDisplayType::Crafted, vec![stat]);
        let item = make_item(vec![m]);
        let entry = item_to_entry(&item);

        // Crafted mods should use "crafted" prefix
        assert_eq!(
            entry.get("crafted.base_lightning_damage_resistance_%"),
            Some(&poe_rqe::eval::EntryValue::Integer(22))
        );
    }

    #[test]
    fn rarity_class_unique() {
        let mut item = make_item(vec![]);
        item.header.rarity = Rarity::Unique;
        let entry = item_to_entry(&item);

        assert_eq!(
            entry.get("rarity_class"),
            Some(&poe_rqe::eval::EntryValue::String("Unique".into()))
        );
    }

    #[test]
    fn rarity_class_non_unique() {
        let item = make_item(vec![]);
        let entry = item_to_entry(&item);

        assert_eq!(
            entry.get("rarity_class"),
            Some(&poe_rqe::eval::EntryValue::String("Non-Unique".into()))
        );
    }
}
