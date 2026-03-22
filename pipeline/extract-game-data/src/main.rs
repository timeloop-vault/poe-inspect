/// Extract datc64 tables from a PoE GGPK install for the poe-inspect pipeline.
///
/// Usage: extract-game-data -p <poe_install_dir> [-o <output_dir>] [--all]
///
/// By default extracts only the tables needed by poe-data.
/// With --all, extracts ALL ~911 datc64 tables (for research/reference).
use std::path::PathBuf;

use clap::Parser;
use poe_bundle::{BundleReader, BundleReaderRead};

/// Tables needed by poe-data for the core pipeline.
const CORE_TABLES: &[&str] = &[
    "stats",
    "tags",
    "itemclasses",
    "itemclasscategories",
    "baseitemtypes",
    "modfamily",
    "modtype",
    "mods",
    "rarity",
    // Base item type stat tables (DPS/defence calculations)
    "armourtypes",
    "weapontypes",
    "shieldtypes",
    // Display text (data-driven validation, property names, status/influence text)
    "clientstrings",
];

#[derive(clap::Parser)]
#[command(name = "extract-game-data")]
#[command(about = "Extract datc64 tables from PoE GGPK for poe-inspect")]
struct Args {
    /// Path to PoE installation directory (contains Content.ggpk)
    #[arg(short, long, value_name = "INSTALL_DIR")]
    path: PathBuf,

    /// Output directory for extracted .datc64 files
    #[arg(short, long, value_name = "OUTPUT_DIR")]
    output: Option<PathBuf>,

    /// Extract ALL datc64 tables (not just core tables)
    #[arg(long)]
    all: bool,
}

fn main() {
    let args = Args::parse();

    let output_dir = args
        .output
        .unwrap_or_else(|| std::env::temp_dir().join("poe-dat"));
    std::fs::create_dir_all(&output_dir).expect("failed to create output directory");

    println!("PoE install: {}", args.path.display());
    println!("Output dir:  {}", output_dir.display());

    let bundles = BundleReader::from_install(&args.path);

    if args.all {
        let mut tables: Vec<String> = bundles
            .index
            .paths
            .iter()
            .filter(|p| {
                p.starts_with("data/")
                    && p.ends_with(".datc64")
                    && p.matches('/').count() == 1
            })
            .filter_map(|p| {
                p.strip_prefix("data/")
                    .and_then(|s| s.strip_suffix(".datc64"))
                    .map(String::from)
            })
            .collect();
        tables.sort();
        println!("Extracting ALL {} tables\n", tables.len());
        extract_tables(&bundles, &tables, &output_dir);
    } else {
        println!("Extracting {} core tables\n", CORE_TABLES.len());
        let tables: Vec<String> = CORE_TABLES.iter().map(|s| (*s).to_string()).collect();
        extract_tables(&bundles, &tables, &output_dir);
    }

    println!("\nDone.");
}

fn extract_tables(bundles: &BundleReader, tables: &[String], output_dir: &PathBuf) {
    let mut extracted = 0;
    let mut errors = 0;

    for table in tables {
        let dat_path = format!("data/{table}.datc64");
        print!("  {table:40}");

        match bundles.bytes(&dat_path) {
            Ok(bytes) => {
                let out_path = output_dir.join(format!("{table}.datc64"));
                std::fs::write(&out_path, &bytes).expect("failed to write file");
                println!(" {:>10} bytes", bytes.len());
                extracted += 1;
            }
            Err(e) => {
                println!(" ERROR: {e:?}");
                errors += 1;
            }
        }
    }

    println!("\nExtracted: {extracted}, Errors: {errors}");
}
