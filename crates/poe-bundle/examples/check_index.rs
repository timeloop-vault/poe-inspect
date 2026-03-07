use poe_bundle::{BundleReader, BundleReaderRead};
use std::path::Path;

fn main() {
    let path = std::env::args().nth(1).expect("Usage: check_index <poe_install_path>");
    let reader = BundleReader::from_install(Path::new(&path));

    // Print first 20 paths
    println!("=== First 20 paths ===");
    for (i, p) in reader.index.paths.iter().take(20).enumerate() {
        println!("  {i}: {p}");
    }

    // Search for anything with "Mods" in the path
    println!("\n=== Paths containing 'Mods' (first 10) ===");
    for p in reader.index.paths.iter().filter(|p| p.contains("Mods")).take(10) {
        println!("  {p}");
    }

    // Search for .dat64 files
    println!("\n=== Paths containing '.dat64' (first 20) ===");
    for p in reader.index.paths.iter().filter(|p| p.contains(".dat64") || p.contains(".dat")).take(20) {
        println!("  {p}");
    }

    // Unique top-level directories
    println!("\n=== Top-level directories ===");
    let mut dirs: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for p in reader.index.paths.iter() {
        if let Some(d) = p.split('/').next() {
            dirs.insert(d);
        }
    }
    let mut dirs: Vec<_> = dirs.into_iter().collect();
    dirs.sort();
    for d in &dirs {
        println!("  {d}");
    }

    // Try reading some files via BundleReaderRead
    let test_paths = [
        "Data/Mods.dat64", "data/mods.dat64",
        "data/mods.datc64", "data/mods.datcl64",
    ];
    println!("\n=== File size lookups ===");
    for tp in &test_paths {
        println!("  size_of(\"{tp}\"): {:?}", reader.size_of(tp));
    }

    // Find data .datc64 files
    println!("\n=== data/*.datc64 paths (first 10) ===");
    for p in reader.index.paths.iter()
        .filter(|p| p.starts_with("data/") && p.ends_with(".datc64"))
        .take(10)
    {
        let hash = poe_bundle::util::filepath_hash(p.to_string());
        println!("  {p} => hash={hash}, size={:?}", reader.size_of(p));
    }

    // Also try stat descriptions
    println!("\n=== stat description paths ===");
    for p in reader.index.paths.iter().filter(|p| p.contains("statdescription") || p.contains("stat_description")) {
        println!("  {p}");
    }

    println!("\n=== Total paths: {} ===", reader.index.paths.len());
}
