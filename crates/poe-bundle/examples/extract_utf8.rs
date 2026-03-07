use poe_bundle::{BundleReader, BundleReaderRead};
use std::path::Path;

fn main() {
    let poe_path = std::env::args().nth(1).expect("Usage: extract_utf8 <poe_path> <file_path>");
    let file_path = std::env::args().nth(2).expect("Usage: extract_utf8 <poe_path> <file_path>");
    
    let reader = BundleReader::from_install(Path::new(&poe_path));
    
    match reader.bytes(&file_path) {
        Ok(bytes) => {
            // Detect and convert UTF-16LE (BOM or null-interleaved ASCII)
            let text = if bytes.len() >= 2 && (bytes[0] == 0xFF && bytes[1] == 0xFE) {
                // BOM present
                let u16s: Vec<u16> = bytes[2..].chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect();
                String::from_utf16_lossy(&u16s)
            } else if bytes.len() >= 2 && bytes[1] == 0 {
                // No BOM but looks UTF-16LE
                let u16s: Vec<u16> = bytes.chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect();
                String::from_utf16_lossy(&u16s)
            } else {
                String::from_utf8_lossy(&bytes).to_string()
            };
            print!("{}", text);
        }
        Err(e) => {
            eprintln!("Error reading {}: {}", file_path, e);
            std::process::exit(1);
        }
    }
}
