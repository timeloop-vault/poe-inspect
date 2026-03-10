//! End-to-end test: fixture → parse → build query → search trade API → fetch prices.
//!
//! These tests hit the live pathofexile.com trade API and are `#[ignore]`d by default.
//! Run manually with:
//!
//! ```sh
//! cargo test -p poe-trade --test e2e -- --ignored --nocapture
//! ```

use std::path::PathBuf;
use std::sync::OnceLock;

use poe_data::GameData;
use poe_item::resolve;
use poe_trade::client::TradeClient;
use poe_trade::query::build_query;
use poe_trade::types::{TradeQueryConfig, TradeStatsIndex, TradeStatsResponse};

fn game_data() -> &'static GameData {
    static GD: OnceLock<GameData> = OnceLock::new();
    GD.get_or_init(|| {
        let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../poe-data/data");
        poe_data::load(&data_dir).expect("game data required")
    })
}

fn trade_index() -> &'static TradeStatsIndex {
    static IDX: OnceLock<TradeStatsIndex> = OnceLock::new();
    IDX.get_or_init(|| {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/trade_stats_3.28.json");
        let file = std::fs::File::open(&path).expect("fixture not found");
        let response: TradeStatsResponse =
            serde_json::from_reader(std::io::BufReader::new(file)).expect("valid JSON");
        TradeStatsIndex::from_response(&response, game_data()).index
    })
}

fn parse_fixture(name: &str) -> poe_item::types::ResolvedItem {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/items")
        .join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("fixture not found at {}: {e}", path.display()));
    let raw = poe_item::parse(&text).expect("fixture should parse");
    resolve(&raw, game_data())
}

#[tokio::test]
#[ignore = "hits live trade API — run with --ignored"]
async fn search_rare_body_armour() {
    let item = parse_fixture("rare-body-armour-craft-hybrid-and-normal-life-mod.txt");
    let config = TradeQueryConfig::new("Mirage");
    let query_result = build_query(&item, trade_index(), &config);

    println!(
        "Query: {}/{} stats mapped, {} unmapped: {:?}",
        query_result.stats_mapped,
        query_result.stats_total,
        query_result.unmapped_stats.len(),
        query_result.unmapped_stats,
    );

    let json = serde_json::to_string_pretty(&query_result.body).unwrap();
    println!("Request body:\n{json}");

    let mut client = TradeClient::new();
    let result = client
        .price_check(&query_result.body, &config)
        .await
        .expect("price check should succeed");

    println!("Search ID: {}", result.search_id);
    println!("Total listings: {}", result.total);
    println!("Trade URL: {}", result.trade_url);
    println!("Prices ({}):", result.prices.len());
    for price in &result.prices {
        println!("  {} {}", price.amount, price.currency);
    }

    assert!(!result.search_id.is_empty(), "should have a search ID");
    // Total could be 0 if nobody is selling this exact combo, but search should work
}

#[tokio::test]
#[ignore = "hits live trade API — run with --ignored"]
async fn search_rare_weapon() {
    let item = parse_fixture("battered-foil-rare-ess-craft.txt");
    let config = TradeQueryConfig::new("Mirage");
    let query_result = build_query(&item, trade_index(), &config);

    println!(
        "Battered Foil: {}/{} stats mapped",
        query_result.stats_mapped, query_result.stats_total,
    );

    let mut client = TradeClient::new();
    let result = client
        .price_check(&query_result.body, &config)
        .await
        .expect("price check should succeed");

    println!("Total: {}, prices: {:?}", result.total, result.prices);
    assert!(!result.search_id.is_empty());
}

#[tokio::test]
#[ignore = "hits live trade API — run with --ignored"]
async fn fetch_stats_live() {
    let client = TradeClient::new();
    let stats = client.fetch_stats().await.expect("should fetch stats");

    println!(
        "Fetched {} categories, {} total entries",
        stats.result.len(),
        stats.result.iter().map(|c| c.entries.len()).sum::<usize>(),
    );

    assert!(!stats.result.is_empty());
    assert!(
        stats.result.iter().map(|c| c.entries.len()).sum::<usize>() > 10_000,
        "expected 10k+ stats"
    );
}
