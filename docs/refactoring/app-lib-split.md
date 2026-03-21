# Plan: Split app/src-tauri/lib.rs

**Priority:** 1 (highest impact)
**Status:** TODO
**Effort:** ~4 hours
**Risk:** Low — pure mechanical module extraction, no logic changes

## Problem

`lib.rs` is 1966 lines containing all Tauri commands, clipboard logic, hotkey registration,
window management, state initialization, game data loading, and trade state management.
As the most actively developed file, this hurts navigation and code review.

## Target Structure

```
app/src-tauri/src/
  lib.rs              — App setup, plugin registration, state init (~200 lines)
  commands/
    mod.rs            — Re-exports all command modules
    inspect.rs        — inspect_item, get_item_evaluation, get_item_tier_summary
    evaluate.rs       — evaluate_item_for_profiles, get_all_profiles, save_profile, etc.
    trade.rs          — price_check, trade_search_url, refresh_trade_stats, etc.
    hotkey.rs         — register_hotkeys, get_hotkey_config, save_hotkey_config
    settings.rs       — get_settings, save_settings, check_for_update, etc.
    chat_macro.rs     — execute_chat_macro
  game_data.rs        — load_game_data(), GameDataState, init_game_data()
  trade_state.rs      — TradeClient init, TradeStatsState
  clipboard.rs        — Platform-specific clipboard acquisition
  windows.rs          — Window creation helpers, overlay positioning
```

## Steps

1. Create `commands/` directory with mod.rs
2. Move each command group into its own file (inspect, evaluate, trade, hotkey, settings, chat_macro)
3. Extract `game_data.rs` — GameData loading and caching
4. Extract `trade_state.rs` — TradeClient and stats state
5. Extract `clipboard.rs` — platform-specific clipboard code
6. Extract `windows.rs` — window creation helpers
7. Update lib.rs to import and wire modules
8. Run `cargo clippy --manifest-path app/src-tauri/Cargo.toml --tests`
9. Run `cd app && npx tsc --noEmit && npx biome check --write --unsafe .`
10. Test overlay manually: Ctrl+Alt+C an item, verify price check, verify settings window

## Constraints

- All Tauri commands must stay `pub` and registered in `lib.rs` via `.invoke_handler()`
- State types must be accessible from command modules (pass via Tauri managed state)
- Keep `#[tauri::command]` attribute on each function — Tauri requires it at the definition site
- Platform-specific `#[cfg]` blocks move with their functions
