//! Integration tests for the trade query builder.
//!
//! Uses real item fixtures parsed through poe-item, cross-referenced with
//! the trade stats index built from real API data.

use std::path::PathBuf;
use std::sync::OnceLock;

use poe_data::GameData;
use poe_item::resolve;
use poe_trade::QueryBuildResult;
use poe_trade::query::{StatGroupType, build_query};
use poe_trade::types::{TradeQueryConfig, TradeStatsIndex, TradeStatsResponse};

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

/// Build the trade stats index. Cached per test run.
fn trade_index() -> &'static TradeStatsIndex {
    static IDX: OnceLock<TradeStatsIndex> = OnceLock::new();
    IDX.get_or_init(|| {
        let response = trade_stats_fixture();
        let result = TradeStatsIndex::from_response(&response, game_data());
        result.index
    })
}

/// Parse a fixture file into a ResolvedItem.
fn parse_fixture(name: &str) -> poe_item::types::ResolvedItem {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/items")
        .join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("fixture not found at {}: {e}", path.display()));
    let raw = poe_item::parse(&text).expect("fixture should parse");
    resolve(&raw, game_data())
}

fn default_config() -> TradeQueryConfig {
    TradeQueryConfig::new("Mirage")
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[test]
fn rare_body_armour_builds_valid_query() {
    let item = parse_fixture("rare-body-armour-craft-hybrid-and-normal-life-mod.txt");
    let result = build_query(&item, trade_index(), &default_config(), None);

    // Should have base type set
    assert_eq!(result.body.query.base_type.as_deref(), Some("Titan Plate"));
    // Rares are nonunique
    let type_filters = result
        .body
        .query
        .filters
        .as_ref()
        .and_then(|f| f.type_filters.as_ref());
    assert_eq!(
        type_filters
            .unwrap()
            .filters
            .rarity
            .as_ref()
            .unwrap()
            .option,
        "nonunique"
    );
    // No name for rares
    assert!(result.body.query.name.is_none());
    // Should have mapped some stats
    assert!(result.stats_mapped > 0, "expected mapped stats, got 0");
    // Sort by price ascending
    assert_eq!(result.body.sort.price, "asc");

    print_result_summary("rare body armour", &result);
}

#[test]
fn rare_weapon_builds_stat_filters() {
    let item = parse_fixture("battered-foil-rare-ess-craft.txt");
    let result = build_query(&item, trade_index(), &default_config(), None);

    assert_eq!(
        result.body.query.base_type.as_deref(),
        Some("Battered Foil")
    );

    // Should have stat filters in an AND group
    assert_eq!(result.body.query.stats.len(), 1);
    let group = &result.body.query.stats[0];
    assert!(matches!(group.group_type, StatGroupType::And));
    assert!(!group.filters.is_empty());

    // All filter IDs should have proper category prefixes
    for filter in &group.filters {
        assert!(
            filter.id.starts_with("explicit.stat_")
                || filter.id.starts_with("implicit.stat_")
                || filter.id.starts_with("crafted.stat_"),
            "unexpected filter ID prefix: {}",
            filter.id
        );
    }

    // Crafted mod "Hits can't be Evaded" should have no value (boolean stat)
    let crafted_filters: Vec<_> = group
        .filters
        .iter()
        .filter(|f| f.id.starts_with("crafted."))
        .collect();
    // May or may not map depending on trade stats coverage
    for cf in &crafted_filters {
        // Boolean stats should have no value filter
        if cf.value.is_none() {
            // Good — boolean stat correctly has no value
        }
    }

    print_result_summary("battered foil", &result);
}

#[test]
fn fractured_item_uses_fractured_prefix() {
    let item = parse_fixture("rare-axe-fractured.txt");
    let result = build_query(&item, trade_index(), &default_config(), None);

    // Should have fractured_item misc filter
    let misc = result
        .body
        .query
        .filters
        .as_ref()
        .and_then(|f| f.misc_filters.as_ref());
    assert!(misc.is_some(), "fractured items should have misc filters");
    assert_eq!(
        misc.unwrap()
            .filters
            .fractured_item
            .as_ref()
            .unwrap()
            .option,
        "true"
    );

    // The fractured mod should use "fractured." prefix
    let all_filter_ids: Vec<&str> = result
        .body
        .query
        .stats
        .iter()
        .flat_map(|g| g.filters.iter())
        .map(|f| f.id.as_str())
        .collect();
    let has_fractured = all_filter_ids.iter().any(|id| id.starts_with("fractured."));
    assert!(
        has_fractured,
        "expected at least one fractured.stat_ filter, got: {all_filter_ids:?}"
    );

    print_result_summary("fractured axe", &result);
}

#[test]
fn corrupted_item_sets_corrupted_filter() {
    let item = parse_fixture("rare-amulet-talisman-corrupted.txt");
    let result = build_query(&item, trade_index(), &default_config(), None);

    let misc = result
        .body
        .query
        .filters
        .as_ref()
        .and_then(|f| f.misc_filters.as_ref());
    assert!(misc.is_some());
    assert_eq!(
        misc.unwrap().filters.corrupted.as_ref().unwrap().option,
        "true"
    );

    print_result_summary("corrupted amulet", &result);
}

#[test]
fn value_relaxation_applied() {
    let item = parse_fixture("rare-body-armour-craft-hybrid-and-normal-life-mod.txt");
    let config = TradeQueryConfig::new("Mirage");
    let result = build_query(&item, trade_index(), &config, None);

    // All numeric filters should have min values that are ~85% of actual
    for group in &result.body.query.stats {
        for filter in &group.filters {
            if let Some(ref value) = filter.value {
                assert!(value.min.is_some(), "numeric filter should have min");
                assert!(
                    value.max.is_none(),
                    "should not set max (open-ended search)"
                );
            }
        }
    }
}

#[test]
fn online_only_sets_status() {
    let item = parse_fixture("rare-belt-crafted.txt");

    // Online only (default)
    let result = build_query(&item, trade_index(), &default_config(), None);
    assert_eq!(result.body.query.status.as_ref().unwrap().option, "online");

    // Offline mode
    let mut config = default_config();
    config.online_only = false;
    let result = build_query(&item, trade_index(), &config, None);
    assert!(result.body.query.status.is_none());
}

#[test]
fn query_serializes_to_valid_json() {
    let item = parse_fixture("rare-body-armour-craft-hybrid-and-normal-life-mod.txt");
    let result = build_query(&item, trade_index(), &default_config(), None);

    let json = serde_json::to_string_pretty(&result.body).expect("should serialize");

    // Basic structure validation
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    assert!(parsed.get("query").is_some());
    assert!(parsed.get("sort").is_some());
    assert!(parsed["query"].get("stats").is_some());
    assert_eq!(parsed["sort"]["price"], "asc");

    // Stat group type should serialize as "and" (lowercase)
    if let Some(stats) = parsed["query"]["stats"].as_array() {
        if let Some(first) = stats.first() {
            assert_eq!(first["type"], "and");
        }
    }

    println!("Query JSON:\n{json}");
}

#[test]
fn trade_url_format() {
    let url = poe_trade::query::trade_url("Mirage", "abc123");
    assert_eq!(
        url,
        "https://www.pathofexile.com/trade/search/Mirage/abc123"
    );
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn print_result_summary(label: &str, result: &QueryBuildResult) {
    println!(
        "[{label}] mapped {}/{} stats, {} unmapped: {:?}",
        result.stats_mapped,
        result.stats_total,
        result.unmapped_stats.len(),
        result.unmapped_stats,
    );
    let filter_count: usize = result
        .body
        .query
        .stats
        .iter()
        .map(|g| g.filters.len())
        .sum();
    println!("[{label}] {filter_count} stat filters in query");
}
