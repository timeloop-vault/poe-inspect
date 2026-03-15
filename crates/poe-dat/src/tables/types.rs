//! Typed structs mirroring specific dat table rows.
//!
//! Only fields needed for item evaluation are included.
//! Field offsets are computed from the dat-schema GQL definitions.

/// A stat definition from `Stats.datc64`.
#[derive(Debug, Clone)]
pub struct StatRow {
    /// Internal stat ID (e.g., "`base_maximum_life`").
    pub id: String,
    /// Whether this stat is local to the item.
    pub is_local: bool,
    /// Whether this stat is local to the weapon.
    pub is_weapon_local: bool,
    /// Whether this stat is virtual (computed, not stored).
    pub is_virtual: bool,
}

/// A tag from `Tags.datc64`.
#[derive(Debug, Clone)]
pub struct TagRow {
    /// Internal tag ID (e.g., "default", "shield", "ring").
    pub id: String,
}

/// A mod family from `ModFamily.datc64`.
#[derive(Debug, Clone)]
pub struct ModFamilyRow {
    /// Family ID (e.g., "`IncreasedLife`").
    pub id: String,
}

/// A mod type entry from `ModType.datc64`.
#[derive(Debug, Clone)]
pub struct ModTypeRow {
    /// Display name (e.g., "Prefix", "Suffix").
    pub name: String,
}

/// An item class from `ItemClasses.datc64`.
#[derive(Debug, Clone)]
pub struct ItemClassRow {
    /// Internal ID (e.g., "`LifeFlask`", "`BodyArmour`").
    pub id: String,
    /// Display name (e.g., "Life Flasks", "Body Armours").
    pub name: String,
    /// FK to `ItemClassCategories` (row index, None if null).
    pub category: Option<u64>,
    /// Whether the item can have veiled mods.
    pub can_have_veiled_mods: bool,
    /// Whether items of this class can be corrupted.
    pub can_be_corrupted: bool,
    /// Whether items of this class can have incubators applied.
    pub can_have_incubators: bool,
    /// Whether items of this class can have influence (Shaper, Elder, etc.).
    pub can_have_influence: bool,
    /// Whether items of this class can be double-corrupted in the Temple.
    pub can_be_double_corrupted: bool,
    /// Whether items of this class can be fractured.
    pub can_be_fractured: bool,
}

/// A base item type from `BaseItemTypes.datc64`.
#[derive(Debug, Clone)]
pub struct BaseItemTypeRow {
    /// Metadata path (e.g., "Metadata/Items/Armours/Boots/BootsStr1").
    pub id: String,
    /// FK to `ItemClasses` (row index).
    pub item_class: Option<u64>,
    /// Grid width in inventory.
    pub width: i32,
    /// Grid height in inventory.
    pub height: i32,
    /// Display name (e.g., "Iron Greaves").
    pub name: String,
    /// Minimum drop level.
    pub drop_level: i32,
    /// FK list to Mods (implicit mod row indices).
    pub implicit_mods: Vec<u64>,
    /// FK list to Tags (tag row indices).
    pub tags: Vec<u64>,
}

/// A rarity definition from `Rarity.datc64`.
///
/// Defines mod limits per rarity (Normal, Magic, Rare, Unique).
#[derive(Debug, Clone)]
pub struct RarityRow {
    /// Internal ID (e.g., "Normal", "Magic", "Rare", "Unique").
    pub id: String,
    /// Minimum number of mods.
    pub min_mods: i32,
    /// Maximum total mods.
    pub max_mods: i32,
    /// Maximum number of prefix mods.
    pub max_prefix: i32,
    /// Maximum number of suffix mods.
    pub max_suffix: i32,
    /// Display text (localized).
    pub text: String,
}

/// An item class category from `ItemClassCategories.datc64`.
///
/// Groups related item classes (e.g., "Weapons", "Armour", "Jewellery").
#[derive(Debug, Clone)]
pub struct ItemClassCategoryRow {
    /// Internal ID (e.g., "Weapons", "Armour").
    pub id: String,
    /// Display text.
    pub text: String,
}

/// Base armour/defence values from `ArmourTypes.datc64`.
///
/// Maps a `BaseItemType` to its base defence values (before quality and mods).
#[derive(Debug, Clone)]
pub struct ArmourTypeRow {
    /// FK to `BaseItemTypes` (row index).
    pub base_item: u64,
    /// Base armour value range.
    pub armour_min: i32,
    pub armour_max: i32,
    /// Base evasion rating range.
    pub evasion_min: i32,
    pub evasion_max: i32,
    /// Base energy shield range.
    pub es_min: i32,
    pub es_max: i32,
    /// Base ward range.
    pub ward_min: i32,
    pub ward_max: i32,
}

/// Base weapon stats from `WeaponTypes.datc64`.
///
/// Maps a `BaseItemType` to its base weapon values.
#[derive(Debug, Clone)]
pub struct WeaponTypeRow {
    /// FK to `BaseItemTypes` (row index).
    pub base_item: u64,
    /// Critical strike chance in hundredths (e.g., 800 = 8.00%).
    pub critical: i32,
    /// Attack speed as ms per attack (e.g., 667 = 1000/667 = 1.50 APS).
    pub speed: i32,
    /// Base physical damage range.
    pub damage_min: i32,
    pub damage_max: i32,
    /// Weapon range in units.
    pub range: i32,
}

/// Base shield block chance from `ShieldTypes.datc64`.
#[derive(Debug, Clone)]
pub struct ShieldTypeRow {
    /// FK to `BaseItemTypes` (row index).
    pub base_item: u64,
    /// Base block chance percentage.
    pub block: i32,
}

/// A client string from `ClientStrings.datc64`.
///
/// GGG's master localization table containing ALL display text used in the
/// game client — property names, status lines, mod header templates, UI labels.
/// This is the authoritative source for item display text.
#[derive(Debug, Clone)]
pub struct ClientStringRow {
    /// Internal string ID (e.g., `"ItemPopupCorrupted"`, `"ItemDisplayArmourArmour"`).
    pub id: String,
    /// Display text (e.g., `"Corrupted"`, `"Armour"`).
    pub text: String,
}

/// A mod entry from `Mods.datc64`.
#[derive(Debug, Clone)]
pub struct ModRow {
    /// Internal mod ID (e.g., "Strength1").
    pub id: String,
    /// FK to `ModType` (row index).
    pub mod_type: Option<u64>,
    /// Required item level.
    pub level: i32,
    /// Up to 6 stat keys (FK to Stats, None if unused).
    pub stat_keys: [Option<u64>; 6],
    /// Generation domain (prefix/suffix/etc.) as raw enum index.
    pub domain: u32,
    /// Mod display name (e.g., "Hale", "of the Yeti").
    pub name: String,
    /// Generation type as raw enum index (1=prefix, 2=suffix, 3=unique, etc.).
    pub generation_type: u32,
    /// FK list to `ModFamily` (row indices).
    pub families: Vec<u64>,
    /// Stat value ranges: [min, max] for each of the 6 stats.
    pub stat_ranges: [(i32, i32); 6],
    /// Spawn weight tags (FK list to Tags).
    pub spawn_weight_tags: Vec<u64>,
    /// Spawn weight values (parallel with `spawn_weight_tags`).
    pub spawn_weight_values: Vec<i32>,
    /// Tags applied to this mod (FK list to Tags).
    pub tags: Vec<u64>,
    /// Whether this mod is essence-only.
    pub is_essence_only: bool,
    /// Maximum level (0 if no max).
    pub max_level: i32,
}
