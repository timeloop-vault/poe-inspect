//! Extract typed rows from raw `DatFile` data.
//!
//! Field offsets are derived from dat-schema GQL definitions for `PoE` 3.28.
//! FK fields are 16 bytes (u64 row index + u64 key hash); we read only
//! the first u64 (row index). Lists are 16 bytes (u64 length + u64 offset).

use super::types::{
    ArmourTypeRow, BaseItemTypeRow, ClientStringRow, ItemClassCategoryRow, ItemClassRow,
    ModFamilyRow, ModRow, ModTypeRow, RarityRow, ShieldTypeRow, StatRow, TagRow, WeaponTypeRow,
};
use crate::dat_reader::DatFile;

// ── Stats ───────────────────────────────────────────────────────────────────
// type Stats { Id: string, _: bool, IsLocal: bool, IsWeaponLocal: bool,
//   Semantics: StatSemantics(enum/u32), _: bool, IsVirtual: bool, ... }
mod stats_offsets {
    pub const ID: usize = 0; // ref|string (8)
    pub const _BOOL1: usize = 8; // bool (1)
    pub const IS_LOCAL: usize = 9; // bool (1)
    pub const IS_WEAPON_LOCAL: usize = 10; // bool (1)
    pub const _SEMANTICS: usize = 11; // enum/u32 (4)
    pub const _BOOL2: usize = 15; // bool (1)
    pub const IS_VIRTUAL: usize = 16; // bool (1)
}

/// Extract all rows from `Stats.datc64`.
pub fn extract_stats(dat: &DatFile) -> Vec<StatRow> {
    (0..dat.row_count)
        .filter_map(|row| {
            Some(StatRow {
                id: dat.read_string(row, stats_offsets::ID)?,
                is_local: dat.read_bool(row, stats_offsets::IS_LOCAL)?,
                is_weapon_local: dat.read_bool(row, stats_offsets::IS_WEAPON_LOCAL)?,
                is_virtual: dat.read_bool(row, stats_offsets::IS_VIRTUAL)?,
            })
        })
        .collect()
}

// ── ArmourTypes ─────────────────────────────────────────────────────────────
// type ArmourTypes { BaseItemTypesKey: FK(16), ArmourMin: i32, ArmourMax: i32,
//   EvasionMin: i32, EvasionMax: i32, ESMin: i32, ESMax: i32,
//   IncreasedMovementSpeed: i32, WardMin: i32, WardMax: i32, _: i32, _: i32 }
// Row size: 60 bytes. Verified against 3.28 Mirage data (2026-03-15).
mod armour_type_offsets {
    pub const BASE_ITEM: usize = 0; // FK (16) — first u64 is row index
    pub const ARMOUR_MIN: usize = 16;
    pub const ARMOUR_MAX: usize = 20;
    pub const EVASION_MIN: usize = 24;
    pub const EVASION_MAX: usize = 28;
    pub const ES_MIN: usize = 32;
    pub const ES_MAX: usize = 36;
    pub const _MOVEMENT_SPEED: usize = 40;
    pub const WARD_MIN: usize = 44;
    pub const WARD_MAX: usize = 48;
}

/// Extract all rows from `ArmourTypes.datc64`.
pub fn extract_armour_types(dat: &DatFile) -> Vec<ArmourTypeRow> {
    (0..dat.row_count)
        .filter_map(|row| {
            Some(ArmourTypeRow {
                base_item: dat.read_fk(row, armour_type_offsets::BASE_ITEM)?,
                armour_min: dat.read_i32(row, armour_type_offsets::ARMOUR_MIN)?,
                armour_max: dat.read_i32(row, armour_type_offsets::ARMOUR_MAX)?,
                evasion_min: dat.read_i32(row, armour_type_offsets::EVASION_MIN)?,
                evasion_max: dat.read_i32(row, armour_type_offsets::EVASION_MAX)?,
                es_min: dat.read_i32(row, armour_type_offsets::ES_MIN)?,
                es_max: dat.read_i32(row, armour_type_offsets::ES_MAX)?,
                ward_min: dat.read_i32(row, armour_type_offsets::WARD_MIN)?,
                ward_max: dat.read_i32(row, armour_type_offsets::WARD_MAX)?,
            })
        })
        .collect()
}

// ── WeaponTypes ─────────────────────────────────────────────────────────────
// type WeaponTypes { BaseItemTypesKey: FK(16), Critical: i32, Speed: i32,
//   DamageMin: i32, DamageMax: i32, RangeMax: i32, _: i32 }
// Row size: 40 bytes. Verified against 3.28 Mirage data (2026-03-15).
mod weapon_type_offsets {
    pub const BASE_ITEM: usize = 0; // FK (16)
    pub const CRITICAL: usize = 16;
    pub const SPEED: usize = 20;
    pub const DAMAGE_MIN: usize = 24;
    pub const DAMAGE_MAX: usize = 28;
    pub const RANGE: usize = 32;
}

/// Extract all rows from `WeaponTypes.datc64`.
pub fn extract_weapon_types(dat: &DatFile) -> Vec<WeaponTypeRow> {
    (0..dat.row_count)
        .filter_map(|row| {
            Some(WeaponTypeRow {
                base_item: dat.read_fk(row, weapon_type_offsets::BASE_ITEM)?,
                critical: dat.read_i32(row, weapon_type_offsets::CRITICAL)?,
                speed: dat.read_i32(row, weapon_type_offsets::SPEED)?,
                damage_min: dat.read_i32(row, weapon_type_offsets::DAMAGE_MIN)?,
                damage_max: dat.read_i32(row, weapon_type_offsets::DAMAGE_MAX)?,
                range: dat.read_i32(row, weapon_type_offsets::RANGE)?,
            })
        })
        .collect()
}

// ── ShieldTypes ─────────────────────────────────────────────────────────────
// type ShieldTypes { BaseItemTypesKey: FK(16), Block: i32 }
// Row size: 20 bytes. Verified against 3.28 Mirage data (2026-03-15).
mod shield_type_offsets {
    pub const BASE_ITEM: usize = 0; // FK (16)
    pub const BLOCK: usize = 16;
}

/// Extract all rows from `ShieldTypes.datc64`.
pub fn extract_shield_types(dat: &DatFile) -> Vec<ShieldTypeRow> {
    (0..dat.row_count)
        .filter_map(|row| {
            Some(ShieldTypeRow {
                base_item: dat.read_fk(row, shield_type_offsets::BASE_ITEM)?,
                block: dat.read_i32(row, shield_type_offsets::BLOCK)?,
            })
        })
        .collect()
}

// ── ClientStrings ───────────────────────────────────────────────────────────
// type ClientStrings { Id: string, Text: string, XBoxText: string,
//   XBoxText2: string, HASH32: i32 }
// Row size: 52 bytes (8+8+8+8+4+16 extra). We only read Id and Text.
mod client_string_offsets {
    pub const ID: usize = 0; // ref|string (8)
    pub const TEXT: usize = 8; // ref|string (8)
}

/// Extract all rows from `ClientStrings.datc64`.
pub fn extract_client_strings(dat: &DatFile) -> Vec<ClientStringRow> {
    (0..dat.row_count)
        .filter_map(|row| {
            Some(ClientStringRow {
                id: dat.read_string(row, client_string_offsets::ID)?,
                text: dat.read_string(row, client_string_offsets::TEXT)?,
            })
        })
        .collect()
}

// ── Tags ────────────────────────────────────────────────────────────────────
// type Tags { Id: string, _: i32, DisplayString: string, Name: string }
mod tags_offsets {
    pub const ID: usize = 0; // ref|string (8)
}

/// Extract all rows from `Tags.datc64`.
pub fn extract_tags(dat: &DatFile) -> Vec<TagRow> {
    (0..dat.row_count)
        .filter_map(|row| {
            Some(TagRow {
                id: dat.read_string(row, tags_offsets::ID)?,
            })
        })
        .collect()
}

// ── ModFamily ───────────────────────────────────────────────────────────────
// type ModFamily { Id: string }
mod mod_family_offsets {
    pub const ID: usize = 0; // ref|string (8)
}

/// Extract all rows from `ModFamily.datc64`.
pub fn extract_mod_families(dat: &DatFile) -> Vec<ModFamilyRow> {
    (0..dat.row_count)
        .filter_map(|row| {
            Some(ModFamilyRow {
                id: dat.read_string(row, mod_family_offsets::ID)?,
            })
        })
        .collect()
}

// ── ModType ─────────────────────────────────────────────────────────────────
// type ModType { Name: string, ... }
mod mod_type_offsets {
    pub const NAME: usize = 0; // ref|string (8)
}

/// Extract all rows from `ModType.datc64`.
pub fn extract_mod_types(dat: &DatFile) -> Vec<ModTypeRow> {
    (0..dat.row_count)
        .filter_map(|row| {
            Some(ModTypeRow {
                name: dat.read_string(row, mod_type_offsets::NAME)?,
            })
        })
        .collect()
}

// ── ItemClasses ─────────────────────────────────────────────────────────────
// type ItemClasses { Id: string, Name: string, ItemClassCategory: FK(16),
//   RemovedIfLeavesArea: bool, _: list(16), IdentifyAchievements: list(16),
//   AllocateToMapOwner: bool, AlwaysAllocate: bool, CanHaveVeiledMods: bool, ... }
mod item_classes_offsets {
    pub const ID: usize = 0; // ref|string (8)
    pub const NAME: usize = 8; // ref|string (8)
    pub const CATEGORY: usize = 16; // FK (16) — u64 row index
    // 32: RemovedIfLeavesArea (bool=1)
    // 33: _ (list=16)
    // 49: IdentifyAchievements (list=16)
    // 65: AllocateToMapOwner (bool=1)
    // 66: AlwaysAllocate (bool=1)
    pub const CAN_HAVE_VEILED: usize = 67; // bool (1)
    // 68: PickedUpQuest (FK=16)
    // 84: _ (i32=4)
    // 88: AlwaysShow (bool=1)
    pub const CAN_BE_CORRUPTED: usize = 89; // bool (1)
    pub const CAN_HAVE_INCUBATORS: usize = 90; // bool (1)
    pub const CAN_HAVE_INFLUENCE: usize = 91; // bool (1)
    pub const CAN_BE_DOUBLE_CORRUPTED: usize = 92; // bool (1)
    // 93: CanHaveAspects (bool=1)
    // 94: CanTransferSkin (bool=1)
    // 95: ItemStance (FK=16)
    // 111: CanScourge (bool=1)
    // 112: CanUpgradeRarity (bool=1)
    // 113-114: _ (bool=1 x2)
    // 115: MaxInventoryDimensions (list=16)
    // 131: Unmodifiable (bool=1)
    pub const CAN_BE_FRACTURED: usize = 132; // bool (1)
    // Byte offsets verified against 3.28 Mirage data (2026-03-15)
}

/// Extract all rows from `ItemClasses.datc64`.
pub fn extract_item_classes(dat: &DatFile) -> Vec<ItemClassRow> {
    (0..dat.row_count)
        .filter_map(|row| {
            Some(ItemClassRow {
                id: dat.read_string(row, item_classes_offsets::ID)?,
                name: dat.read_string(row, item_classes_offsets::NAME)?,
                category: dat.read_fk(row, item_classes_offsets::CATEGORY),
                can_have_veiled_mods: dat
                    .read_bool(row, item_classes_offsets::CAN_HAVE_VEILED)
                    .unwrap_or(false),
                can_be_corrupted: dat
                    .read_bool(row, item_classes_offsets::CAN_BE_CORRUPTED)
                    .unwrap_or(false),
                can_have_incubators: dat
                    .read_bool(row, item_classes_offsets::CAN_HAVE_INCUBATORS)
                    .unwrap_or(false),
                can_have_influence: dat
                    .read_bool(row, item_classes_offsets::CAN_HAVE_INFLUENCE)
                    .unwrap_or(false),
                can_be_double_corrupted: dat
                    .read_bool(row, item_classes_offsets::CAN_BE_DOUBLE_CORRUPTED)
                    .unwrap_or(false),
                can_be_fractured: dat
                    .read_bool(row, item_classes_offsets::CAN_BE_FRACTURED)
                    .unwrap_or(false),
            })
        })
        .collect()
}

// ── BaseItemTypes ───────────────────────────────────────────────────────────
// type BaseItemTypes { Id: string(8), ItemClassesKey: FK(16), Width: i32(4),
//   Height: i32(4), Name: string(8), InheritsFrom: string(8), DropLevel: i32(4),
//   FlavourTextKey: FK(16), Implicit_ModsKeys: list(16), SizeOnGround: i32(4),
//   SoundEffect: FK(16), TagsKeys: list(16), ModDomain: enum/u32(4), ... }
mod base_item_offsets {
    pub const ID: usize = 0; // ref|string (8)
    pub const ITEM_CLASS: usize = 8; // FK (16)
    pub const WIDTH: usize = 24; // i32 (4)
    pub const HEIGHT: usize = 28; // i32 (4)
    pub const NAME: usize = 32; // ref|string (8)
    pub const INHERITS_FROM: usize = 40; // ref|string (8)
    pub const DROP_LEVEL: usize = 48; // i32 (4)
    pub const _FLAVOUR: usize = 52; // FK (16)
    pub const IMPLICIT_MODS: usize = 68; // list|u64 (16)
    pub const _SIZE_ON_GROUND: usize = 84; // i32 (4)
    pub const _SOUND: usize = 88; // FK (16)
    pub const TAGS: usize = 104; // list|u64 (16)
}

/// Extract all rows from `BaseItemTypes.datc64`.
pub fn extract_base_item_types(dat: &DatFile) -> Vec<BaseItemTypeRow> {
    (0..dat.row_count)
        .filter_map(|row| {
            Some(BaseItemTypeRow {
                id: dat.read_string(row, base_item_offsets::ID)?,
                item_class: dat.read_fk(row, base_item_offsets::ITEM_CLASS),
                width: dat.read_i32(row, base_item_offsets::WIDTH)?,
                height: dat.read_i32(row, base_item_offsets::HEIGHT)?,
                name: dat.read_string(row, base_item_offsets::NAME)?,
                inherits_from: dat
                    .read_string(row, base_item_offsets::INHERITS_FROM)
                    .unwrap_or_default(),
                drop_level: dat.read_i32(row, base_item_offsets::DROP_LEVEL)?,
                implicit_mods: dat.read_list_u64(row, base_item_offsets::IMPLICIT_MODS),
                tags: dat.read_list_u64(row, base_item_offsets::TAGS),
            })
        })
        .collect()
}

// ── Rarity ──────────────────────────────────────────────────────────────────
// type Rarity { Id: string, MinMods: i32, MaxMods: i32, _: i32,
//   MaxPrefix: i32, _: i32, MaxSuffix: i32, Color: string, Text: string @localized }
mod rarity_offsets {
    pub const ID: usize = 0; // ref|string (8)
    pub const MIN_MODS: usize = 8; // i32 (4)
    pub const MAX_MODS: usize = 12; // i32 (4)
    // _: i32 @ 16 (skip)
    pub const MAX_PREFIX: usize = 20; // i32 (4)
    // _: i32 @ 24 (skip)
    pub const MAX_SUFFIX: usize = 28; // i32 (4)
    // Color: string @ 32 (skip)
    pub const TEXT: usize = 40; // ref|string (8) @localized
}

/// Extract all rows from `Rarity.datc64`.
pub fn extract_rarity(dat: &DatFile) -> Vec<RarityRow> {
    (0..dat.row_count)
        .filter_map(|row| {
            Some(RarityRow {
                id: dat.read_string(row, rarity_offsets::ID)?,
                min_mods: dat.read_i32(row, rarity_offsets::MIN_MODS).unwrap_or(0),
                max_mods: dat.read_i32(row, rarity_offsets::MAX_MODS).unwrap_or(0),
                max_prefix: dat.read_i32(row, rarity_offsets::MAX_PREFIX).unwrap_or(0),
                max_suffix: dat.read_i32(row, rarity_offsets::MAX_SUFFIX).unwrap_or(0),
                text: dat
                    .read_string(row, rarity_offsets::TEXT)
                    .unwrap_or_default(),
            })
        })
        .collect()
}

// ── ItemClassCategories ─────────────────────────────────────────────────────
// type ItemClassCategories { Id: string @unique, Text: string, _: rid }
mod item_class_categories_offsets {
    pub const ID: usize = 0; // ref|string (8)
    pub const TEXT: usize = 8; // ref|string (8)
}

/// Extract all rows from `ItemClassCategories.datc64`.
pub fn extract_item_class_categories(dat: &DatFile) -> Vec<ItemClassCategoryRow> {
    (0..dat.row_count)
        .filter_map(|row| {
            Some(ItemClassCategoryRow {
                id: dat.read_string(row, item_class_categories_offsets::ID)?,
                text: dat
                    .read_string(row, item_class_categories_offsets::TEXT)
                    .unwrap_or_default(),
            })
        })
        .collect()
}

// ── Mods ────────────────────────────────────────────────────────────────────
// Source: _Core.gql type Mods (PoE1 3.28 / Mirage)
// Row size: 654 bytes. HASH16 is i16 = 2 bytes in datc64.
// Offsets validated against real Strength1 row data.
mod mods_offsets {
    pub const ID: usize = 0; // ref|string (8)
    // HASH16: i16 (2) @ offset 8 — not read
    pub const MOD_TYPE: usize = 10; // FK (16)
    pub const LEVEL: usize = 26; // i32 (4)
    pub const STATS_KEY1: usize = 30; // FK (16)
    pub const STATS_KEY2: usize = 46; // FK (16)
    pub const STATS_KEY3: usize = 62; // FK (16)
    pub const STATS_KEY4: usize = 78; // FK (16)
    pub const DOMAIN: usize = 94; // enum/u32 (4)
    pub const NAME: usize = 98; // ref|string (8)
    pub const GENERATION_TYPE: usize = 106; // enum/u32 (4)
    pub const FAMILIES: usize = 110; // list|u64 (16)
    pub const STAT1_MIN: usize = 126; // i32 (4)
    pub const STAT1_MAX: usize = 130; // i32 (4)
    pub const STAT2_MIN: usize = 134; // i32 (4)
    pub const STAT2_MAX: usize = 138; // i32 (4)
    pub const STAT3_MIN: usize = 142; // i32 (4)
    pub const STAT3_MAX: usize = 146; // i32 (4)
    pub const STAT4_MIN: usize = 150; // i32 (4)
    pub const STAT4_MAX: usize = 154; // i32 (4)
    pub const SPAWN_WEIGHT_TAGS: usize = 158; // list|u64 (16)
    pub const SPAWN_WEIGHT_VALUES: usize = 174; // list|i32 (16)
    pub const TAGS: usize = 190; // list|u64 (16)
    // skip: GrantedEffectsPerLevel(list,16), _(list,16), MonsterMetadata(string,8),
    //   MonsterKillAchievements(list,16), ChestModType(list,16)
    pub const STAT5_MIN: usize = 278; // i32 (4)
    pub const STAT5_MAX: usize = 282; // i32 (4)
    pub const STATS_KEY5: usize = 286; // FK (16)
    // skip: FullAreaClear(list,16), AchievementItems(list,16),
    //   GenWeight_Tags(list,16), GenWeight_Values(list,16), ModifyMapsAch(list,16)
    pub const IS_ESSENCE_ONLY: usize = 382; // bool (1)
    pub const STAT6_MIN: usize = 383; // i32 (4)
    pub const STAT6_MAX: usize = 387; // i32 (4)
    pub const STATS_KEY6: usize = 391; // FK (16)
    pub const MAX_LEVEL: usize = 407; // i32 (4)
}

/// Extract all rows from `Mods.datc64`.
pub fn extract_mods(dat: &DatFile) -> Vec<ModRow> {
    (0..dat.row_count)
        .filter_map(|row| {
            let id = dat.read_string(row, mods_offsets::ID)?;
            let name = dat.read_string(row, mods_offsets::NAME).unwrap_or_default();

            Some(ModRow {
                id,
                mod_type: dat.read_fk(row, mods_offsets::MOD_TYPE),
                level: dat.read_i32(row, mods_offsets::LEVEL).unwrap_or(0),
                stat_keys: [
                    dat.read_fk(row, mods_offsets::STATS_KEY1),
                    dat.read_fk(row, mods_offsets::STATS_KEY2),
                    dat.read_fk(row, mods_offsets::STATS_KEY3),
                    dat.read_fk(row, mods_offsets::STATS_KEY4),
                    dat.read_fk(row, mods_offsets::STATS_KEY5),
                    dat.read_fk(row, mods_offsets::STATS_KEY6),
                ],
                domain: dat.read_u32(row, mods_offsets::DOMAIN).unwrap_or(0),
                name,
                generation_type: dat
                    .read_u32(row, mods_offsets::GENERATION_TYPE)
                    .unwrap_or(0),
                families: dat.read_list_u64(row, mods_offsets::FAMILIES),
                stat_ranges: [
                    (
                        dat.read_i32(row, mods_offsets::STAT1_MIN).unwrap_or(0),
                        dat.read_i32(row, mods_offsets::STAT1_MAX).unwrap_or(0),
                    ),
                    (
                        dat.read_i32(row, mods_offsets::STAT2_MIN).unwrap_or(0),
                        dat.read_i32(row, mods_offsets::STAT2_MAX).unwrap_or(0),
                    ),
                    (
                        dat.read_i32(row, mods_offsets::STAT3_MIN).unwrap_or(0),
                        dat.read_i32(row, mods_offsets::STAT3_MAX).unwrap_or(0),
                    ),
                    (
                        dat.read_i32(row, mods_offsets::STAT4_MIN).unwrap_or(0),
                        dat.read_i32(row, mods_offsets::STAT4_MAX).unwrap_or(0),
                    ),
                    (
                        dat.read_i32(row, mods_offsets::STAT5_MIN).unwrap_or(0),
                        dat.read_i32(row, mods_offsets::STAT5_MAX).unwrap_or(0),
                    ),
                    (
                        dat.read_i32(row, mods_offsets::STAT6_MIN).unwrap_or(0),
                        dat.read_i32(row, mods_offsets::STAT6_MAX).unwrap_or(0),
                    ),
                ],
                spawn_weight_tags: dat.read_list_u64(row, mods_offsets::SPAWN_WEIGHT_TAGS),
                spawn_weight_values: dat.read_list_i32(row, mods_offsets::SPAWN_WEIGHT_VALUES),
                tags: dat.read_list_u64(row, mods_offsets::TAGS),
                is_essence_only: dat
                    .read_bool(row, mods_offsets::IS_ESSENCE_ONLY)
                    .unwrap_or(false),
                max_level: dat.read_i32(row, mods_offsets::MAX_LEVEL).unwrap_or(0),
            })
        })
        .collect()
}
