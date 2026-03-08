//! Central game data store with indexed lookup tables.
//!
//! Loads the 7 datc64 tables extracted by poe-dat and builds id-based
//! indexes for fast access. FK row indices can be resolved to strings
//! via helper methods.

use std::collections::HashMap;
use std::path::Path;

use poe_dat::dat_reader::DatFile;
use poe_dat::stat_desc::ReverseIndex;
use poe_dat::tables::{
    self, BaseItemTypeRow, ItemClassCategoryRow, ItemClassRow, ModFamilyRow, ModRow, ModTypeRow,
    RarityRow, StatRow, TagRow,
};

// ── GameData ────────────────────────────────────────────────────────────────

/// All game data tables with pre-built indexes.
///
/// Intended to be built once and shared via `Arc<GameData>`.
pub struct GameData {
    // Raw tables (poe-dat row structs, no reshaping)
    pub stats: Vec<StatRow>,
    pub tags: Vec<TagRow>,
    pub item_classes: Vec<ItemClassRow>,
    pub item_class_categories: Vec<ItemClassCategoryRow>,
    pub base_item_types: Vec<BaseItemTypeRow>,
    pub mod_families: Vec<ModFamilyRow>,
    pub mod_types: Vec<ModTypeRow>,
    pub mods: Vec<ModRow>,
    pub rarities: Vec<RarityRow>,

    // Indexes: id string → row index in the corresponding Vec
    stat_by_id: HashMap<String, usize>,
    tag_by_id: HashMap<String, usize>,
    mod_by_id: HashMap<String, usize>,
    item_class_by_id: HashMap<String, usize>,
    base_item_by_name: HashMap<String, usize>,
    rarity_by_id: HashMap<String, usize>,
    item_class_category_by_id: HashMap<String, usize>,

    // Stat description reverse index (display text → stat IDs + values).
    // Optional because loading it requires the raw stat_descriptions.txt.
    pub reverse_index: Option<ReverseIndex>,
}

impl GameData {
    /// Construct `GameData` from pre-loaded table rows.
    ///
    /// Builds all id-based indexes automatically. `reverse_index` is set to `None`;
    /// call `set_reverse_index()` separately if needed.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        stats: Vec<StatRow>,
        tags: Vec<TagRow>,
        item_classes: Vec<ItemClassRow>,
        item_class_categories: Vec<ItemClassCategoryRow>,
        base_item_types: Vec<BaseItemTypeRow>,
        mod_families: Vec<ModFamilyRow>,
        mod_types: Vec<ModTypeRow>,
        mods: Vec<ModRow>,
        rarities: Vec<RarityRow>,
    ) -> Self {
        let stat_by_id = index_by(&stats, |s| s.id.clone());
        let tag_by_id = index_by(&tags, |t| t.id.clone());
        let mod_by_id = index_by(&mods, |m| m.id.clone());
        let item_class_by_id = index_by(&item_classes, |c| c.id.clone());
        let base_item_by_name = index_by(&base_item_types, |b| b.name.clone());
        let rarity_by_id = index_by(&rarities, |r| r.id.clone());
        let item_class_category_by_id = index_by(&item_class_categories, |c| c.id.clone());

        Self {
            stats,
            tags,
            item_classes,
            item_class_categories,
            base_item_types,
            mod_families,
            mod_types,
            mods,
            rarities,
            stat_by_id,
            tag_by_id,
            mod_by_id,
            item_class_by_id,
            base_item_by_name,
            rarity_by_id,
            item_class_category_by_id,
            reverse_index: None,
        }
    }

    /// Set the stat description reverse index.
    pub fn set_reverse_index(&mut self, ri: ReverseIndex) {
        self.reverse_index = Some(ri);
    }

    // ── Lookup by string id ─────────────────────────────────────────────

    pub fn stat(&self, id: &str) -> Option<&StatRow> {
        self.stat_by_id.get(id).map(|&i| &self.stats[i])
    }

    pub fn tag(&self, id: &str) -> Option<&TagRow> {
        self.tag_by_id.get(id).map(|&i| &self.tags[i])
    }

    pub fn mod_by_id(&self, id: &str) -> Option<&ModRow> {
        self.mod_by_id.get(id).map(|&i| &self.mods[i])
    }

    pub fn item_class(&self, id: &str) -> Option<&ItemClassRow> {
        self.item_class_by_id.get(id).map(|&i| &self.item_classes[i])
    }

    pub fn base_item_by_name(&self, name: &str) -> Option<&BaseItemTypeRow> {
        self.base_item_by_name.get(name).map(|&i| &self.base_item_types[i])
    }

    // ── FK resolution (row index → string) ──────────────────────────────

    /// Resolve a stat FK (row index) to the stat's string ID.
    pub fn stat_id(&self, fk: u64) -> Option<&str> {
        self.stats.get(fk as usize).map(|s| s.id.as_str())
    }

    /// Resolve a tag FK (row index) to the tag's string ID.
    pub fn tag_id(&self, fk: u64) -> Option<&str> {
        self.tags.get(fk as usize).map(|t| t.id.as_str())
    }

    /// Resolve a mod family FK (row index) to the family's string ID.
    pub fn mod_family_id(&self, fk: u64) -> Option<&str> {
        self.mod_families.get(fk as usize).map(|f| f.id.as_str())
    }

    /// Resolve a mod type FK (row index) to the type's display name.
    pub fn mod_type_name(&self, fk: u64) -> Option<&str> {
        self.mod_types.get(fk as usize).map(|t| t.name.as_str())
    }

    /// Resolve an item class FK (row index) to the class row.
    pub fn item_class_by_index(&self, fk: u64) -> Option<&ItemClassRow> {
        self.item_classes.get(fk as usize)
    }

    pub fn rarity(&self, id: &str) -> Option<&RarityRow> {
        self.rarity_by_id.get(id).map(|&i| &self.rarities[i])
    }

    pub fn item_class_category(&self, id: &str) -> Option<&ItemClassCategoryRow> {
        self.item_class_category_by_id.get(id).map(|&i| &self.item_class_categories[i])
    }

    /// Resolve an item class category FK (row index) to the category row.
    pub fn item_class_category_by_index(&self, fk: u64) -> Option<&ItemClassCategoryRow> {
        self.item_class_categories.get(fk as usize)
    }

    /// Get the max prefix count for a given rarity ID (e.g., "Rare" → 3).
    pub fn max_prefixes(&self, rarity_id: &str) -> Option<i32> {
        self.rarity(rarity_id).map(|r| r.max_prefix)
    }

    /// Get the max suffix count for a given rarity ID (e.g., "Rare" → 3).
    pub fn max_suffixes(&self, rarity_id: &str) -> Option<i32> {
        self.rarity(rarity_id).map(|r| r.max_suffix)
    }
}

// ── Loading ─────────────────────────────────────────────────────────────────

/// Load `GameData` from a directory of extracted datc64 files.
///
/// Expects files named `{table}.datc64` (lowercase):
/// stats, tags, itemclasses, baseitemtypes, modfamily, modtype, mods.
pub fn load(dat_dir: &Path) -> Result<GameData, LoadError> {
    let stats = load_table(dat_dir, "stats", tables::extract_stats)?;
    let tags = load_table(dat_dir, "tags", tables::extract_tags)?;
    let item_classes = load_table(dat_dir, "itemclasses", tables::extract_item_classes)?;
    let item_class_categories = load_table(dat_dir, "itemclasscategories", tables::extract_item_class_categories)?;
    let base_item_types = load_table(dat_dir, "baseitemtypes", tables::extract_base_item_types)?;
    let mod_families = load_table(dat_dir, "modfamily", tables::extract_mod_families)?;
    let mod_types = load_table(dat_dir, "modtype", tables::extract_mod_types)?;
    let mods = load_table(dat_dir, "mods", tables::extract_mods)?;
    let rarities = load_table(dat_dir, "rarity", tables::extract_rarity)?;

    tracing::info!(
        stats = stats.len(),
        tags = tags.len(),
        item_classes = item_classes.len(),
        item_class_categories = item_class_categories.len(),
        base_items = base_item_types.len(),
        mods = mods.len(),
        rarities = rarities.len(),
        "GameData loaded"
    );

    Ok(GameData::new(
        stats,
        tags,
        item_classes,
        item_class_categories,
        base_item_types,
        mod_families,
        mod_types,
        mods,
        rarities,
    ))
}

/// Read a single datc64 file and extract typed rows.
fn load_table<T>(
    dir: &Path,
    name: &str,
    extract: fn(&DatFile) -> Vec<T>,
) -> Result<Vec<T>, LoadError> {
    let path = dir.join(format!("{name}.datc64"));
    let bytes = std::fs::read(&path).map_err(|e| LoadError::Io {
        table: name.to_string(),
        source: e,
    })?;
    let dat = DatFile::from_bytes(bytes).map_err(|e| LoadError::Parse {
        table: name.to_string(),
        source: e,
    })?;
    Ok(extract(&dat))
}

/// Build a `HashMap<String, usize>` from a slice, keyed by a string field.
fn index_by<T, F: Fn(&T) -> String>(items: &[T], key_fn: F) -> HashMap<String, usize> {
    items
        .iter()
        .enumerate()
        .map(|(i, item)| (key_fn(item), i))
        .collect()
}

// ── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("failed to read {table}.datc64")]
    Io {
        table: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse {table}.datc64")]
    Parse {
        table: String,
        #[source]
        source: poe_dat::dat_reader::DatError,
    },
}
