use poe_bundle::{BundleReader, BundleReaderRead};
use std::path::Path;

fn main() {
    let path = std::env::args().nth(1).expect("Usage: list_dat <poe_install_path>");
    let reader = BundleReader::from_install(Path::new(&path));

    // All data/*.datc64 files (English, non-localized)
    let mut dat_files: Vec<&str> = reader.index.paths.iter()
        .filter(|p| p.starts_with("data/") && p.ends_with(".datc64") && p.matches('/').count() == 1)
        .map(|p| p.as_str())
        .collect();
    dat_files.sort();

    println!("=== English data/*.datc64 files ({} total) ===", dat_files.len());
    for f in &dat_files {
        let size = reader.size_of(f).unwrap_or(0);
        println!("  {} ({})", f, size);
    }

    // Localized data files
    let mut lang_dirs: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for p in reader.index.paths.iter()
        .filter(|p| p.starts_with("data/") && p.ends_with(".datc64") && p.matches('/').count() == 2)
    {
        if let Some(lang) = p.strip_prefix("data/").and_then(|s| s.split('/').next()) {
            lang_dirs.insert(lang);
        }
    }
    let mut langs: Vec<_> = lang_dirs.into_iter().collect();
    langs.sort();
    println!("\n=== Languages with localized data ===");
    for l in &langs {
        let count = reader.index.paths.iter()
            .filter(|p| p.starts_with(&format!("data/{}/", l)) && p.ends_with(".datc64"))
            .count();
        println!("  {} ({} files)", l, count);
    }

    // Stat description files
    let mut stat_desc: Vec<&str> = reader.index.paths.iter()
        .filter(|p| p.starts_with("metadata/statdescriptions/") && p.ends_with(".txt"))
        .map(|p| p.as_str())
        .collect();
    stat_desc.sort();
    println!("\n=== Stat description files ({}) ===", stat_desc.len());
    for f in &stat_desc {
        let size = reader.size_of(f).unwrap_or(0);
        println!("  {} ({})", f, size);
    }
}
