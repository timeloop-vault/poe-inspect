/// Extract datc64 tables and unique item art from a PoE GGPK install.
///
/// Usage: extract-game-data -p <poe_install_dir> [-o <output_dir>] [--all]
///
/// By default extracts only the tables needed by poe-data.
/// With --all, extracts ALL ~911 datc64 tables (for research/reference).
///
/// Also extracts unique item 2D art (DDS → PNG) and generates an enriched
/// unique_items.json with art paths when --art-dir is specified.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use clap::Parser;
use poe_bundle::{BundleReader, BundleReaderRead};
use poe_dat::dat_reader::DatFile;
use poe_dat::tables;

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
    // Unique item data (disambiguation picker, art extraction)
    "uniquestashlayout",
    "uniquestashtypes",
    "words",
    "itemvisualidentity",
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

    /// Output directory for unique item art PNGs.
    /// When set, extracts DDS art from GGPK, converts to PNG, and writes here.
    #[arg(long, value_name = "ART_DIR")]
    art_dir: Option<PathBuf>,
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

    // Extract unique item art if --art-dir is specified
    if let Some(ref art_dir) = args.art_dir {
        std::fs::create_dir_all(art_dir).expect("failed to create art output directory");
        println!("\n--- Unique item art extraction ---");
        println!("Art dir: {}\n", art_dir.display());
        extract_unique_art(&bundles, &output_dir, art_dir);
    }

    println!("\nDone.");
}

fn extract_tables(bundles: &BundleReader, tables: &[String], output_dir: &Path) {
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

/// Read a datc64 file from the output directory.
fn load_dat(dir: &Path, name: &str) -> Option<DatFile> {
    let path = dir.join(format!("{name}.datc64"));
    let bytes = std::fs::read(&path).ok()?;
    DatFile::from_bytes(bytes).ok()
}

/// Extract unique item 2D art from the GGPK and convert DDS → PNG.
///
/// Reads UniqueStashLayout → Words (name) + ItemVisualIdentity (DDS path),
/// extracts each DDS file from the GGPK, decodes to RGBA, writes as PNG.
fn extract_unique_art(bundles: &BundleReader, dat_dir: &Path, art_dir: &Path) {
    let Some(layout) = load_dat(dat_dir, "uniquestashlayout") else {
        eprintln!("  ERROR: uniquestashlayout.datc64 not found in output dir");
        return;
    };
    let Some(words) = load_dat(dat_dir, "words") else {
        eprintln!("  ERROR: words.datc64 not found in output dir");
        return;
    };
    let Some(vis) = load_dat(dat_dir, "itemvisualidentity") else {
        eprintln!("  ERROR: itemvisualidentity.datc64 not found in output dir");
        return;
    };

    let layout_rows = tables::extract_unique_stash_layout(&layout);
    let words_rows = tables::extract_words(&words);
    let vis_rows = tables::extract_item_visual_identity(&vis);

    println!("  UniqueStashLayout: {} rows", layout_rows.len());
    println!("  Words: {} rows", words_rows.len());
    println!("  ItemVisualIdentity: {} rows\n", vis_rows.len());

    // Build name → art path mapping and extract DDS files
    let mut extracted = 0u32;
    let mut skipped = 0u32;
    let mut errors = 0u32;
    let mut name_to_art: HashMap<String, String> = HashMap::new();

    for row in &layout_rows {
        // Skip alternate art variants
        if row.is_alternate_art {
            skipped += 1;
            continue;
        }

        let Some(words_idx) = row.words_key else {
            continue;
        };
        let Some(vis_idx) = row.visual_identity_key else {
            continue;
        };

        let name = match words_rows.get(words_idx as usize) {
            Some(w) => &w.text,
            None => continue,
        };
        let dds_path = match vis_rows.get(vis_idx as usize) {
            Some(v) => &v.dds_file,
            None => continue,
        };

        if name.is_empty() || dds_path.is_empty() {
            continue;
        }

        // Derive a filesystem-safe filename from the unique name
        let safe_name: String = name
            .chars()
            .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
            .collect();
        let png_filename = format!("{safe_name}.png");

        // Extract DDS from GGPK
        let dds_bytes = match bundles.bytes(dds_path) {
            Ok(b) => b,
            Err(_) => {
                // Try lowercase path (GGPK paths are case-insensitive)
                match bundles.bytes(&dds_path.to_lowercase()) {
                    Ok(b) => b,
                    Err(e) => {
                        if errors < 5 {
                            eprintln!("  WARN: failed to extract {dds_path}: {e:?}");
                        }
                        errors += 1;
                        continue;
                    }
                }
            }
        };

        // Convert DDS → PNG
        match dds_to_png(&dds_bytes, &art_dir.join(&png_filename)) {
            Ok(()) => {
                name_to_art.insert(name.clone(), png_filename);
                extracted += 1;
            }
            Err(e) => {
                if errors < 5 {
                    eprintln!("  WARN: failed to convert {name}: {e}");
                }
                errors += 1;
            }
        }
    }

    if errors > 5 {
        eprintln!("  ... and {} more errors", errors - 5);
    }

    println!(
        "\n  Art extracted: {extracted}, Skipped (alt art): {skipped}, Errors: {errors}"
    );

    // Write the name → art filename mapping as JSON (for enriching unique_items.json)
    let map_path = art_dir.join("_art_map.json");
    let json = serde_json::to_string_pretty(&name_to_art).expect("failed to serialize art map");
    std::fs::write(&map_path, json).expect("failed to write art map");
    println!("  Art map written to: {}", map_path.display());
}

/// Decode a DDS file and write it as PNG.
fn dds_to_png(dds_bytes: &[u8], out_path: &Path) -> Result<(), String> {
    let dds = ddsfile::Dds::read(std::io::Cursor::new(dds_bytes))
        .map_err(|e| format!("DDS parse: {e}"))?;

    let width = dds.get_width();
    let height = dds.get_height();

    // Use raw data directly — get_data(0) fails on some PoE DDS files.
    // For the top mip level, data starts at offset 0 in dds.data.
    let data = &dds.data;

    // Decode based on format
    let rgba = match dds.get_dxgi_format() {
        Some(ddsfile::DxgiFormat::BC1_UNorm | ddsfile::DxgiFormat::BC1_UNorm_sRGB) => {
            decode_bc1(data, width, height)
        }
        Some(ddsfile::DxgiFormat::BC3_UNorm | ddsfile::DxgiFormat::BC3_UNorm_sRGB) => {
            decode_bc3(data, width, height)
        }
        Some(ddsfile::DxgiFormat::BC7_UNorm | ddsfile::DxgiFormat::BC7_UNorm_sRGB) => {
            decode_bc7(data, width, height)
        }
        Some(ddsfile::DxgiFormat::R8G8B8A8_UNorm | ddsfile::DxgiFormat::R8G8B8A8_UNorm_sRGB) => {
            let top_mip = (width * height * 4) as usize;
            Ok(data[..top_mip.min(data.len())].to_vec())
        }
        Some(ddsfile::DxgiFormat::B8G8R8A8_UNorm | ddsfile::DxgiFormat::B8G8R8A8_UNorm_sRGB) => {
            let top_mip = (width * height * 4) as usize;
            let mut rgba = data[..top_mip.min(data.len())].to_vec();
            for pixel in rgba.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
            Ok(rgba)
        }
        Some(fmt) => Err(format!("unsupported DXGI format: {fmt:?}")),
        None => {
            // Try D3D format (older DDS files)
            match dds.get_d3d_format() {
                Some(ddsfile::D3DFormat::DXT1) => decode_bc1(data, width, height),
                Some(ddsfile::D3DFormat::DXT5) => decode_bc3(data, width, height),
                Some(ddsfile::D3DFormat::A8R8G8B8) => {
                    let top_mip = (width * height * 4) as usize;
                    let src = &data[..top_mip.min(data.len())];
                    let mut rgba = Vec::with_capacity(src.len());
                    for pixel in src.chunks_exact(4) {
                        rgba.extend_from_slice(&[pixel[1], pixel[2], pixel[3], pixel[0]]);
                    }
                    Ok(rgba)
                }
                Some(fmt) => Err(format!("unsupported D3D format: {fmt:?}")),
                None => Err("unknown DDS format".to_string()),
            }
        }
    }?;

    // Write PNG
    let img = image::RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| "failed to create image buffer".to_string())?;
    img.save(out_path)
        .map_err(|e| format!("PNG write: {e}"))?;

    Ok(())
}

// ── Block compression decoders ─────────────────────────────────────────────
// Minimal BC1/BC3/BC7 decoders for 2D item art.

fn decode_bc1(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    let blocks_x = width.div_ceil(4);
    let blocks_y = height.div_ceil(4);

    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let block_idx = (by * blocks_x + bx) as usize;
            let offset = block_idx * 8;
            if offset + 8 > data.len() {
                return Err("BC1 data truncated".to_string());
            }
            let block = &data[offset..offset + 8];
            decode_bc1_block(block, &mut rgba, bx, by, width, height);
        }
    }
    Ok(rgba)
}

fn decode_bc1_block(block: &[u8], rgba: &mut [u8], bx: u32, by: u32, width: u32, height: u32) {
    let c0 = u16::from_le_bytes([block[0], block[1]]);
    let c1 = u16::from_le_bytes([block[2], block[3]]);
    let mut colors = [[0u8; 4]; 4];

    colors[0] = rgb565_to_rgba(c0);
    colors[1] = rgb565_to_rgba(c1);

    if c0 > c1 {
        colors[2] = lerp_color(colors[0], colors[1], 1, 3);
        colors[3] = lerp_color(colors[0], colors[1], 2, 3);
    } else {
        colors[2] = lerp_color(colors[0], colors[1], 1, 2);
        colors[3] = [0, 0, 0, 0]; // transparent
    }

    let indices = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);
    for py in 0..4 {
        for px in 0..4 {
            let x = bx * 4 + px;
            let y = by * 4 + py;
            if x >= width || y >= height {
                continue;
            }
            let bit_pos = (py * 4 + px) * 2;
            let idx = ((indices >> bit_pos) & 3) as usize;
            let dst = ((y * width + x) * 4) as usize;
            rgba[dst..dst + 4].copy_from_slice(&colors[idx]);
        }
    }
}

fn decode_bc3(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    let blocks_x = width.div_ceil(4);
    let blocks_y = height.div_ceil(4);

    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let block_idx = (by * blocks_x + bx) as usize;
            let offset = block_idx * 16;
            if offset + 16 > data.len() {
                return Err("BC3 data truncated".to_string());
            }
            let block = &data[offset..offset + 16];

            // First 8 bytes: alpha block
            let alpha_block = &block[0..8];
            // Last 8 bytes: BC1 color block
            let color_block = &block[8..16];

            decode_bc1_block(color_block, &mut rgba, bx, by, width, height);

            // Decode alpha and overwrite
            let a0 = alpha_block[0];
            let a1 = alpha_block[1];
            let mut alphas = [0u8; 8];
            alphas[0] = a0;
            alphas[1] = a1;
            if a0 > a1 {
                for i in 1..7 {
                    alphas[i + 1] =
                        ((u16::from(a0) * (7 - i as u16) + u16::from(a1) * i as u16) / 7) as u8;
                }
            } else {
                for i in 1..5 {
                    alphas[i + 1] =
                        ((u16::from(a0) * (5 - i as u16) + u16::from(a1) * i as u16) / 5) as u8;
                }
                alphas[6] = 0;
                alphas[7] = 255;
            }

            // 48-bit index block (6 bytes, 3 bits per pixel)
            let mut alpha_bits: u64 = 0;
            for i in 0..6 {
                alpha_bits |= u64::from(alpha_block[2 + i]) << (8 * i);
            }

            for py in 0..4u32 {
                for px in 0..4u32 {
                    let x = bx * 4 + px;
                    let y = by * 4 + py;
                    if x >= width || y >= height {
                        continue;
                    }
                    let bit_pos = (py * 4 + px) * 3;
                    let idx = ((alpha_bits >> bit_pos) & 7) as usize;
                    let dst = ((y * width + x) * 4 + 3) as usize;
                    rgba[dst] = alphas[idx];
                }
            }
        }
    }
    Ok(rgba)
}

fn decode_bc7(_data: &[u8], _width: u32, _height: u32) -> Result<Vec<u8>, String> {
    // BC7 decoding is complex (8 modes, 64 partitions each).
    // Placeholder — return error so we know which items need it.
    Err("BC7 decoding not yet implemented — add texture2ddecoder crate if needed".to_string())
}

fn rgb565_to_rgba(c: u16) -> [u8; 4] {
    let r = ((c >> 11) & 0x1F) as u8;
    let g = ((c >> 5) & 0x3F) as u8;
    let b = (c & 0x1F) as u8;
    [
        (r << 3) | (r >> 2),
        (g << 2) | (g >> 4),
        (b << 3) | (b >> 2),
        255,
    ]
}

fn lerp_color(a: [u8; 4], b: [u8; 4], t: u16, total: u16) -> [u8; 4] {
    [
        ((u16::from(a[0]) * (total - t) + u16::from(b[0]) * t) / total) as u8,
        ((u16::from(a[1]) * (total - t) + u16::from(b[1]) * t) / total) as u8,
        ((u16::from(a[2]) * (total - t) + u16::from(b[2]) * t) / total) as u8,
        ((u16::from(a[3]) * (total - t) + u16::from(b[3]) * t) / total) as u8,
    ]
}
