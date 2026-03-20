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

    // ── Stat value predicates ────────────────────────────────────────
    /// Rolled value of a mod's stat line(s).
    ///
    /// - **1 condition**: matches if ANY mod has a stat line satisfying it.
    /// - **2+ conditions**: matches only if a SINGLE mod satisfies ALL conditions
    ///   (same-mod check — used for hybrid mod detection).
    StatValue { conditions: Vec<StatCondition> },

    /// Check the tier/rank of the mod that provides a given stat.
    ///
    /// Like `StatValue` but checks the mod's tier number instead of the rolled value.
    /// For pseudo stats, the tier is the worst (highest number) tier among contributing mods.
    StatTier {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        stat_ids: Vec<String>,
        kind: TierKindFilter,
        op: Cmp,
        value: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<ModSourceKind>,
    },

    /// Count mods matching a tier/rank condition.
    ///
    /// "At least N mods with tier/rank <op> value". Replaces the old name-based `ModTier`.
    TierCount {
        kind: TierKindFilter,
        op: Cmp,
        value: u32,
        min_count: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        slot: Option<ModSlotKind>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<ModSourceKind>,
    },

    /// Roll quality: how close the current roll is to the max, as a percentage.
    /// Matches by `stat_ids` (language-independent).
    RollPercent {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        stat_ids: Vec<String>,
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

    // ── Socket / quality predicates ─────────────────────────────────
    /// Total number of sockets on the item.
    SocketCount { op: Cmp, value: u32 },

    /// Largest linked socket group on the item.
    LinkCount { op: Cmp, value: u32 },

    /// Item quality percentage (parsed from properties).
    Quality { op: Cmp, value: u32 },
}

/// A single stat condition: identifies a stat and checks its rolled value.
///
/// Used as the building block for `StatValue` predicates. The `text` field
/// is a display label (the stat template text); `stat_ids` contains one or
/// more equivalent stat IDs (e.g., both the local and non-local variants)
/// so the condition matches items regardless of item slot context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StatCondition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stat_ids: Vec<String>,
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
    /// Matches both Prefix and Suffix (any explicit affix).
    Affix,
}

/// Mod source filter for predicates that can filter by how a mod was acquired.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ModSourceKind {
    /// Only regular (dropped) mods.
    Regular,
    /// Only fractured mods.
    Fractured,
    /// Only master-crafted mods.
    MasterCrafted,
}

/// Whether to match Tier mods, Rank mods, or either.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TierKindFilter {
    /// Only match mods with `Tier(n)`.
    Tier,
    /// Only match mods with `Rank(n)`.
    Rank,
    /// Match either tier or rank.
    Either,
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
