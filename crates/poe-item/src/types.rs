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
pub struct Requirement {
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
/// stat IDs resolved.
#[derive(Debug, Clone)]
pub struct ResolvedItem {
    pub header: ResolvedHeader,
    pub item_level: Option<u32>,
    pub monster_level: Option<u32>,
    pub talisman_tier: Option<u32>,
    pub requirements: Vec<Requirement>,
    pub sockets: Option<String>,
    pub experience: Option<String>,
    pub mods: Vec<ResolvedMod>,
    pub influences: Vec<InfluenceKind>,
    pub statuses: Vec<StatusKind>,
    /// Unresolved generic sections (properties, flavor text, usage text, etc.)
    pub properties: Vec<Vec<String>>,
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
