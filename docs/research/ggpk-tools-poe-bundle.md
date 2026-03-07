# GGPK Direct-Read Tools: poe-bundle and poe-query

**Date**: 2026-03-07
**Status**: Research complete
**Purpose**: Analyze two Rust repos (by `ex-nihil`) that read PoE's GGPK/bundle files directly, and assess whether they could replace or supplement RePoE as poe-inspect-2's game data source.

---

## 1. poe-bundle

**Repository**: `github.com/ex-nihil/poe-bundle`
**Author**: `ex-nihil`
**Published**: crates.io as `poe_bundle` v0.1.5
**License**: LGPL-3.0-only
**Rust edition**: 2021

### What It Is

A Rust library for reading PoE's compressed bundle file format. Since patch 3.11.2, GGG stores game data files inside Oodle-compressed bundles. This library handles the decompression pipeline:

1. **GGPK container** (optional) -- uses the `ggpk` crate (v1.2.2) for reading the outer GGPK archive. Can also read from an extracted install directory where `Bundles2/_.index.bin` exists as a standalone file.
2. **Bundle index** -- parses `Bundles2/_.index.bin` to build a file path index. The index contains a list of bundles, a file-hash-to-bundle mapping, and a compressed path table.
3. **Oodle decompression** -- calls `Ooz_Decompress` (a C function from the bundled `ooz` library, an open-source reimplementation of RAD Game Tools' Oodle) via FFI.
4. **File extraction** -- given a virtual file path like `"Data/Mods.dat"`, looks up the hash, finds the containing bundle, decompresses it, and returns the raw bytes at the correct offset/length.

### Architecture

```
poe-bundle/
  build.rs              -- CMake build for the ooz C library
  ooz/                  -- Git submodule (ooz decompression C code)
  src/
    lib.rs              -- Exports: BundledFile, BundleReader, BundleReaderRead
    reader.rs           -- Core implementation (350 lines)
    util.rs             -- FNV1a hashing for file path lookups
    main.rs             -- Minimal demo (extracts GemTags.dat, Tags.dat)
```

### Key Types

```rust
pub struct BundleReader {
    install_path: String,
    pub index: BundleIndex,
    ggpk: Option<GGPK>,  // None when reading from extracted install dir
}

pub trait BundleReaderRead {
    fn size_of(&self, file: &str) -> Option<usize>;
    fn write_into(&self, file: &str, dst: &mut impl Write) -> Result<usize, Error>;
    fn bytes(&self, file: &str) -> Result<Vec<u8>, Error>;
}

pub struct BundledFile {
    pub bundle_path: String,
    pub bundle_uncompressed_size: u32,
    pub offset: u32,
    pub size: u32,
}
```

Usage: `BundleReader::from_install(Path::new("/path/to/poe/"))` reads the index and builds the file lookup table. Then `reader.bytes("Data/Mods.dat")` returns raw `.dat` file bytes.

### File Path Hashing

File lookups use FNV-1a hash of the lowercased path with `++` suffix appended:

```rust
pub fn filepath_hash(data: String) -> u64 {
    let lowercase_salted = format!("{}++", data.to_lowercase());
    hash_fnv1a(lowercase_salted.as_bytes())
}
```

### Build Dependencies

- **cmake** (build-time): Compiles the ooz C library
- **ggpk** crate v1.2.2: For reading the outer GGPK container
- **byteorder**: Little-endian binary parsing
- **libc**: FFI to the C decompression function

### Maintenance Status

- Last git activity: The repo has files dated March 7 (today), likely due to a fresh clone. The code references PoE 3.11.2+ bundle format. The `ggpk` crate v1.2.2 and other dependencies are from the 2020-2022 era.
- The `ooz` submodule directory exists but appears empty (no files found by glob), suggesting the submodule was not initialized in this clone.
- **Critical concern**: The ooz decompression library may need updates for newer Oodle versions used in PoE2/3.25+. GGG has updated their Oodle compression across patches.

---

## 2. poe-query

**Repository**: `github.com/ex-nihil/poe-query`
**Author**: `ex-nihil`
**Rust edition**: 2021
**Last file timestamps**: November 12, 2022

### What It Is

A CLI tool that uses `poe_bundle` to extract and query `.dat` files from a PoE installation. It implements:

1. **A `.dat` file parser** -- reads the fixed-row binary format of PoE's data tables
2. **A schema system** -- uses the community `poe-tool-dev/dat-schema` GraphQL format to define table structures
3. **A query language (PQL)** -- a jq-like DSL for traversing, filtering, and reshaping the extracted data into JSON

### Architecture

```
poe-query/
  dat-schema/           -- Community .gql schema files (from poe-tool-dev/dat-schema)
  examples/
    mods.pql            -- Example query extracting mod data
  src/
    main.rs             -- CLI entry point (clap-based)
    dat/
      mod.rs            -- Module declarations
      file.rs           -- DatFile: binary .dat parser (165 lines)
      reader.rs         -- DatContainer: ties bundles + specs together (67 lines)
      specification.rs  -- FileSpec/FieldSpec: parses .gql schema into specs (230 lines)
      util.rs           -- Byte pattern search utility
    query/
      mod.rs            -- PQL parser (pest-based, 320 lines)
      grammar.pest      -- PEG grammar for the query language
    traversal/
      mod.rs
      traverse.rs       -- Query execution engine (609 lines)
      value.rs          -- Value enum for the query system (113 lines)
  update_schemas.sh     -- Downloads latest schemas from poe-tool-dev/dat-schema
  extract.sh            -- Builds and runs a mods extraction query
```

### DAT File Format

The `.dat` binary format (as implemented in `file.rs`):

```
[4 bytes] row_count (u32 LE)
[row_count * row_size bytes] fixed-size rows
[0xBB 0xBB 0xBB 0xBB 0xBB 0xBB 0xBB 0xBB] data section marker
[variable-length data section] strings (UTF-16LE, null-terminated), lists, refs
```

- Row size is computed: `(data_section_offset - 4) / row_count`
- The marker `0xBBBBBBBBBBBBBBBB` (8 bytes of 0xBB) separates the fixed-row section from the variable-length data section.
- Null/empty values are signaled by `0xFEFEFEFE` (u32) or `0xFEFEFEFEFEFEFEFE` (u64).
- Strings in the data section are UTF-16LE, null-terminated.
- Lists are stored as `(length: u32, offset: u32)` in the row, with the actual data at the given offset in the data section.
- Foreign key references are row indices (u32) into other `.dat` files.

### Schema System (dat-schema)

The schemas come from the community project `poe-tool-dev/dat-schema` and are written in a GraphQL-like format. The `update_schemas.sh` script downloads the latest:

```sh
curl -Ls https://github.com/poe-tool-dev/dat-schema/archive/refs/heads/main.zip --output dat_schema.zip
unzip -jo dat_schema.zip dat-schema-main/dat-schema/* -d dat-schema
```

**Schema structure**: Each `.gql` file defines types (tables) and enums. Files are versioned by league/expansion (e.g., `3_19_Lake_of_Kalandra.gql`). The `_Core.gql` file contains the base definitions for all tables.

**Example -- Mods table schema:**

```graphql
type Mods {
  Id: string @unique
  HASH16: i32
  ModTypeKey: ModType
  Level: i32
  StatsKey1: Stats
  StatsKey2: Stats
  StatsKey3: Stats
  StatsKey4: Stats
  Domain: ModDomains
  Name: string
  GenerationType: ModGenerationType
  Families: [ModFamily]
  Stat1Min: i32
  Stat1Max: i32
  ...
  SpawnWeight_TagsKeys: [Tags]
  SpawnWeight_Values: [i32]
  TagsKeys: [Tags]
  GenerationWeight_TagsKeys: [Tags]
  GenerationWeight_Values: [i32]
  IsEssenceOnlyModifier: bool
  ...
  InfluenceTypes: InfluenceTypes
  ImplicitTagsKeys: [Tags]
  ...
}
```

**How poe-query parses schemas:**

The `specification.rs` file uses `apollo-parser` (a GraphQL parser) to read `.gql` files. For each `ObjectTypeDefinition`, it:
1. Extracts field names and types
2. Computes byte offsets for each field based on type sizes:
   - `bool`, `u8` = 1 byte
   - `u32`, `i32`, `rid`, `string` (ref) = 4 bytes
   - `u64`, foreign keys, `list` = 8 bytes (lists store length + offset)
   - Self-references = 4 bytes
3. Converts GraphQL types to internal type tags: `"ref|string"`, `"list|u32"`, `"list|i32"`, etc.
4. Resolves enum types so enum fields return human-readable string values instead of raw integers.

**Supported primitive types:**
- `bool` (1 byte)
- `u8` (1 byte)
- `u32` (4 bytes, unsigned)
- `i32` (4 bytes, signed)
- `u64` / `ptr` (8 bytes)
- `string` (4-byte offset into data section, UTF-16LE null-terminated)
- `list|<type>` (8 bytes: 4-byte length + 4-byte offset, contents in data section)
- Foreign keys: type name resolves to `u32` (4-byte row index into the referenced table)

### Query Language (PQL)

PQL is a jq-inspired DSL built with the `pest` PEG parser. It supports:

- **Field access**: `.Mods.Id`, `.StatsKey1.Id`
- **Indexing**: `[0]`, `[-1]`
- **Slicing**: `[0:10]`
- **Iteration**: `[]` (like jq's `.[]`)
- **Filtering**: `select(.Domain == "ITEM")`
- **Object construction**: `{ "key": .Value }`
- **Array construction**: `[.Field1, .Field2]`
- **Variables**: `as $var`, `$var`
- **Map/Reduce**: `map(...)`, `reduce ... as $var (init; body)`
- **Transpose**: `transpose` (zip lists together)
- **Arithmetic**: `+`, `-`, `*`, `/`
- **Pipe**: `|`

**Example query (from `examples/mods.pql`):**

```
.Mods[0] |
{
    "id": .Id,
    "domain": .Domain,
    "type": .GenerationType,
    "name": .Name,
    "ilvl": .Level,
    "mod_type": .ModTypeKey.Name,
    "stats": {
        .StatsKey1.Id: { "min": .Stat1Min, "max": .Stat1Max },
        .StatsKey2.Id: { "min": .Stat2Min, "max": .Stat2Max },
        ...
    },
    "spawn_weights": [.SpawnWeight_TagsKeys[].Id, .SpawnWeight_Values] | zip_to_obj,
    "gen_weights": [.GenerationWeight_TagsKeys[].Id, .GenerationWeight_Values] | zip_to_obj
}
```

This extracts the first mod row with its ID, domain, generation type, human-readable name, level requirement, mod type name, stats with ranges, and spawn/generation weights as tag-value maps.

### Foreign Key Resolution

When a field is a foreign key (its type references another table, e.g., `ModTypeKey: ModType`), PQL automatically resolves it. Accessing `.ModTypeKey.Name` will:
1. Read the u32 row index from the current row's `ModTypeKey` field
2. Load `Data/ModType.dat` (if not cached)
3. Read the referenced row
4. Extract the `Name` field from that row

This is equivalent to a SQL JOIN -- the raw data stores integer indices, and poe-query resolves them to full objects on access.

---

## 3. Raw GGPK Data vs. RePoE: Detailed Comparison

### 3a. Mods Table

**Raw `Data/Mods.dat` (via dat-schema):**

| Field | Raw Type | Description |
|-------|----------|-------------|
| `Id` | string | Internal mod ID, e.g., `"IncreasedLife1"` |
| `HASH16` | i32 | 16-bit hash |
| `ModTypeKey` | FK -> ModType | Reference to mod type (prefix group name) |
| `Level` | i32 | Required item level |
| `StatsKey1`-`StatsKey6` | FK -> Stats | Up to 6 stat references (row indices) |
| `Domain` | enum ModDomains | `ITEM`, `FLASK`, `CRAFTED`, `MONSTER`, etc. |
| `Name` | string | **The display name shown in Ctrl+Alt+C headers** (e.g., `"Hale"`, `"of the Leech"`) |
| `GenerationType` | enum ModGenerationType | `PREFIX`, `SUFFIX`, `UNIQUE`, `ENCHANTMENT`, etc. |
| `Families` | list[FK -> ModFamily] | Mod group(s) for mutual exclusion |
| `Stat1Min`-`Stat6Max` | i32 | Min/max ranges for each stat |
| `SpawnWeight_TagsKeys` | list[FK -> Tags] | Tags that affect spawn weight |
| `SpawnWeight_Values` | list[i32] | Corresponding spawn weight values |
| `TagsKeys` | list[FK -> Tags] | Tags on the mod itself |
| `GenerationWeight_TagsKeys` | list[FK -> Tags] | Tags for generation weight |
| `GenerationWeight_Values` | list[i32] | Generation weight values |
| `IsEssenceOnlyModifier` | bool | Essence-only flag |
| `InfluenceTypes` | enum | `SHAPER`, `ELDER`, `CRUSADER`, etc. |
| `ImplicitTagsKeys` | list[FK -> Tags] | Implicit tags (for harvest craft targeting) |
| `MaxLevel` | i32 | Maximum level for the mod |
| `CraftingItemClassRestrictions` | list[FK -> ItemClasses] | Item class restrictions |
| `ArchnemesisMinionMod` | FK -> Mods | Self-referencing (archnemesis) |
| `HASH32` | i32 | 32-bit hash |
| `BuffTemplate` | FK -> BuffTemplates | Associated buff |

**RePoE `mods.json` (per our prior research):**

RePoE restructures this data significantly:
- `Id` -> key of the JSON object
- `Name` -> `name` (the Ctrl+Alt+C display name)
- `Domain` -> `domain` (integer or string)
- `GenerationType` -> `generation_type` (string)
- Stats are flattened into a `stats` array of `{id, min, max}` objects
- `spawn_weights` and `generation_weights` are arrays of `{tag, weight}` objects
- `required_level` corresponds to `Level`
- `adds_tags` corresponds to `ImplicitTagsKeys`
- `is_essence_only` corresponds to `IsEssenceOnlyModifier`

**Key finding -- the `Name` field:**

The `Name` field in raw `Mods.dat` is **exactly what appears in the Ctrl+Alt+C header** inside quotes (e.g., `Prefix Modifier "Hale" (Tier: 3)`). This field is preserved as `name` in RePoE's `mods.json`. So for this critical mapping, RePoE does NOT lose information.

However, note that `ModTypeKey` (FK -> `ModType.Name`) is the **mod group name** (like `"IncreasedLife"` or `"PhysicalDamage"`), while `Families` are the **mod family/group IDs** used for mutual exclusion. RePoE maps `ModTypeKey` to `type` and `Families` to `group`. The naming is different but the data is equivalent.

### 3b. BaseItemTypes Table

**Raw `Data/BaseItemTypes.dat`:**

| Field | Raw Type | Description |
|-------|----------|-------------|
| `Id` | string | Internal ID, e.g., `"Metadata/Items/Armours/Helmets/HelmetInt1"` |
| `ItemClassesKey` | FK -> ItemClasses | Item class (e.g., Helmet, Body Armour) |
| `Width` / `Height` | i32 | Inventory dimensions |
| `Name` | string @localized | **Display name** (e.g., `"Hubris Circlet"`) |
| `InheritsFrom` | string | Metadata inheritance path |
| `DropLevel` | i32 | Minimum drop level |
| `Implicit_ModsKeys` | list[FK -> Mods] | Implicit mod references |
| `TagsKeys` | list[FK -> Tags] | Item tags (for mod pool filtering) |
| `ModDomain` | enum ModDomains | Which mod domain applies |
| `ItemVisualIdentity` | FK | Visual identity reference |
| `HASH32` | i32 | 32-bit hash |
| `IsCorrupted` | bool | Whether base is inherently corrupted |
| `TradeMarketCategory` | FK | Trade site category |

**Comparison with RePoE `base_items.json`:**

RePoE restructures base items with:
- `Id` as the JSON key (the metadata path)
- `name` = the display `Name` (localized)
- `item_class` = resolved `ItemClassesKey` name
- `drop_level` = `DropLevel`
- `tags` = resolved `TagsKeys` (as string IDs)
- `implicits` = resolved `Implicit_ModsKeys` (as mod ID strings)
- `domain` = resolved `ModDomain`

The display `Name` is directly available in both. RePoE's `base_items.json` provides it as-is. The mapping from clipboard `base_type` text to the game data `Name` field should work identically whether reading from RePoE or raw `.dat` files.

### 3c. Stats and Stat Translations

**Raw `Data/Stats.dat`:**

| Field | Raw Type | Description |
|-------|----------|-------------|
| `Id` | string | Internal stat ID (e.g., `"base_maximum_life"`) |
| `IsLocal` | bool | Whether the stat is local to the item |
| `IsWeaponLocal` | bool | Whether it's weapon-local |
| `Semantics` | enum StatSemantics | `PERCENT`, `VALUE`, `FLAG`, `PERMYRIAD`, etc. |
| `Text` | string | A text field (possibly an internal description) |
| `IsVirtual` | bool | Virtual stats (computed, not displayed) |
| `MainHandAlias_StatsKey` | FK -> Stats | MH alias |
| `OffHandAlias_StatsKey` | FK -> Stats | OH alias |
| `HASH32` | i32 | Hash |
| `Category` | FK -> PassiveSkillStatCategories | Stat category |

**Stat translations** -- this is the critical gap:

The raw `.dat` files do NOT contain stat translation templates directly. `Stats.dat` has a `Text` field, but the actual display text templates (e.g., `"{0}% increased maximum Life"`) come from **separate text files**, not `.dat` tables. These are the `stat_descriptions.txt` files located at paths like `Metadata/StatDescriptions/stat_descriptions.txt`.

The `StatDescriptionFunctions.dat` table maps stat IDs to translation function IDs, but the actual template strings are in separate non-`.dat` text files that follow their own format (the same files that RePoE's `stat_translations.json` is derived from).

**What RePoE does for stat translations:**

RePoE parses the raw `stat_descriptions.txt` files and converts them to `stat_translations.json`, which is a structured JSON format with:
- Language/locale support
- Stat ID groups (multiple stats per translation entry)
- Condition-based text selection (different text based on value ranges)
- Format strings with placeholders

**Key finding**: poe-bundle/poe-query do NOT handle stat translation text files. They only read `.dat` binary tables. The stat description files are a different format entirely. RePoE is the only readily-available tool that parses these into structured JSON.

### 3d. Tags Table

**Raw `Data/Tags.dat`:**

```graphql
type Tags {
  Id: string @unique      -- e.g., "axe", "helmet", "default"
  _: i32                  -- unknown field
  DisplayString: string   -- human-readable display string
  Name: string            -- tag name
}
```

RePoE's `tags.json` provides these as a simple list. No data loss.

### 3e. Enums in Raw Data vs. RePoE

Raw `.dat` files store enum values as integers. The dat-schema defines the mapping:

```graphql
enum ModDomains @indexing(first: 1) {
  ITEM          -- 1
  FLASK         -- 2
  MONSTER       -- 3
  ...
  CRAFTED       -- 9
  ...
}

enum ModGenerationType @indexing(first: 1) {
  PREFIX        -- 1
  SUFFIX        -- 2
  UNIQUE        -- 3
  ...
  ENCHANTMENT   -- 9
  ...
}
```

RePoE converts these to string labels. The `@indexing(first: N)` directive tells poe-query which integer maps to the first enum variant. Unnamed variants (`_`) represent unknown/unused values.

---

## 4. What Raw Data Has That RePoE Might Not

### 4a. Fields Present in Raw Data

Examining the Mods schema, several fields exist in raw data that may not appear in RePoE:

1. **`HASH16` / `HASH32`**: Hash values used for efficient lookups. Not useful for our purposes.
2. **`MonsterMetadata`**: Monster-related metadata string. Only relevant for monster mods.
3. **`MonsterKillAchievements`**: Achievement references. Not relevant.
4. **`ChestModType`**: Chest-specific mod types. Not relevant for item evaluation.
5. **`MonsterOnDeath`**: Monster death effects. Not relevant.
6. **`Heist_*` fields**: Heist-specific stat overrides. Niche but potentially relevant for heist items.
7. **`BuffTemplate`**: Associated buff template. Could be useful for understanding some mod effects.
8. **`ArchnemesisMinionMod`**: Archnemesis self-reference. Not relevant for items.
9. **`MaxLevel`**: Maximum item level for the mod. **This is potentially useful** -- it defines the upper bound of an ilvl range where this mod can roll. RePoE may include this as `max_level` but it should be verified.
10. **Several unnamed fields (`_: i32`, `_: bool`)**: Unknown/undocumented fields that GGG uses internally. These could contain data relevant to newer mechanics.

### 4b. Information Equivalence

For the fields critical to poe-inspect-2's clipboard parsing:

| Need | Raw .dat field | RePoE field | Same? |
|------|---------------|-------------|-------|
| Mod display name (Ctrl+Alt+C) | `Mods.Name` | `mods[id].name` | YES |
| Mod internal ID | `Mods.Id` | `mods` key | YES |
| Prefix/Suffix/etc. | `Mods.GenerationType` (enum) | `mods[id].generation_type` | YES |
| Mod domain | `Mods.Domain` (enum) | `mods[id].domain` | YES |
| Mod group (mutual exclusion) | `Mods.Families` -> `ModFamily.Id` | `mods[id].group` | YES |
| Required level | `Mods.Level` | `mods[id].required_level` | YES |
| Stats + ranges | `Mods.StatsKey1-6` + `Stat1Min-6Max` | `mods[id].stats[]` | YES |
| Spawn weights | `SpawnWeight_TagsKeys` + `Values` | `mods[id].spawn_weights[]` | YES |
| Generation weights | `GenerationWeight_TagsKeys` + `Values` | `mods[id].generation_weights[]` | YES |
| Tags | `Mods.TagsKeys` -> `Tags.Id` | `mods[id].adds_tags` | YES |
| Implicit tags | `Mods.ImplicitTagsKeys` | `mods[id].implicit_tags` | VERIFY |
| Essence-only flag | `Mods.IsEssenceOnlyModifier` | `mods[id].is_essence_only` | YES |
| Influence type | `Mods.InfluenceTypes` (enum) | Not standard in RePoE | CHECK |
| Base item name | `BaseItemTypes.Name` | `base_items[id].name` | YES |
| Item class | `BaseItemTypes.ItemClassesKey.Name` | `base_items[id].item_class` | YES |
| Base item tags | `BaseItemTypes.TagsKeys` | `base_items[id].tags` | YES |
| Stat translation text | Not in .dat files | `stat_translations.json` | RePoE ONLY |

**Bottom line**: RePoE preserves all the data fields that matter for clipboard parsing. It does not lose critical information in its transformation. The field renaming (e.g., `Level` -> `required_level`, `Families` -> `group`) is documented and consistent.

---

## 5. The dat-schema Format

### Source

The schemas in poe-query's `dat-schema/` directory are downloaded from `github.com/poe-tool-dev/dat-schema`. This is the same community-maintained schema project that PyPoE and RePoE themselves rely on.

### Format Details

Each schema file is a `.gql` (GraphQL-like) file defining:

- **`type TableName { ... }`**: Defines a .dat table with named, typed, ordered fields
- **`enum EnumName @indexing(first: N) { VARIANT1, VARIANT2, ... }`**: Defines integer-to-string mappings
- **Unnamed fields**: `_: i32` marks unknown/undocumented columns (preserving byte alignment)
- **Directives**: `@unique`, `@localized`, `@file(ext: ".dds")`, `@ref(column: "Id")` provide metadata

### Versioning

Schema files are versioned by expansion:
- `_Core.gql` -- base definitions (4,800+ lines, covers all core tables)
- `3_19_Lake_of_Kalandra.gql` -- adds/modifies tables for that expansion
- Earlier versions go back to `0_11_Anarchy.gql`

The most recent schema file in this repo is `3_19_Lake_of_Kalandra.gql` (August 2022). **The schemas in this repo are frozen at patch 3.19.** Current PoE is 3.28+, and the upstream `poe-tool-dev/dat-schema` has been updated for newer patches. Running `update_schemas.sh` would pull the latest.

### As a Data Definition Layer

Could dat-schema replace or supplement RePoE as our data definition layer?

**Pros:**
- It is the authoritative source -- RePoE itself is derived from these definitions
- It preserves exact binary layout information (byte offsets, field sizes)
- It includes unnamed fields, showing where unknown data lives
- Enum definitions map integers to human-readable values
- Foreign key relationships are explicit in the type system

**Cons:**
- It defines table structures only, not the data itself -- you still need the game files or a tool to extract the data
- It does not cover stat translation files (the `.txt` description files)
- The GraphQL-like format would require a custom parser (or borrowing apollo-parser like poe-query does)
- Keeping schemas current requires tracking the upstream project

---

## 6. Viability Assessment

### Could poe-inspect-2 use poe-bundle as a dependency?

**Technically possible, but not recommended for our use case.** Here is why:

1. **Build complexity**: poe-bundle requires a C/C++ toolchain and CMake to compile the ooz decompression library. This adds significant build complexity to a Tauri (TypeScript + Rust) project.

2. **Platform concerns**: The ooz FFI layer with its C bindings needs careful cross-platform testing. The build.rs uses CMake which can be fragile across platforms.

3. **Runtime requirement**: Using poe-bundle means reading from the user's PoE installation directory at runtime. This requires:
   - Knowing the install path (auto-detection or user config)
   - Having read access to the game files
   - The game files being up-to-date
   - Handling the case where files are locked by the running game

4. **Maintenance burden**: The library was last meaningfully updated around 2022. Newer PoE patches may have changed the bundle format, Oodle compression parameters, or .dat file structures. The `ggpk` crate dependency (v1.2.2) may also be outdated.

5. **PoE2 compatibility**: PoE2 may use different compression or file organization. The library was built for PoE1.

6. **We only need the data, not the reader**: poe-inspect-2 needs structured game data (mod definitions, stat translations, base items). RePoE already provides this as pre-extracted JSON. Reading GGPK directly would give us the same data but with much more complexity.

### What about using dat-schema as supplementary type definitions?

**Moderately useful.** The dat-schema could serve as documentation for understanding what fields exist in the raw data and how they map to RePoE's JSON output. However, since RePoE already transforms the data for us, the schema adds a layer we don't need.

If we ever found a field missing from RePoE that we needed, the dat-schema would tell us exactly where it lives in the raw data. But as shown in Section 4, RePoE preserves all fields relevant to our clipboard parsing needs.

### Tradeoffs Summary

| Approach | Data freshness | Build complexity | Stat translations | Offline use | Maintenance |
|----------|---------------|-----------------|-------------------|-------------|-------------|
| **RePoE JSON** (current plan) | Updated per-patch by community | None (static JSON) | YES (parsed) | YES (bundle JSON) | Low (just update URLs) |
| **poe-bundle direct read** | Always current (reads game files) | HIGH (C++, CMake, FFI) | NO (not in .dat files) | NO (needs game installed) | HIGH (track format changes) |
| **Hybrid (RePoE + dat-schema types)** | Per-patch | Low | YES (from RePoE) | YES | Medium |

### Recommendation

**Stick with RePoE as the data source.** The analysis confirms that RePoE does not lose any information critical to clipboard parsing. The `Name` field (mod display names for Ctrl+Alt+C), stat ranges, spawn weights, tags, and all other fields needed for the clipboard-to-game-data pipeline are preserved in RePoE's transformation.

The one area where raw data access would theoretically help -- if a brand-new field were added to a .dat table that RePoE hadn't exported yet -- is a rare edge case that can be handled by filing an issue with the RePoE maintainers.

**Stat translations are the clincher.** The raw .dat files do NOT contain stat translation templates. These are in separate `.txt` files that poe-bundle/poe-query do not parse. RePoE's `stat_translations.json` is the only readily-available structured source for this data. Since stat translations are essential for mapping clipboard display text to internal stat IDs, we cannot avoid RePoE anyway.

---

## 7. Key Takeaways for poe-inspect-2

1. **The mod `Name` field** (e.g., `"Hale"`, `"of the Leech"`) that appears in Ctrl+Alt+C headers is a direct column in `Mods.dat`. RePoE preserves it as `mods[id].name`. No transformation ambiguity.

2. **`ModType.Name`** (e.g., `"IncreasedLife"`) is the mod group name, accessed via `ModTypeKey` foreign key. RePoE exports this as `type`. It is NOT the same as the display name.

3. **`ModFamily.Id`** defines mutual exclusion groups. RePoE exports this as `group`. Mods in the same family cannot coexist on an item.

4. **Spawn weights are tag-indexed**: Each mod has parallel arrays of `SpawnWeight_TagsKeys` and `SpawnWeight_Values`. A weight of 0 for a given tag means the mod cannot spawn on items with that tag. RePoE preserves this structure as `spawn_weights: [{tag, weight}, ...]`.

5. **Stat translations live outside .dat files** -- they are in `Metadata/StatDescriptions/*.txt` files with their own format. RePoE's `stat_translations.json` is the only convenient structured source.

6. **The dat-schema project** (`poe-tool-dev/dat-schema`) is the authoritative source for understanding raw game data structure. Both RePoE and poe-query derive from it. If we ever need to understand what a RePoE field maps to in the raw data, the `.gql` schemas are the reference.

7. **poe-query's mods.pql example** demonstrates exactly the data shape we need -- it extracts mod ID, name, domain, generation type, stats with ranges, and spawn/generation weights. This confirms RePoE's `mods.json` contains equivalent information.

8. **The `@localized` annotation** on `BaseItemTypes.Name` indicates this field is language-dependent. Our parser should be aware that base item names could differ by locale, though for English this is not an issue.
