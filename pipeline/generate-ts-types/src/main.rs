/// Generate TypeScript type definitions from Rust types via ts-rs.
///
/// Usage: generate-ts-types -o <output_dir>
///
/// Replaces the old `cargo test --features ts -- export_bindings` pattern.
/// All types with `#[derive(TS)]` are exported to the output directory.
use std::path::PathBuf;

use clap::Parser;
use ts_rs::{Config, TS};

#[derive(clap::Parser)]
#[command(name = "generate-ts-types")]
#[command(about = "Generate TypeScript type definitions from Rust types")]
struct Args {
    /// Output directory for generated .ts files
    #[arg(short, long, value_name = "OUTPUT_DIR")]
    output: PathBuf,
}

fn main() {
    let args = Args::parse();

    std::fs::create_dir_all(&args.output).expect("failed to create output directory");

    let cfg = Config::new().with_out_dir(&args.output);

    println!("Generating TypeScript types to: {}", args.output.display());

    let mut count = 0u32;
    let mut errors = 0u32;

    // Export root types with all dependencies.
    // export_all() transitively exports all referenced types.
    let exports: Vec<(&str, Box<dyn Fn() -> Result<(), ts_rs::ExportError>>)> = vec![
        // poe-item: ResolvedItem pulls in all item types
        ("ResolvedItem", Box::new(|| poe_item::types::ResolvedItem::export_all(&cfg))),
        // poe-eval: ItemEvaluation pulls in all eval types
        ("ItemEvaluation", Box::new(|| poe_eval::ItemEvaluation::export_all(&cfg))),
        // poe-trade: root types for trade query + edit schema
        ("QueryBuildResult", Box::new(|| poe_trade::QueryBuildResult::export_all(&cfg))),
        ("TradeSearchBody", Box::new(|| poe_trade::TradeSearchBody::export_all(&cfg))),
        ("TradeFilterConfig", Box::new(|| poe_trade::TradeFilterConfig::export_all(&cfg))),
        ("TradeQueryConfig", Box::new(|| poe_trade::TradeQueryConfig::export_all(&cfg))),
        ("PriceCheckResult", Box::new(|| poe_trade::PriceCheckResult::export_all(&cfg))),
        ("LeagueList", Box::new(|| poe_trade::LeagueList::export_all(&cfg))),
        ("ListingStatus", Box::new(|| poe_trade::ListingStatus::export_all(&cfg))),
        // poe-trade: filter schema types
        ("TradeEditSchema", Box::new(|| poe_trade::filter_schema::TradeEditSchema::export_all(&cfg))),
        // poe-eval: profile/rule editor types
        ("Profile", Box::new(|| poe_eval::Profile::export_all(&cfg))),
        ("PredicateSchema", Box::new(|| poe_eval::PredicateSchema::export_all(&cfg))),
        // poe-data: stat suggestions
        ("StatSuggestion", Box::new(|| poe_data::StatSuggestion::export_all(&cfg))),
    ];

    for (name, export_fn) in &exports {
        match export_fn() {
            Ok(()) => count += 1,
            Err(e) => {
                eprintln!("  ERROR exporting {name}: {e}");
                errors += 1;
            }
        }
    }

    // Count actual .ts files written (export_all writes dependencies too)
    let ts_files = std::fs::read_dir(&args.output)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|ext| ext == "ts")
                })
                .count()
        })
        .unwrap_or(0);

    println!(
        "  Exported {count} root types ({ts_files} .ts files written), {errors} errors"
    );
}
