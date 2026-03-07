# poe-dat

Lowest-level crate in the workspace. Reads raw PoE game data files — no game-domain logic.

## Scope

- Parse `.dat` / `.dat64` binary table files using community `dat-schema` definitions
- Parse stat description files (`Metadata/StatDescriptions/*.txt`) — UTF-16LE text, line-oriented, tab-indented
- Expose raw typed rows, not game-domain types (that's poe-data's job)
- No network access, no caching — this crate only reads bytes in, structs out

## Does NOT own

- Game-domain types (mods, base items, etc.) — that's `poe-data`
- Bundle extraction / GGPK reading — that's `poe-bundle` (submodule)
- Any interpretation of what the data *means*

## Key Design Decisions

- **Own the parsing**: We parse .dat files ourselves using dat-schema, rather than depending on RePoE JSON. This avoids 1000+ lines of reshaping code that v1 needed to invert lookup directions, filter rollable mods, and re-index base items.
- **dat-schema as source of truth**: Column names, types, and offsets come from `poe-tool-dev/dat-schema` (GraphQL SDL format). poe-query's bundled copy under `crates/poe-query/dat-schema/` can be used as reference.
- **Stat description parser**: The `Metadata/StatDescriptions/*.txt` files are a stable format (10+ years). Three constructs: `no_description` blocks, `description` blocks with condition→template lines, and `include` directives. Index handlers transform values (negate, divide_by_one_hundred, etc.). Prior art: a ~260 line JS parser exists from an earlier project.

## .dat Binary Format (Quick Reference)

```
[4 bytes: row_count (u32le)]
[row_count × row_size bytes: fixed-width rows]
[magic: 0xBBBBBBBBBBBBBBBB (8 bytes)]
[variable-length data pool: strings (UTF-16LE, null-terminated), lists]
```

- Strings in rows are `[offset: u32]` pointing into the variable pool
- Lists are `[count: u32, offset: u32]` — offset points to `count` consecutive values in the pool
- Foreign keys are row indices (u32) into other tables, `0xFEFEFEFE` = null

## Stat Description Format (Quick Reference)

```
description <stat_id_1> <stat_id_2> ...
    <n_stats>
        <condition_1> <condition_2> ... "<display_text>" [reminder_text]
        ...
```

- Conditions: `#` (any), `1` (exact), `!0` (not zero), `2|5` (range 2–5)
- Placeholders: `%0%` (raw value), `%0$+d` (with sign), `%%` (literal %)
- Handlers: `negate 1`, `per_minute_to_per_second 2`, `divide_by_one_hundred 1`

## Build Order

This is the first crate to build. No intra-workspace dependencies.

## Plan

1. Schema parser: read dat-schema `.graphql` files → column definitions
2. .dat reader: given schema + bytes → Vec<Row> with typed column access
3. Stat description parser: `.txt` file → lookup structure (stat_ids + values → display string)
4. Tests against real extracted data files
