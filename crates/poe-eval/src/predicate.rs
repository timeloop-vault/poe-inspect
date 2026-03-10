//! Atomic predicates — the building blocks of item evaluation rules.
//!
//! Each predicate is a single testable condition against a `ResolvedItem`.
//! Predicates are combined into rules via AND/OR logic in the `rule` module.

use serde::{Deserialize, Serialize};

use poe_item::types::{InfluenceKind, Rarity, StatusKind};

/// A comparison operator for numeric and string fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum Cmp {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

impl Cmp {
    /// Evaluate `lhs <op> rhs` for ordered types.
    pub fn eval<T: PartialOrd>(self, lhs: &T, rhs: &T) -> bool {
        match self {
            Self::Eq => lhs == rhs,
            Self::Ne => lhs != rhs,
            Self::Gt => lhs > rhs,
            Self::Ge => lhs >= rhs,
            Self::Lt => lhs < rhs,
            Self::Le => lhs <= rhs,
        }
    }
}

/// An atomic condition that can be tested against a `ResolvedItem`.
///
/// Predicates are pure data (serializable). They carry no game knowledge —
/// all PoE-specific logic lives in the evaluation layer which queries
/// `GameData` as needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum Predicate {
    // ── Header predicates ────────────────────────────────────────────
    /// Rarity comparison (e.g., `rarity == Rare`, `rarity >= Magic`).
    Rarity { op: Cmp, value: RarityValue },

    /// Item class string match (e.g., `item_class == "Body Armours"`).
    ItemClass { op: Cmp, value: String },

    /// Base type string match (e.g., `base_type == "Vaal Regalia"`).
    BaseType { op: Cmp, value: String },

    /// Base type contains substring (e.g., `base_type contains "Regalia"`).
    BaseTypeContains { value: String },

    // ── Numeric item properties ──────────────────────────────────────
    /// Item level comparison.
    ItemLevel { op: Cmp, value: u32 },

    // ── Mod predicates ───────────────────────────────────────────────
    /// Count of mods in a given slot (e.g., `prefix_count >= 2`).
    ModCount {
        slot: ModSlotKind,
        op: Cmp,
        value: u32,
    },

    /// Open (available) mod slots. Requires `GameData` for max affix lookup.
    /// `open_prefixes >= 1` means "has at least one craftable prefix slot".
    OpenMods {
        slot: ModSlotKind,
        op: Cmp,
        value: u32,
    },

    /// Whether any mod has a specific name (from the `{ }` header).
    HasModNamed { name: String },

    /// Whether any stat line's display text contains the given substring.
    ///
    /// **Deprecated**: Use `StatValue` with `stat_id` instead. Text matching is
    /// fragile (language-dependent, ambiguous). Kept for backward compatibility
    /// with saved profiles.
    HasStatText { text: String },

    /// Whether any stat line has a resolved stat ID matching the given ID.
    ///
    /// **Deprecated**: Use `StatValue` with `op: Ge, value: 0` instead — same
    /// presence check but with an adjustable threshold. Kept for backward
    /// compatibility with saved profiles.
    HasStatId { stat_id: String },

    /// Mod tier comparison — checks if any mod of the given name has tier <op> value.
    ModTier { name: String, op: Cmp, value: u32 },

    // ── Stat value predicates ────────────────────────────────────────
    /// Rolled value of a mod's stat line(s).
    ///
    /// - **1 condition**: matches if ANY mod has a stat line satisfying it.
    /// - **2+ conditions**: matches only if a SINGLE mod satisfies ALL conditions
    ///   (same-mod check — used for hybrid mod detection).
    StatValue { conditions: Vec<StatCondition> },

    /// Roll quality: how close the current roll is to the max, as a percentage.
    /// Matches by `stat_id` (language-independent).
    RollPercent {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stat_id: Option<String>,
        value_index: usize,
        op: Cmp,
        value: u32,
    },

    // ── Influence / status predicates ────────────────────────────────
    /// Whether the item has a specific influence.
    HasInfluence { influence: InfluenceValue },

    /// Whether the item has a specific status (Corrupted, Mirrored, etc.).
    HasStatus { status: StatusValue },

    /// Total number of influences.
    InfluenceCount { op: Cmp, value: u32 },
}

/// A single stat condition: identifies a stat and checks its rolled value.
///
/// Used as the building block for `StatValue` predicates. The `text` field
/// is a display label (the stat template text); `stat_id` is the resolved
/// language-independent identifier used for matching.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StatCondition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stat_id: Option<String>,
    pub value_index: usize,
    pub op: Cmp,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub value: i64,
}

/// Mod slot kind for counting and open-mod queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ModSlotKind {
    Prefix,
    Suffix,
    Implicit,
}

/// Serializable rarity value (maps to `poe_item::types::Rarity`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RarityValue {
    Normal = 0,
    Magic = 1,
    Rare = 2,
    Unique = 3,
    Gem = 4,
    Currency = 5,
    DivinationCard = 6,
}

impl RarityValue {
    #[must_use]
    pub fn from_rarity(r: Rarity) -> Self {
        match r {
            Rarity::Magic => Self::Magic,
            Rarity::Rare => Self::Rare,
            Rarity::Unique => Self::Unique,
            Rarity::Gem => Self::Gem,
            Rarity::Currency => Self::Currency,
            Rarity::DivinationCard => Self::DivinationCard,
            Rarity::Normal | Rarity::Unknown => Self::Normal,
        }
    }
}

/// Serializable influence kind (maps to `poe_item::types::InfluenceKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum InfluenceValue {
    Shaper,
    Elder,
    Crusader,
    Hunter,
    Redeemer,
    Warlord,
    SearingExarch,
    EaterOfWorlds,
    Synthesised,
    Fractured,
}

impl InfluenceValue {
    #[must_use]
    pub fn matches(self, kind: InfluenceKind) -> bool {
        matches!(
            (self, kind),
            (Self::Shaper, InfluenceKind::Shaper)
                | (Self::Elder, InfluenceKind::Elder)
                | (Self::Crusader, InfluenceKind::Crusader)
                | (Self::Hunter, InfluenceKind::Hunter)
                | (Self::Redeemer, InfluenceKind::Redeemer)
                | (Self::Warlord, InfluenceKind::Warlord)
                | (Self::SearingExarch, InfluenceKind::SearingExarch)
                | (Self::EaterOfWorlds, InfluenceKind::EaterOfWorlds)
                | (Self::Synthesised, InfluenceKind::Synthesised)
                | (Self::Fractured, InfluenceKind::Fractured)
        )
    }
}

/// Serializable status kind (maps to `poe_item::types::StatusKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum StatusValue {
    Corrupted,
    Mirrored,
    Unmodifiable,
    Split,
    Transfigured,
    Unidentified,
}

impl StatusValue {
    #[must_use]
    pub fn matches(self, kind: StatusKind) -> bool {
        matches!(
            (self, kind),
            (Self::Corrupted, StatusKind::Corrupted)
                | (Self::Mirrored, StatusKind::Mirrored)
                | (Self::Unmodifiable, StatusKind::Unmodifiable)
                | (Self::Split, StatusKind::Split)
                | (Self::Transfigured, StatusKind::Transfigured)
                | (Self::Unidentified, StatusKind::Unidentified)
        )
    }
}
