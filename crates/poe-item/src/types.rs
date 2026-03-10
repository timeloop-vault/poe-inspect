/// Raw parsed item — output of Pass 1 (PEST grammar + tree walker).
///
/// Contains structured data extracted from the parse tree, but no game-data
/// lookups have been performed yet. Magic item base types are not split,
/// stat IDs are not resolved, etc.
#[derive(Debug, Clone)]
pub struct RawItem {
    pub header: Header,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone)]
pub struct Header {
    pub item_class: String,
    pub rarity: Rarity,
    /// First name line. For Rare/Unique this is the item name.
    /// For Normal/Magic this is the base type (or affixed name for Magic).
    pub name1: String,
    /// Second name line (base type). Only present for Rare/Unique.
    pub name2: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum Rarity {
    Normal,
    Magic,
    Rare,
    Unique,
    Gem,
    Currency,
    Unknown,
}

impl From<&str> for Rarity {
    fn from(s: &str) -> Self {
        match s {
            "Normal" => Self::Normal,
            "Magic" => Self::Magic,
            "Rare" => Self::Rare,
            "Unique" => Self::Unique,
            "Gem" => Self::Gem,
            "Currency" => Self::Currency,
            _ => Self::Unknown,
        }
    }
}

/// A parsed section between `--------` separators.
#[derive(Debug, Clone)]
pub enum Section {
    Requirements(Vec<Requirement>),
    Sockets(String),
    ItemLevel(u32),
    MonsterLevel(u32),
    TalismanTier(u32),
    Experience(String),
    Modifiers(ModSection),
    Influence(Vec<InfluenceKind>),
    Status(StatusKind),
    /// Catch-all for unclassified sections (properties, flavor text, enchants, etc.)
    Generic(Vec<String>),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Requirement {
    #[cfg_attr(feature = "serde", serde(rename = "name"))]
    pub key: String,
    pub value: String,
}

/// A section containing one or more modifier groups + optional trailing influence markers.
#[derive(Debug, Clone)]
pub struct ModSection {
    pub groups: Vec<ModGroup>,
    pub trailing_influences: Vec<InfluenceKind>,
}

/// A single modifier: header + body lines.
#[derive(Debug, Clone)]
pub struct ModGroup {
    pub header: ModHeader,
    pub body_lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ModHeader {
    pub source: ModSource,
    pub slot: ModSlot,
    pub influence_tier: Option<String>,
    pub name: Option<String>,
    pub tier: Option<ModTierKind>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModSource {
    Regular,
    MasterCrafted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModSlot {
    Implicit,
    Prefix,
    Suffix,
    Unique,
    SearingExarchImplicit,
    EaterOfWorldsImplicit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModTierKind {
    Tier(u32),
    Rank(u32),
}

impl ModTierKind {
    /// Display classification: tier (regular mods) or rank (bench crafts).
    #[must_use]
    pub fn display_kind(&self) -> TierDisplayKind {
        match self {
            Self::Tier(_) => TierDisplayKind::Tier,
            Self::Rank(_) => TierDisplayKind::Rank,
        }
    }

    /// The tier/rank number.
    #[must_use]
    pub fn number(&self) -> u32 {
        match self {
            Self::Tier(n) | Self::Rank(n) => *n,
        }
    }
}

/// Flat display type combining ModSlot + ModSource for the frontend.
/// "T1" badge shows as Tier, "R1" as Rank. The UI doesn't need to know
/// about SearingExarchImplicit vs EaterOfWorldsImplicit — both are "implicit".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export, rename = "ModType"))]
pub enum ModDisplayType {
    Prefix,
    Suffix,
    Implicit,
    Enchant,
    Unique,
    Crafted,
}

/// Whether a mod number is a tier (regular) or rank (bench craft).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export, rename = "TierKind"))]
pub enum TierDisplayKind {
    Tier,
    Rank,
}

/// A parsed item property line (e.g., "Armour: 890 (augmented)").
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ItemProperty {
    pub name: String,
    pub value: String,
    pub augmented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfluenceKind {
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
    RelicUnique,
}

impl std::fmt::Display for InfluenceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Shaper => write!(f, "Shaper"),
            Self::Elder => write!(f, "Elder"),
            Self::Crusader => write!(f, "Crusader"),
            Self::Hunter => write!(f, "Hunter"),
            Self::Redeemer => write!(f, "Redeemer"),
            Self::Warlord => write!(f, "Warlord"),
            Self::SearingExarch => write!(f, "Searing Exarch"),
            Self::EaterOfWorlds => write!(f, "Eater of Worlds"),
            Self::Synthesised => write!(f, "Synthesised"),
            Self::Fractured => write!(f, "Fractured"),
            Self::RelicUnique => write!(f, "Relic"),
        }
    }
}

impl InfluenceKind {
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Shaper Item" => Some(Self::Shaper),
            "Elder Item" => Some(Self::Elder),
            "Crusader Item" => Some(Self::Crusader),
            "Hunter Item" => Some(Self::Hunter),
            "Redeemer Item" => Some(Self::Redeemer),
            "Warlord Item" => Some(Self::Warlord),
            "Searing Exarch Item" => Some(Self::SearingExarch),
            "Eater of Worlds Item" => Some(Self::EaterOfWorlds),
            "Synthesised Item" => Some(Self::Synthesised),
            "Fractured Item" => Some(Self::Fractured),
            "Relic Unique" => Some(Self::RelicUnique),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Corrupted,
    Mirrored,
    Unmodifiable,
    Split,
    Transfigured,
}

impl StatusKind {
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Corrupted" => Some(Self::Corrupted),
            "Mirrored" => Some(Self::Mirrored),
            "Unmodifiable" => Some(Self::Unmodifiable),
            "Split" => Some(Self::Split),
            "Transfigured" => Some(Self::Transfigured),
            _ => None,
        }
    }
}

// ── Resolved types (Pass 2 output) ─────────────────────────────────────────

/// Fully resolved item — output of Pass 2 (resolver).
///
/// Sections are flattened into typed fields. Value ranges are parsed,
/// type suffixes stripped, and (when `GameData` has a `ReverseIndex`)
/// stat IDs resolved. Properties are parsed, mods are pre-split into
/// implicits/explicits, and convenience booleans are pre-computed.
#[derive(Debug, Clone)]
pub struct ResolvedItem {
    pub header: ResolvedHeader,
    pub item_level: Option<u32>,
    pub monster_level: Option<u32>,
    pub talisman_tier: Option<u32>,
    pub requirements: Vec<Requirement>,
    pub sockets: Option<String>,
    pub experience: Option<String>,
    /// Parsed property lines (e.g., "Armour: 890 (augmented)").
    pub properties: Vec<ItemProperty>,
    /// Implicit mods (including Searing Exarch / Eater of Worlds implicits).
    pub implicits: Vec<ResolvedMod>,
    /// Explicit mods (prefixes, suffixes, unique mods).
    pub explicits: Vec<ResolvedMod>,
    /// Enchant mods (currently always empty — needs section classification in parser).
    pub enchants: Vec<ResolvedMod>,
    pub influences: Vec<InfluenceKind>,
    pub statuses: Vec<StatusKind>,
    /// Whether the item is corrupted.
    pub is_corrupted: bool,
    /// Whether the item is fractured.
    pub is_fractured: bool,
    /// Flavor text (last generic section with no property-like lines).
    pub flavor_text: Option<String>,
    /// Remaining unclassified generic sections.
    pub unclassified_sections: Vec<Vec<String>>,
}

impl ResolvedItem {
    /// All mods in order: enchants, then implicits, then explicits.
    pub fn all_mods(&self) -> impl Iterator<Item = &ResolvedMod> {
        self.enchants
            .iter()
            .chain(self.implicits.iter())
            .chain(self.explicits.iter())
    }
}

/// Resolved header with base type always extracted.
#[derive(Debug, Clone)]
pub struct ResolvedHeader {
    pub item_class: String,
    pub rarity: Rarity,
    /// Item name. Present for Rare/Unique items only.
    pub name: Option<String>,
    /// Base type name. Always present after resolution.
    /// For Magic items, extracted from the affixed name via game data lookup.
    pub base_type: String,
}

/// A modifier with resolved stat lines.
#[derive(Debug, Clone)]
pub struct ResolvedMod {
    pub header: ModHeader,
    pub stat_lines: Vec<ResolvedStatLine>,
    /// Whether this mod has the `(fractured)` suffix on any stat line.
    pub is_fractured: bool,
}

impl ResolvedMod {
    /// Flat display type for the frontend (prefix/suffix/implicit/crafted/unique).
    #[must_use]
    pub fn display_type(&self) -> ModDisplayType {
        match (self.header.slot, self.header.source) {
            (_, ModSource::MasterCrafted) => ModDisplayType::Crafted,
            (ModSlot::Prefix, _) => ModDisplayType::Prefix,
            (ModSlot::Suffix, _) => ModDisplayType::Suffix,
            (
                ModSlot::Implicit | ModSlot::SearingExarchImplicit | ModSlot::EaterOfWorldsImplicit,
                _,
            ) => ModDisplayType::Implicit,
            (ModSlot::Unique, _) => ModDisplayType::Unique,
        }
    }
}

/// A single stat line with parsed values and optional stat ID resolution.
#[derive(Debug, Clone)]
pub struct ResolvedStatLine {
    /// Original text from Ctrl+Alt+C output.
    pub raw_text: String,
    /// Display text with range annotations and type suffixes removed.
    /// Suitable for `ReverseIndex::lookup()`.
    pub display_text: String,
    /// Parsed value ranges from inline annotations like `+32(25-40)`.
    pub values: Vec<ValueRange>,
    /// Resolved stat IDs from `ReverseIndex` lookup. `None` if lookup unavailable or failed.
    pub stat_ids: Option<Vec<String>>,
    /// Raw stat values from `ReverseIndex` (transforms reversed). `None` if lookup failed.
    pub stat_values: Option<Vec<i64>>,
    /// Whether this line is reminder text (parenthesized).
    pub is_reminder: bool,
}

/// A rolled value with its possible range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueRange {
    /// The actual rolled value on this item.
    pub current: i64,
    /// Lower bound of the roll range.
    pub min: i64,
    /// Upper bound of the roll range.
    pub max: i64,
}
