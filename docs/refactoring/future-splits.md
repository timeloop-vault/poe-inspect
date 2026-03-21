# Future Splits (Watch List)

**Priority:** 5
**Status:** WATCH — not needed yet, trigger when files cross thresholds

These files are approaching the point where splitting helps, but aren't there yet.
Monitor line counts and split when the threshold is crossed.

## poe-item: resolver.rs (1257 lines → split at ~1500)

**Natural split points:**
```
resolver/
  mod.rs               — resolve() orchestrator
  header.rs            — resolve_header(), extract_magic_base_type()
  mods.rs              — resolve_mod(), apply_confirmed_stat_ids()
  sections.rs          — classify_generic_sections(), extract_gem_data()
  pseudos.rs           — compute_pseudo_stats(), compute_dps_pseudos()
  values.rs            — parse_value_ranges(), build_display_text(), parse_socket_info()
```

**Trigger:** Adding flask parsing or Ctrl+C fallback parser will push this over 1500.

## poe-trade: query.rs (1490 lines → split at ~1700)

**Natural split points:**
```
query/
  mod.rs               — build_query() orchestrator
  builder.rs           — resolve_trade_id(), compute_filter_value()
  item_filters.rs      — build_item_filters(), build_type_filters(), build_misc_filters()
  weapon_filters.rs    — build_weapon_filters(), build_socket_filters()
```

**Trigger:** Adding PoE2 trade support or bulk exchange will push this over.

## poe-trade: filter_schema.rs (764 lines → split at ~1000)

Currently manageable. Would split schema parsing from projection logic if it grows.

## poe-data: game_data.rs (993 lines → split at ~1200)

Would extract `suggestions.rs` (stat_suggestions_for_query + helpers, ~130 lines)
if the lookup methods grow significantly.
