//! Predicate schema — describes what predicates exist and what inputs they need.
//!
//! The app's profile editor UI is built dynamically from this schema.
//! When a new predicate is added to `predicate.rs`, a matching schema
//! entry should be added here. A unit test catches drift.

use serde::Serialize;

use crate::predicate::Cmp;

/// Describes one predicate type for the UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PredicateSchema {
    /// Predicate type name (matches serde `"type"` tag in `Predicate`).
    pub type_name: String,
    /// Human-readable label.
    pub label: String,
    /// Tooltip description.
    pub description: String,
    /// Category for grouping in UI (e.g., "Header", "Mods", "Influence").
    pub category: String,
    /// Ordered list of input fields the user fills in.
    pub fields: Vec<PredicateField>,
}

/// A single input field within a predicate.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PredicateField {
    /// JSON key name (matches serde field name in `Predicate`).
    pub name: String,
    /// Human-readable label.
    pub label: String,
    /// What kind of widget to render.
    pub kind: FieldKind,
}

/// Widget type for a predicate field.
///
/// The UI maps each variant to a specific widget. New predicates that
/// use existing field kinds get UI for free.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FieldKind {
    /// Comparison operator dropdown.
    /// `allowed_ops` restricts which operators are valid.
    Comparison {
        #[serde(rename = "allowedOps")]
        allowed_ops: Vec<Cmp>,
    },
    /// Numeric input.
    Number {
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        min: Option<i64>,
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        max: Option<i64>,
    },
    /// Fixed set of choices (only == / != comparison makes sense).
    Enum { options: Vec<EnumOption> },
    /// Ordered set of choices (>=, <= comparisons are meaningful).
    OrderedEnum { options: Vec<EnumOption> },
    /// Text input with optional autocomplete from a data source.
    /// `suggestions_from` names a source the app resolves via `get_suggestions`.
    Text {
        #[serde(rename = "suggestionsFrom")]
        suggestions_from: Option<String>,
    },
    /// Mod slot dropdown (Prefix / Suffix / Implicit).
    Slot { options: Vec<EnumOption> },
}

/// A selectable option in an enum/ordered-enum field.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EnumOption {
    /// Serialized value (matches serde representation).
    pub value: String,
    /// Human-readable label.
    pub label: String,
}

/// All comparison operators.
const ALL_CMP: [Cmp; 6] = [Cmp::Eq, Cmp::Ne, Cmp::Gt, Cmp::Ge, Cmp::Lt, Cmp::Le];
/// Numeric comparison operators (no != since it's rarely useful).
const NUM_CMP: [Cmp; 5] = [Cmp::Eq, Cmp::Ge, Cmp::Gt, Cmp::Le, Cmp::Lt];

/// Return the schema for all predicates.
///
/// This is the contract between poe-eval and the app. The app builds
/// profile editor UI from this schema without hardcoding predicate types.
#[must_use]
pub fn predicate_schema() -> Vec<PredicateSchema> {
    vec![
        // ── Header predicates ────────────────────────────────────────
        PredicateSchema {
            type_name: "Rarity".into(),
            label: "Rarity".into(),
            description: "Item rarity (Normal < Magic < Rare; Unique is separate)".into(),
            category: "Header".into(),
            fields: vec![
                comparison_field(&ALL_CMP),
                PredicateField {
                    name: "value".into(),
                    label: "Rarity".into(),
                    kind: FieldKind::OrderedEnum {
                        options: vec![
                            opt("Normal", "Normal"),
                            opt("Magic", "Magic"),
                            opt("Rare", "Rare"),
                            opt("Unique", "Unique"),
                            opt("Gem", "Gem"),
                            opt("Currency", "Currency"),
                        ],
                    },
                },
            ],
        },
        PredicateSchema {
            type_name: "ItemClass".into(),
            label: "Item Class".into(),
            description: "Equipment category (e.g., Body Armours, Boots, Rings)".into(),
            category: "Header".into(),
            fields: vec![
                comparison_field(&[Cmp::Eq, Cmp::Ne]),
                PredicateField {
                    name: "value".into(),
                    label: "Class".into(),
                    kind: FieldKind::Text {
                        suggestions_from: Some("item_classes".into()),
                    },
                },
            ],
        },
        PredicateSchema {
            type_name: "BaseType".into(),
            label: "Base Type".into(),
            description: "Exact base type match (e.g., Vaal Regalia)".into(),
            category: "Header".into(),
            fields: vec![
                comparison_field(&[Cmp::Eq, Cmp::Ne]),
                PredicateField {
                    name: "value".into(),
                    label: "Base".into(),
                    kind: FieldKind::Text {
                        suggestions_from: Some("base_types".into()),
                    },
                },
            ],
        },
        PredicateSchema {
            type_name: "BaseTypeContains".into(),
            label: "Base Type Contains".into(),
            description: "Base type substring match (e.g., \"Regalia\" matches Vaal Regalia)"
                .into(),
            category: "Header".into(),
            fields: vec![PredicateField {
                name: "value".into(),
                label: "Text".into(),
                kind: FieldKind::Text {
                    suggestions_from: Some("base_types".into()),
                },
            }],
        },
        // ── Numeric item properties ──────────────────────────────────
        PredicateSchema {
            type_name: "ItemLevel".into(),
            label: "Item Level".into(),
            description: "Item level (affects available mod tiers)".into(),
            category: "Properties".into(),
            fields: vec![
                comparison_field(&NUM_CMP),
                PredicateField {
                    name: "value".into(),
                    label: "Level".into(),
                    kind: FieldKind::Number {
                        min: Some(1),
                        max: Some(100),
                    },
                },
            ],
        },
        // ── Mod predicates ───────────────────────────────────────────
        PredicateSchema {
            type_name: "ModCount".into(),
            label: "Mod Count".into(),
            description: "Number of mods in a given slot".into(),
            category: "Mods".into(),
            fields: vec![
                slot_field(),
                comparison_field(&NUM_CMP),
                PredicateField {
                    name: "value".into(),
                    label: "Count".into(),
                    kind: FieldKind::Number {
                        min: Some(0),
                        max: Some(6),
                    },
                },
            ],
        },
        PredicateSchema {
            type_name: "OpenMods".into(),
            label: "Open Mod Slots".into(),
            description: "Available (unfilled) mod slots. Requires game data for max.".into(),
            category: "Mods".into(),
            fields: vec![
                slot_field(),
                comparison_field(&NUM_CMP),
                PredicateField {
                    name: "value".into(),
                    label: "Count".into(),
                    kind: FieldKind::Number {
                        min: Some(0),
                        max: Some(3),
                    },
                },
            ],
        },
        PredicateSchema {
            type_name: "HasModNamed".into(),
            label: "Has Mod Named".into(),
            description: "Whether any mod has a specific name (e.g., \"Merciless\")".into(),
            category: "Mods".into(),
            fields: vec![PredicateField {
                name: "name".into(),
                label: "Mod Name".into(),
                kind: FieldKind::Text {
                    suggestions_from: Some("mod_names".into()),
                },
            }],
        },
        PredicateSchema {
            type_name: "ModTier".into(),
            label: "Mod Tier".into(),
            description: "Tier of a named mod (T1 = best for regular, R1 = worst for bench)".into(),
            category: "Mods".into(),
            fields: vec![
                PredicateField {
                    name: "name".into(),
                    label: "Mod Name".into(),
                    kind: FieldKind::Text {
                        suggestions_from: Some("mod_names".into()),
                    },
                },
                comparison_field(&NUM_CMP),
                PredicateField {
                    name: "value".into(),
                    label: "Tier".into(),
                    kind: FieldKind::Number {
                        min: Some(1),
                        max: Some(20),
                    },
                },
            ],
        },
        // ── Stat value predicates ────────────────────────────────────
        PredicateSchema {
            type_name: "StatValue".into(),
            label: "Mod Stat Value".into(),
            description:
                "Rolled value of a mod's stat. 1 condition = any mod; 2+ = all on same mod (hybrid).".into(),
            category: "Mods".into(),
            fields: vec![
                PredicateField {
                    name: "text".into(),
                    label: "Stat".into(),
                    kind: FieldKind::Text {
                        suggestions_from: Some("stat_texts".into()),
                    },
                },
                // stat_id and value_index are auto-resolved (hidden from UI).
                comparison_field(&NUM_CMP),
                PredicateField {
                    name: "value".into(),
                    label: "Value".into(),
                    kind: FieldKind::Number {
                        min: None,
                        max: None,
                    },
                },
            ],
        },
        PredicateSchema {
            type_name: "RollPercent".into(),
            label: "Roll Quality %".into(),
            description: "How close a mod's roll is to max (0-100%). Matches by stat ID.".into(),
            category: "Mods".into(),
            fields: vec![
                PredicateField {
                    name: "text".into(),
                    label: "Stat".into(),
                    kind: FieldKind::Text {
                        suggestions_from: Some("stat_texts".into()),
                    },
                },
                // stat_id and value_index are auto-resolved (hidden from UI).
                comparison_field(&NUM_CMP),
                PredicateField {
                    name: "value".into(),
                    label: "Percent".into(),
                    kind: FieldKind::Number {
                        min: Some(0),
                        max: Some(100),
                    },
                },
            ],
        },
        // ── Influence / status predicates ────────────────────────────
        PredicateSchema {
            type_name: "HasInfluence".into(),
            label: "Has Influence".into(),
            description: "Whether the item has a specific influence type".into(),
            category: "Influence".into(),
            fields: vec![PredicateField {
                name: "influence".into(),
                label: "Influence".into(),
                kind: FieldKind::Enum {
                    options: vec![
                        opt("Shaper", "Shaper"),
                        opt("Elder", "Elder"),
                        opt("Crusader", "Crusader"),
                        opt("Hunter", "Hunter"),
                        opt("Redeemer", "Redeemer"),
                        opt("Warlord", "Warlord"),
                        opt("SearingExarch", "Searing Exarch"),
                        opt("EaterOfWorlds", "Eater of Worlds"),
                        opt("Synthesised", "Synthesised"),
                        opt("Fractured", "Fractured"),
                    ],
                },
            }],
        },
        PredicateSchema {
            type_name: "HasStatus".into(),
            label: "Has Status".into(),
            description: "Whether the item has a specific status".into(),
            category: "Status".into(),
            fields: vec![PredicateField {
                name: "status".into(),
                label: "Status".into(),
                kind: FieldKind::Enum {
                    options: vec![
                        opt("Corrupted", "Corrupted"),
                        opt("Mirrored", "Mirrored"),
                        opt("Unmodifiable", "Unmodifiable"),
                        opt("Split", "Split"),
                        opt("Transfigured", "Transfigured"),
                    ],
                },
            }],
        },
        PredicateSchema {
            type_name: "InfluenceCount".into(),
            label: "Influence Count".into(),
            description: "Total number of influences on the item".into(),
            category: "Influence".into(),
            fields: vec![
                comparison_field(&NUM_CMP),
                PredicateField {
                    name: "value".into(),
                    label: "Count".into(),
                    kind: FieldKind::Number {
                        min: Some(0),
                        max: Some(4),
                    },
                },
            ],
        },
        // ── Socket / quality predicates ─────────────────────────────
        PredicateSchema {
            type_name: "SocketCount".into(),
            label: "Socket Count".into(),
            description: "Total number of sockets on the item".into(),
            category: "Properties".into(),
            fields: vec![
                comparison_field(&NUM_CMP),
                PredicateField {
                    name: "value".into(),
                    label: "Count".into(),
                    kind: FieldKind::Number {
                        min: Some(0),
                        max: Some(6),
                    },
                },
            ],
        },
        PredicateSchema {
            type_name: "LinkCount".into(),
            label: "Link Count".into(),
            description: "Largest linked socket group on the item".into(),
            category: "Properties".into(),
            fields: vec![
                comparison_field(&NUM_CMP),
                PredicateField {
                    name: "value".into(),
                    label: "Count".into(),
                    kind: FieldKind::Number {
                        min: Some(1),
                        max: Some(6),
                    },
                },
            ],
        },
        PredicateSchema {
            type_name: "Quality".into(),
            label: "Quality".into(),
            description: "Item quality percentage".into(),
            category: "Properties".into(),
            fields: vec![
                comparison_field(&NUM_CMP),
                PredicateField {
                    name: "value".into(),
                    label: "Quality %".into(),
                    kind: FieldKind::Number {
                        min: Some(0),
                        max: Some(30),
                    },
                },
            ],
        },
    ]
}

// ── Helpers ──────────────────────────────────────────────────────────

fn opt(value: &str, label: &str) -> EnumOption {
    EnumOption {
        value: value.into(),
        label: label.into(),
    }
}

fn comparison_field(ops: &[Cmp]) -> PredicateField {
    PredicateField {
        name: "op".into(),
        label: "Operator".into(),
        kind: FieldKind::Comparison {
            allowed_ops: ops.to_vec(),
        },
    }
}

fn slot_field() -> PredicateField {
    PredicateField {
        name: "slot".into(),
        label: "Slot".into(),
        kind: FieldKind::Slot {
            options: vec![
                opt("Prefix", "Prefix"),
                opt("Suffix", "Suffix"),
                opt("Implicit", "Implicit"),
            ],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_has_all_predicates() {
        let schema = predicate_schema();
        assert_eq!(schema.len(), 17, "schema should have exactly 17 predicates");
    }

    #[test]
    fn no_duplicate_type_names() {
        let schema = predicate_schema();
        let mut names: Vec<&str> = schema.iter().map(|s| s.type_name.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 17, "all type names should be unique");
    }

    #[test]
    fn schema_type_names_match_predicate_variants() {
        // Verify each schema type_name can be deserialized as a Predicate
        // by constructing minimal JSON with the right tag + required fields.
        use crate::predicate::Predicate;

        let test_cases = [
            r#"{"type":"Rarity","op":"Eq","value":"Normal"}"#,
            r#"{"type":"ItemClass","op":"Eq","value":"Boots"}"#,
            r#"{"type":"BaseType","op":"Eq","value":"Iron Hat"}"#,
            r#"{"type":"BaseTypeContains","value":"Hat"}"#,
            r#"{"type":"ItemLevel","op":"Ge","value":1}"#,
            r#"{"type":"ModCount","slot":"Prefix","op":"Ge","value":1}"#,
            r#"{"type":"OpenMods","slot":"Suffix","op":"Ge","value":1}"#,
            r#"{"type":"HasModNamed","name":"Test"}"#,
            r#"{"type":"ModTier","name":"Test","op":"Le","value":3}"#,
            r#"{"type":"StatValue","conditions":[{"stat_ids":["base_maximum_life"],"value_index":0,"op":"Ge","value":50}]}"#,
            r#"{"type":"RollPercent","text":"Life","stat_ids":["base_maximum_life"],"value_index":0,"op":"Ge","value":80}"#,
            r#"{"type":"HasInfluence","influence":"Shaper"}"#,
            r#"{"type":"HasStatus","status":"Corrupted"}"#,
            r#"{"type":"InfluenceCount","op":"Ge","value":1}"#,
            r#"{"type":"SocketCount","op":"Ge","value":4}"#,
            r#"{"type":"LinkCount","op":"Ge","value":5}"#,
            r#"{"type":"Quality","op":"Ge","value":20}"#,
        ];

        let schema = predicate_schema();
        assert_eq!(
            test_cases.len(),
            schema.len(),
            "test cases should cover all schema entries"
        );

        for (i, json) in test_cases.iter().enumerate() {
            let result = serde_json::from_str::<Predicate>(json);
            assert!(
                result.is_ok(),
                "schema[{}] '{}' failed to deserialize: {:?}",
                i,
                schema[i].type_name,
                result.err()
            );
        }
    }

    #[test]
    fn schema_serializes_to_json() {
        let schema = predicate_schema();
        let json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(json.contains("\"typeName\""), "missing typeName key");
        assert!(json.contains("\"fields\""), "missing fields key");
        // Text fields with suggestions reference a data source
        assert!(
            json.contains("\"item_classes\""),
            "missing item_classes suggestion source"
        );
        assert!(
            json.contains("\"stat_texts\""),
            "missing stat_texts suggestion source"
        );
    }
}
