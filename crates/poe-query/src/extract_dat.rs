/// Extract specific dat tables from GGPK to individual files.
///
/// Usage: extract_dat -p <poe_install_dir> -o <output_dir>
///
/// Writes raw .datc64 bytes to <output_dir>/<table>.datc64 for each
/// table needed by poe-dat.
use std::path::PathBuf;

use clap::Parser;
use poe_bundle::{BundleReader, BundleReaderRead};

const TABLES: &[&str] = &[
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
    println!();

    let bundles = BundleReader::from_install(&args.path);

    for table in TABLES {
        let dat_path = format!("data/{table}.datc64");
        print!("  {table:30}");

        match bundles.bytes(&dat_path) {
            Ok(bytes) => {
                let out_path = output_dir.join(format!("{table}.datc64"));
                std::fs::write(&out_path, &bytes).expect("failed to write file");
                println!(" {} bytes → {}", bytes.len(), out_path.display());
            }
            Err(e) => {
                println!(" ERROR: {e:?}");
            }
        }
    }

    println!("\nDone. Run poe-dat tests with: cargo test -p poe-dat --test extract_tables -- --nocapture");
}
