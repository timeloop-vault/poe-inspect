/// Extract dat tables from GGPK to individual files.
///
/// Usage: extract_dat -p <poe_install_dir> [-o <output_dir>] [--all]
///
/// By default extracts only the tables needed by poe-dat.
/// With --all, extracts ALL ~911 datc64 tables (for research/reference).
use std::path::PathBuf;

use clap::Parser;
use poe_bundle::{BundleReader, BundleReaderRead};

/// Tables needed by poe-dat for the core pipeline.
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
];

#[derive(clap::Parser)]
#[command(name = "extract_dat")]
#[command(about = "Extract dat tables from PoE GGPK for poe-dat")]
struct Args {
    #[arg(short, long, value_name = "INSTALL_DIR")]
    path: PathBuf,

    #[arg(short, long, value_name = "OUTPUT_DIR", default_value = "")]
    output: String,

    /// Extract ALL datc64 tables (not just core tables).
    #[arg(long)]
    all: bool,
}

fn main() {
    let args = Args::parse();

    let output_dir = if args.output.is_empty() {
        std::env::temp_dir().join("poe-dat")
    } else {
        PathBuf::from(&args.output)
    };
    std::fs::create_dir_all(&output_dir).expect("failed to create output directory");

    println!("PoE install: {}", args.path.display());
    println!("Output dir:  {}", output_dir.display());

    let bundles = BundleReader::from_install(&args.path);

    if args.all {
        // Discover all English datc64 files from the GGPK index
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

    println!("\nDone. Run poe-dat tests with: cargo test -p poe-dat --test extract_tables -- --nocapture");
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
