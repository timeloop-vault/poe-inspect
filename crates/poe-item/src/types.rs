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
pub enum Rarity {
    Normal,
    Magic,
    Rare,
    Unique,
    Gem,
    Currency,
    DivinationCard,
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
            "Divination Card" => Self::DivinationCard,
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
    /// GGG trade pricing annotation: "Note: ~b/o 35 chaos"
    Note(String),
    /// Enchant lines — every line ends with `(enchant)` suffix.
    /// The suffix text is included (not stripped) for downstream stat resolution.
    Enchants(Vec<String>),
    /// Property lines in "Key: Value" format, optionally preceded by a sub-header.
    /// Sub-header is the first line without ": " (weapon type name, gem tags, etc.)
    Properties {
        subheader: Option<String>,
        lines: Vec<RawPropertyLine>,
    },
    /// Catch-all for unclassified sections (flavor text, descriptions, etc.)
    Generic(Vec<String>),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModHeader {
    pub source: ModSource,
    pub slot: ModSlot,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub influence_tier: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub name: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub tier: Option<ModTierKind>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum ModSource {
    Regular,
    MasterCrafted,
    Fractured,
    /// Synthetic mod computed by the resolver (e.g., pseudo stats).
    Computed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum ModSlot {
    Implicit,
    Prefix,
    Suffix,
    Unique,
    SearingExarchImplicit,
    EaterOfWorldsImplicit,
    CorruptionImplicit,
    Enchant,
    /// Synthetic slot for computed pseudo stats.
    Pseudo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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

/// Flat display type combining `ModSlot` + `ModSource` for the frontend.
/// "T1" badge shows as Tier, "R1" as Rank. The UI doesn't need to know
/// about `SearingExarchImplicit` vs `EaterOfWorldsImplicit` — both are "implicit".
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
    /// Computed pseudo stat (aggregated from multiple mods).
    Pseudo,
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

/// A raw property line from Pass 1 — not yet processed for augmented markers.
#[derive(Debug, Clone)]
pub struct RawPropertyLine {
    pub key: String,
    pub value: String,
}

/// A parsed item property line (e.g., "Armour: 890 (augmented)").
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ItemProperty {
    pub name: String,
    pub value: String,
    pub augmented: bool,
    /// Synthetic properties are added by the resolver for trade filter text matching.
    /// They should NOT be rendered in the overlay display — the data is already
    /// shown elsewhere (header, sockets section, etc.).
    #[cfg_attr(feature = "serde", serde(default))]
    pub synthetic: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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

    /// Text as it appears in `PoE` item text (used for trade filter matching).
    #[must_use]
    pub fn as_item_text(self) -> &'static str {
        match self {
            Self::Shaper => "Shaper Item",
            Self::Elder => "Elder Item",
            Self::Crusader => "Crusader Item",
            Self::Hunter => "Hunter Item",
            Self::Redeemer => "Redeemer Item",
            Self::Warlord => "Warlord Item",
            Self::SearingExarch => "Searing Exarch Item",
            Self::EaterOfWorlds => "Eater of Worlds Item",
            Self::Synthesised => "Synthesised Item",
            Self::Fractured => "Fractured Item",
            Self::RelicUnique => "Relic Unique",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum StatusKind {
    Corrupted,
    Mirrored,
    Unmodifiable,
    Split,
    Transfigured,
    Unidentified,
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
            "Unidentified" => Some(Self::Unidentified),
            _ => None,
        }
    }

    /// Text as it appears in `PoE` item text (used for trade filter matching).
    #[must_use]
    pub fn as_item_text(self) -> &'static str {
        match self {
            Self::Corrupted => "Corrupted",
            Self::Mirrored => "Mirrored",
            Self::Unmodifiable => "Unmodifiable",
            Self::Split => "Split",
            Self::Transfigured => "Transfigured",
            Self::Unidentified => "Unidentified",
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
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ResolvedItem {
    pub header: ResolvedHeader,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub item_level: Option<u32>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub monster_level: Option<u32>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub talisman_tier: Option<u32>,
    pub requirements: Vec<Requirement>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub sockets: Option<String>,
    /// Pre-computed socket metadata (total count, max link group).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub socket_info: Option<SocketInfo>,
    /// Item quality percentage (parsed from the `"Quality"` property).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub quality: Option<u32>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub experience: Option<String>,
    /// Parsed property lines (e.g., "Armour: 890 (augmented)").
    pub properties: Vec<ItemProperty>,
    /// Implicit mods (including Searing Exarch / Eater of Worlds implicits).
    pub implicits: Vec<ResolvedMod>,
    /// Explicit mods (prefixes, suffixes, unique mods).
    pub explicits: Vec<ResolvedMod>,
    /// Enchant mods.
    pub enchants: Vec<ResolvedMod>,
    pub influences: Vec<InfluenceKind>,
    pub statuses: Vec<StatusKind>,
    /// Whether the item is corrupted.
    pub is_corrupted: bool,
    /// Whether the item is fractured.
    pub is_fractured: bool,
    /// Whether the item is unidentified (no explicit mods visible).
    pub is_unidentified: bool,
    /// GGG trade pricing annotation (e.g., "~b/o 35 chaos").
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub note: Option<String>,
    /// Item effect/description text (currency effects, scarab effects, gem descriptions, etc.)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub description: Option<String>,
    /// Flavor text (poetic/lore text on uniques, div cards, scarabs).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub flavor_text: Option<String>,
    /// Gem-specific data (tags, stats, quality effects, Vaal variant).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub gem_data: Option<GemData>,
    /// Computed pseudo mods (aggregated from multiple mods).
    /// E.g., "(Pseudo) +142 total maximum Life" summing life mod + strength × 0.5.
    /// These are synthetic `ResolvedMod` entries with `display_type: Pseudo`.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Vec::is_empty"))]
    pub pseudo_mods: Vec<ResolvedMod>,
    /// For unidentified unique items: possible unique names sharing this base type.
    /// Empty for identified items or non-uniques.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Vec::is_empty"))]
    pub unique_candidates: Vec<UniqueCandidate>,
    /// Remaining unclassified generic sections.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Vec::is_empty"))]
    pub unclassified_sections: Vec<Vec<String>>,
}

/// A candidate unique item for disambiguation (unidentified unique picker).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UniqueCandidate {
    /// Unique item name (e.g., `"Headhunter"`).
    pub name: String,
    /// Art filename (e.g., `"headhunter.png"`). Empty if no art available.
    pub art: String,
}

/// Gem-specific structured data extracted from generic sections.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GemData {
    /// Gem tags (e.g., `["Spell", "AoE", "Cold", "Nova"]`).
    pub tags: Vec<String>,
    /// Gem description text.
    pub description: Option<String>,
    /// Gem stat lines (mechanical effects, not property lines).
    pub stats: Vec<String>,
    /// Lines from "Additional Effects From Quality:" section.
    pub quality_stats: Vec<String>,
    /// Vaal variant data (if this is a Vaal gem).
    pub vaal: Option<Box<VaalGemData>>,
}

/// Vaal variant data within a Vaal gem.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct VaalGemData {
    /// Vaal skill name (e.g., "Vaal Ice Nova").
    pub name: String,
    /// Vaal-specific properties (Souls Per Use, Can Store, etc.).
    pub properties: Vec<ItemProperty>,
    /// Vaal skill description.
    pub description: Option<String>,
    /// Vaal stat lines.
    pub stats: Vec<String>,
    /// Vaal quality effect lines.
    pub quality_stats: Vec<String>,
}

impl ResolvedItem {
    /// All mods in order: enchants, then implicits, then explicits, then pseudo mods.
    pub fn all_mods(&self) -> impl Iterator<Item = &ResolvedMod> {
        self.enchants
            .iter()
            .chain(self.implicits.iter())
            .chain(self.explicits.iter())
            .chain(self.pseudo_mods.iter())
    }
}

/// Parsed socket metadata from the socket string.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SocketInfo {
    /// Total number of sockets.
    pub total: u32,
    /// Largest linked socket group.
    pub max_link: u32,
    /// Red socket count.
    pub red: u32,
    /// Green socket count.
    pub green: u32,
    /// Blue socket count.
    pub blue: u32,
    /// White socket count.
    pub white: u32,
}

/// Resolved header with base type always extracted.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ResolvedMod {
    pub header: ModHeader,
    pub stat_lines: Vec<ResolvedStatLine>,
    /// Whether this mod has the `(fractured)` suffix on any stat line.
    pub is_fractured: bool,
    /// Pre-computed display type for the frontend (prefix/suffix/implicit/crafted/unique/enchant).
    pub display_type: ModDisplayType,
}

impl ResolvedMod {
    /// Compute display type from slot + source.
    #[must_use]
    pub fn compute_display_type(slot: ModSlot, source: ModSource) -> ModDisplayType {
        match (slot, source) {
            (_, ModSource::MasterCrafted) => ModDisplayType::Crafted,
            (_, ModSource::Computed) | (ModSlot::Pseudo, _) => ModDisplayType::Pseudo,
            (ModSlot::Prefix, _) => ModDisplayType::Prefix,
            (ModSlot::Suffix, _) => ModDisplayType::Suffix,
            (
                ModSlot::Implicit
                | ModSlot::SearingExarchImplicit
                | ModSlot::EaterOfWorldsImplicit
                | ModSlot::CorruptionImplicit,
                _,
            ) => ModDisplayType::Implicit,
            (ModSlot::Unique, _) => ModDisplayType::Unique,
            (ModSlot::Enchant, _) => ModDisplayType::Enchant,
        }
    }
}

/// A single stat line with parsed values and optional stat ID resolution.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    #[cfg_attr(feature = "ts", ts(type = "Array<number> | null"))]
    pub stat_values: Option<Vec<i64>>,
    /// Whether this line is reminder text (parenthesized).
    pub is_reminder: bool,
    /// Whether this stat has the "Unscalable Value" annotation (fixed, cannot be modified).
    pub is_unscalable: bool,
}

/// A rolled value with its possible range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ValueRange {
    /// The actual rolled value on this item.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub current: i64,
    /// Lower bound of the roll range.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub min: i64,
    /// Upper bound of the roll range.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub max: i64,
}
