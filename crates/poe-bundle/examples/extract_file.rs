use poe_bundle::{BundleReader, BundleReaderRead};
use std::path::Path;

fn main() {
    let poe_path = std::env::args().nth(1).expect("Usage: extract_file <poe_path> <file_path>");
    let file_path = std::env::args().nth(2).expect("Usage: extract_file <poe_path> <file_path>");
    
    let reader = BundleReader::from_install(Path::new(&poe_path));
    
    match reader.bytes(&file_path) {
        Ok(bytes) => {
            use std::io::Write;
            std::io::stdout().write_all(&bytes).unwrap();
        }
        Err(e) => {
            eprintln!("Error reading {}: {}", file_path, e);
            std::process::exit(1);
        }
    }
}
