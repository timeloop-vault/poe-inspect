/// Search the GGPK bundle index for art files matching given patterns.
///
/// Usage: cargo run --example find_art -- <pattern> [pattern2...]
/// Example: cargo run --example find_art -- ItemsHeader ItemsSeparator
use poe_bundle::BundleReader;
use std::path::Path;

fn main() {
    let patterns: Vec<String> = std::env::args().skip(1).collect();
    if patterns.is_empty() {
        eprintln!("Usage: find_art <pattern> [pattern2...]");
        std::process::exit(1);
    }

    let bundles = BundleReader::from_install(Path::new("D:/games/PathofExile"));
    let mut count = 0;
    for path in &bundles.index.paths {
        let lower = path.to_lowercase();
        for pat in &patterns {
            if lower.contains(&pat.to_lowercase()) {
                println!("{path}");
                count += 1;
                break;
            }
        }
    }
    eprintln!("\n{count} files found");
}
