/// Generate TypeScript type definitions from Rust types via ts-rs.
///
/// Usage: generate-ts-types -o <`output_dir`>
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

/// Export a single root type, counting successes/errors.
fn export<T: TS + 'static>(name: &str, cfg: &Config, count: &mut u32, errors: &mut u32) {
    match T::export_all(cfg) {
        Ok(()) => *count += 1,
        Err(e) => {
            eprintln!("  ERROR exporting {name}: {e}");
            *errors += 1;
        }
    }
}

fn main() {
    let args = Args::parse();

    std::fs::create_dir_all(&args.output).expect("failed to create output directory");

    let cfg = Config::new().with_out_dir(&args.output);

    println!("Generating TypeScript types to: {}", args.output.display());

    let mut count = 0u32;
    let mut errors = 0u32;

    // poe-item: ResolvedItem pulls in all item types
    export::<poe_item::types::ResolvedItem>("ResolvedItem", &cfg, &mut count, &mut errors);
    // poe-eval: ItemEvaluation + Profile + PredicateSchema
    export::<poe_eval::ItemEvaluation>("ItemEvaluation", &cfg, &mut count, &mut errors);
    export::<poe_eval::Profile>("Profile", &cfg, &mut count, &mut errors);
    export::<poe_eval::PredicateSchema>("PredicateSchema", &cfg, &mut count, &mut errors);
    // poe-trade: query + config + results
    export::<poe_trade::QueryBuildResult>("QueryBuildResult", &cfg, &mut count, &mut errors);
    export::<poe_trade::TradeSearchBody>("TradeSearchBody", &cfg, &mut count, &mut errors);
    export::<poe_trade::TradeFilterConfig>("TradeFilterConfig", &cfg, &mut count, &mut errors);
    export::<poe_trade::TradeQueryConfig>("TradeQueryConfig", &cfg, &mut count, &mut errors);
    export::<poe_trade::PriceCheckResult>("PriceCheckResult", &cfg, &mut count, &mut errors);
    export::<poe_trade::LeagueList>("LeagueList", &cfg, &mut count, &mut errors);
    export::<poe_trade::ListingStatus>("ListingStatus", &cfg, &mut count, &mut errors);
    // poe-trade: filter schema
    export::<poe_trade::filter_schema::TradeEditSchema>(
        "TradeEditSchema",
        &cfg,
        &mut count,
        &mut errors,
    );
    // poe-data: stat suggestions
    export::<poe_data::StatSuggestion>("StatSuggestion", &cfg, &mut count, &mut errors);

    // Count actual .ts files written (export_all writes dependencies too)
    let ts_files = std::fs::read_dir(&args.output)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "ts"))
                .count()
        })
        .unwrap_or(0);

    println!("  Exported {count} root types ({ts_files} .ts files written), {errors} errors");
}
