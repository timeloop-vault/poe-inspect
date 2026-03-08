/// Extract a specific file from the GGPK bundle to disk.
///
/// Usage: cargo run --example extract_file -- <bundle_path> <output_path>
/// Example: cargo run --example extract_file -- "art/uiimages1.txt" /tmp/uiimages1.txt
use poe_bundle::{BundleReader, BundleReaderRead};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: extract_file <bundle_path> <output_path>");
        std::process::exit(1);
    }
    let bundle_path = &args[1];
    let output_path = &args[2];

    let bundles = BundleReader::from_install(Path::new("D:/games/PathofExile"));

    match bundles.bytes(bundle_path) {
        Ok(bytes) => {
            std::fs::write(output_path, &bytes).expect("failed to write output file");
            eprintln!("Extracted {} bytes → {}", bytes.len(), output_path);
        }
        Err(e) => {
            eprintln!("ERROR: {e:?}");
            std::process::exit(1);
        }
    }
}
