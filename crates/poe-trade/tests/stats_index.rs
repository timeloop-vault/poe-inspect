//! Integration tests for trade stats index building and cross-referencing.
//!
//! Uses a real trade API response snapshot and real GGPK-extracted game data.

use std::path::PathBuf;
use std::sync::OnceLock;

use poe_data::GameData;
use poe_trade::types::{TradeStatsIndex, TradeStatsResponse};

/// Load full game data (with reverse index). Cached per test run.
fn game_data() -> &'static GameData {
    static GD: OnceLock<GameData> = OnceLock::new();
    GD.get_or_init(|| {
        let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../poe-data/data");
        poe_data::load(&data_dir).expect("game data required — run poe-data extraction first")
    })
}

/// Load the trade stats fixture.
fn trade_stats_fixture() -> TradeStatsResponse {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/trade_stats_3.28.json");
    let file = std::fs::File::open(&path)
        .unwrap_or_else(|e| panic!("fixture not found at {}: {e}", path.display()));
    serde_json::from_reader(std::io::BufReader::new(file)).expect("valid JSON fixture")
}

#[test]
fn fixture_parses_all_categories() {
    let response = trade_stats_fixture();
    assert_eq!(response.result.len(), 13, "expected 13 stat categories");

    let total: usize = response.result.iter().map(|c| c.entries.len()).sum();
    assert!(total > 15_000, "expected 15k+ entries, got {total}");
}

#[test]
fn build_index_from_fixture() {
    let response = trade_stats_fixture();
    let gd = game_data();
    let result = TradeStatsIndex::from_response(&response, gd);

    // Index should contain most entries (some trade IDs are duplicated across categories)
    assert!(
        result.index.len() > 15_000,
        "expected 15k+ unique trade IDs, got {}",
        result.index.len()
    );

    // We should have mapped some GGPK stat IDs
    assert!(
        result.index.mapped_stat_count() > 0,
        "no GGPK stat IDs were mapped"
    );

    println!("=== Trade Stats Index Build Results ===");
    println!("Total entries:    {}", result.total);
    println!("Matched:          {}", result.matched);
    println!("Unmatched:        {}", result.unmatched);
    println!("GGPK stat mapped: {}", result.index.mapped_stat_count());
    println!(
        "Match rate:       {:.1}%",
        result.matched as f64 / (result.matched + result.unmatched) as f64 * 100.0
    );
}

#[test]
fn match_rate_above_threshold() {
    let response = trade_stats_fixture();
    let gd = game_data();
    let result = TradeStatsIndex::from_response(&response, gd);

    let stat_entries = result.matched + result.unmatched;
    let match_rate = result.matched as f64 / stat_entries as f64;

    // Print unmatched templates for debugging
    if !result.unmatched_templates.is_empty() {
        println!(
            "=== {}/{} unmatched templates (first 30) ===",
            result.unmatched_templates.len(),
            stat_entries
        );
        for t in result.unmatched_templates.iter().take(30) {
            println!("  MISS: {t}");
        }
    }

    // We expect at least 85% match rate. Unmatched are mostly atlas passives
    // ("Your Maps have..."), Sanctum/Graft mods, and legacy league-specific mods
    // that don't appear on equipment items.
    assert!(
        match_rate >= 0.85,
        "match rate {:.1}% is below 85% threshold ({}/{} entries)",
        match_rate * 100.0,
        result.matched,
        stat_entries
    );
}

#[test]
fn known_stat_roundtrip() {
    let response = trade_stats_fixture();
    let gd = game_data();
    let result = TradeStatsIndex::from_response(&response, gd);
    let idx = &result.index;

    // "+# to maximum Life" should map to "base_maximum_life"
    let trade_num = idx.trade_stat_number("base_maximum_life");
    assert!(
        trade_num.is_some(),
        "base_maximum_life should have a trade stat number"
    );

    let trade_num = trade_num.unwrap();
    let full_id = idx.full_trade_id("base_maximum_life", "explicit");
    assert!(full_id.is_some());
    println!(
        "base_maximum_life → stat_{trade_num} → {}",
        full_id.unwrap()
    );

    // Reverse: trade stat number → GGPK stat IDs
    let ggpk_ids = idx.ggpk_stat_ids(trade_num);
    assert!(ggpk_ids.is_some());
    assert!(
        ggpk_ids.unwrap().contains(&"base_maximum_life".to_string()),
        "reverse lookup should contain base_maximum_life"
    );
}

#[test]
fn known_stat_template_lookup() {
    let response = trade_stats_fixture();
    let gd = game_data();
    let result = TradeStatsIndex::from_response(&response, gd);
    let idx = &result.index;

    // Look up by template text (case-insensitive)
    let entries = idx.entries_for_template("+# to maximum Life");
    assert!(entries.is_some(), "template lookup should find entries");

    let entries = entries.unwrap();
    // Should have entries across multiple categories (explicit, implicit, fractured, etc.)
    let categories: Vec<&str> = entries.iter().map(|e| e.stat_type.as_str()).collect();
    println!("+# to maximum Life categories: {categories:?}");
    assert!(
        categories.contains(&"explicit"),
        "should include explicit category"
    );
}

#[test]
fn resistance_stats_map() {
    let response = trade_stats_fixture();
    let gd = game_data();
    let result = TradeStatsIndex::from_response(&response, gd);
    let idx = &result.index;

    for stat_id in &[
        "base_fire_damage_resistance_%",
        "base_cold_damage_resistance_%",
        "base_lightning_damage_resistance_%",
    ] {
        let trade_num = idx.trade_stat_number(stat_id);
        assert!(trade_num.is_some(), "{stat_id} should have a trade mapping");
        println!("{stat_id} → stat_{}", trade_num.unwrap());
    }
}

#[test]
fn trade_id_lookup() {
    let response = trade_stats_fixture();
    let gd = game_data();
    let result = TradeStatsIndex::from_response(&response, gd);
    let idx = &result.index;

    // Look up a known trade ID
    let entry = idx.entry_by_trade_id("explicit.stat_3299347043");
    assert!(entry.is_some(), "should find explicit max life entry");
    let entry = entry.unwrap();
    assert!(
        entry.text.contains("maximum Life") || entry.text.contains("maximum life"),
        "entry text should mention maximum life, got: {}",
        entry.text
    );
}

#[test]
fn disk_cache_roundtrip() {
    let response = trade_stats_fixture();
    let temp = std::env::temp_dir().join("poe-trade-test-cache.json");

    // Save
    TradeStatsIndex::save_response(&response, &temp).expect("save should succeed");

    // Load
    let loaded = TradeStatsIndex::load_response(&temp).expect("load should succeed");
    assert_eq!(loaded.result.len(), response.result.len());

    let original_total: usize = response.result.iter().map(|c| c.entries.len()).sum();
    let loaded_total: usize = loaded.result.iter().map(|c| c.entries.len()).sum();
    assert_eq!(loaded_total, original_total);

    // Clean up
    let _ = std::fs::remove_file(&temp);
}
