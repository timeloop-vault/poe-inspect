/// Extract datc64 tables and unique item art from a `PoE` GGPK install.
///
/// Usage: extract-game-data -p <`poe_install_dir`> [-o <`output_dir`>] [--all]
///
/// By default extracts only the tables needed by poe-data.
/// With --all, extracts ALL ~911 datc64 tables (for research/reference).
///
/// Also extracts unique item 2D art (DDS → PNG) and generates an enriched
/// `unique_items.json` with art paths when --art-dir is specified.
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
    /// Path to `PoE` installation directory (contains Content.ggpk)
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

    /// Output path for `unique_items.json`.
    /// Fetches the trade API, cross-references with GGPK art, writes enriched JSON.
    #[arg(long, value_name = "UNIQUE_ITEMS_JSON")]
    unique_items: Option<PathBuf>,

    /// Output path for `reverse_index.json`.
    /// Extracts stat description files from GGPK, builds the reverse lookup index.
    #[arg(long, value_name = "REVERSE_INDEX_JSON")]
    reverse_index: Option<PathBuf>,

    /// Output path for `mod_families.txt`.
    /// Dumps sorted mod family IDs from the Mods table.
    #[arg(long, value_name = "MOD_FAMILIES_TXT")]
    mod_families: Option<PathBuf>,

    /// Maximum art width in pixels (height scaled proportionally). Default: 78.
    #[arg(long, value_name = "PIXELS", default_value = "78")]
    art_max_width: u32,
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
                p.starts_with("data/") && p.ends_with(".datc64") && p.matches('/').count() == 1
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
    let art_map = if let Some(ref art_dir) = args.art_dir {
        std::fs::create_dir_all(art_dir).expect("failed to create art output directory");
        println!("\n--- Unique item art extraction ---");
        println!("Art dir: {}\n", art_dir.display());
        extract_unique_art(&bundles, &output_dir, art_dir, args.art_max_width)
    } else {
        HashMap::new()
    };

    // Generate unique_items.json if --unique-items is specified
    if let Some(ref unique_items_path) = args.unique_items {
        println!("\n--- Generating unique_items.json ---");
        generate_unique_items(unique_items_path, &art_map);
    }

    // Build reverse index if --reverse-index is specified
    if let Some(ref ri_path) = args.reverse_index {
        println!("\n--- Building reverse index ---");
        build_reverse_index(&bundles, ri_path);
    }

    // Dump mod families if --mod-families is specified
    if let Some(ref mf_path) = args.mod_families {
        println!("\n--- Dumping mod families ---");
        dump_mod_families(&output_dir, mf_path);
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
/// Reads `UniqueStashLayout` → Words (name) + `ItemVisualIdentity` (DDS path),
/// extracts each DDS file from the GGPK, decodes to RGBA, writes as PNG.
fn extract_unique_art(
    bundles: &BundleReader,
    dat_dir: &Path,
    art_dir: &Path,
    max_width: u32,
) -> HashMap<String, String> {
    let Some(layout) = load_dat(dat_dir, "uniquestashlayout") else {
        eprintln!("  ERROR: uniquestashlayout.datc64 not found in output dir");
        return HashMap::new();
    };
    let Some(words) = load_dat(dat_dir, "words") else {
        eprintln!("  ERROR: words.datc64 not found in output dir");
        return HashMap::new();
    };
    let Some(vis) = load_dat(dat_dir, "itemvisualidentity") else {
        eprintln!("  ERROR: itemvisualidentity.datc64 not found in output dir");
        return HashMap::new();
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

        #[allow(clippy::cast_possible_truncation)] // Row indices well within usize range
        let name = match words_rows.get(words_idx as usize) {
            Some(w) => &w.text,
            None => continue,
        };
        #[allow(clippy::cast_possible_truncation)]
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
            .map(|c| {
                if c.is_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect();
        let out_filename = format!("{safe_name}.webp");

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

        // Convert DDS → WebP (resized)
        match dds_to_webp(&dds_bytes, &art_dir.join(&out_filename), max_width) {
            Ok(()) => {
                name_to_art.insert(name.clone(), out_filename);
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

    println!("\n  Art extracted: {extracted}, Skipped (alt art): {skipped}, Errors: {errors}");

    name_to_art
}

// ── Trade API types ────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct TradeItemsResponse {
    result: Vec<TradeItemCategory>,
}

#[derive(serde::Deserialize)]
struct TradeItemCategory {
    entries: Vec<TradeItemEntry>,
}

#[derive(serde::Deserialize)]
struct TradeItemEntry {
    #[serde(rename = "type")]
    base_type: String,
    name: Option<String>,
    flags: Option<TradeItemFlags>,
    /// Variant discriminator (e.g., "legacy"). Skip these.
    disc: Option<String>,
}

#[derive(serde::Deserialize)]
struct TradeItemFlags {
    #[serde(default)]
    unique: bool,
}

// ── Output JSON type ───────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct UniqueItemJson {
    name: String,
    base_type: String,
    art: String,
}

/// Fetch the trade API items endpoint and generate `unique_items.json`,
/// merging art filenames from the GGPK extraction.
fn generate_unique_items(output_path: &Path, art_map: &HashMap<String, String>) {
    println!("  Fetching trade API /data/items...");

    let client = reqwest::blocking::Client::builder()
        .user_agent("poe-inspect-2/0.1 (contact: github.com/timeloop-vault/poe-inspect)")
        .build()
        .expect("failed to build HTTP client");

    let response: TradeItemsResponse = client
        .get("https://www.pathofexile.com/api/trade/data/items")
        .send()
        .expect("failed to fetch trade API items")
        .json()
        .expect("failed to parse trade API response");

    let mut uniques: Vec<UniqueItemJson> = Vec::new();
    let mut with_art = 0u32;

    for cat in &response.result {
        for entry in &cat.entries {
            let is_unique = entry.flags.as_ref().is_some_and(|f| f.unique);
            if !is_unique || entry.name.is_none() || entry.disc.is_some() {
                continue;
            }
            let name = entry.name.as_deref().unwrap();
            let art = art_map.get(name).cloned().unwrap_or_default();
            if !art.is_empty() {
                with_art += 1;
            }
            uniques.push(UniqueItemJson {
                name: name.to_string(),
                base_type: entry.base_type.clone(),
                art,
            });
        }
    }

    uniques.sort_by(|a, b| {
        a.base_type
            .cmp(&b.base_type)
            .then_with(|| a.name.cmp(&b.name))
    });

    let json = serde_json::to_string_pretty(&uniques).expect("failed to serialize unique items");
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(output_path, format!("{json}\n")).expect("failed to write unique_items.json");

    println!(
        "  Wrote {} unique items ({with_art} with art) to {}",
        uniques.len(),
        output_path.display()
    );
}

// ── Stat description GGPK paths ────────────────────────────────────────────

/// Base + additional stat description files to merge into the reverse index.
const STAT_DESC_FILES: &[&str] = &[
    "metadata/statdescriptions/stat_descriptions.txt",
    "metadata/statdescriptions/map_stat_descriptions.txt",
    "metadata/statdescriptions/atlas_stat_descriptions.txt",
    "metadata/statdescriptions/sanctum_relic_stat_descriptions.txt",
    "metadata/statdescriptions/heist_equipment_stat_descriptions.txt",
    "metadata/statdescriptions/expedition_relic_stat_descriptions.txt",
];

/// Extract stat description files from GGPK, parse them, build the reverse
/// index, and save as JSON. Replaces the old `save_reverse_index` test.
fn build_reverse_index(bundles: &BundleReader, output_path: &Path) {
    use poe_dat::stat_desc;

    let mut index: Option<stat_desc::ReverseIndex> = None;

    for ggpk_path in STAT_DESC_FILES {
        print!("  {ggpk_path:<65}");
        let bytes = match bundles.bytes(ggpk_path) {
            Ok(b) => b,
            Err(e) => {
                println!(" SKIP ({e:?})");
                continue;
            }
        };

        // Stat desc files are UTF-16LE in the GGPK — convert to UTF-8.
        let utf16: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        // Strip BOM if present (U+FEFF)
        let utf16 = if utf16.first() == Some(&0xFEFF) {
            &utf16[1..]
        } else {
            &utf16
        };
        let text = String::from_utf16_lossy(utf16);

        let file = match stat_desc::parse(&text) {
            Ok(f) => f,
            Err(e) => {
                println!(" PARSE ERROR: {e}");
                continue;
            }
        };

        if let Some(ref mut idx) = index {
            let before = idx.len();
            idx.merge(&file);
            println!(" +{} (total: {})", idx.len() - before, idx.len());
        } else {
            let idx = stat_desc::ReverseIndex::from_file(&file);
            println!(" {} patterns", idx.len());
            index = Some(idx);
        }
    }

    let Some(index) = index else {
        eprintln!("  ERROR: no stat description files found in GGPK");
        return;
    };

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    index
        .save(output_path)
        .expect("failed to save reverse index");

    let size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);
    println!(
        "  Wrote {} patterns ({} bytes) to {}",
        index.len(),
        size,
        output_path.display()
    );
}

/// Dump sorted mod family IDs from the extracted Mods table.
/// Replaces the old `dump_mod_families_txt` test.
fn dump_mod_families(dat_dir: &Path, output_path: &Path) {
    let Some(dat) = load_dat(dat_dir, "modfamily") else {
        eprintln!("  ERROR: modfamily.datc64 not found");
        return;
    };

    let rows = tables::extract_mod_families(&dat);
    let mut names: Vec<&str> = rows.iter().map(|f| f.id.as_str()).collect();
    names.sort_unstable();

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(output_path, names.join("\n")).expect("failed to write mod families");
    println!(
        "  Wrote {} families to {}",
        names.len(),
        output_path.display()
    );
}

/// Decode a DDS file, resize to `max_width`, and write as WebP.
#[allow(clippy::cast_possible_truncation)] // Pixel math always in u8 range
fn dds_to_webp(dds_bytes: &[u8], out_path: &Path, max_width: u32) -> Result<(), String> {
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

    // Build image and resize if needed
    let img = image::RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| "failed to create image buffer".to_string())?;

    let img = if max_width > 0 && width > max_width {
        let scale = f64::from(max_width) / f64::from(width);
        #[allow(clippy::cast_sign_loss)] // scale is always positive
        let new_height = (f64::from(height) * scale).round() as u32;
        image::imageops::resize(
            &img,
            max_width,
            new_height,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        img
    };

    img.save(out_path).map_err(|e| format!("WebP write: {e}"))?;

    Ok(())
}

// ── Block compression decoders ─────────────────────────────────────────────
// Minimal BC1/BC3/BC7 decoders for 2D item art.

#[allow(clippy::cast_possible_truncation)]
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

#[allow(clippy::cast_possible_truncation)]
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

#[allow(clippy::cast_possible_truncation)]
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

#[allow(clippy::cast_possible_truncation)]
fn lerp_color(a: [u8; 4], b: [u8; 4], t: u16, total: u16) -> [u8; 4] {
    [
        ((u16::from(a[0]) * (total - t) + u16::from(b[0]) * t) / total) as u8,
        ((u16::from(a[1]) * (total - t) + u16::from(b[1]) * t) / total) as u8,
        ((u16::from(a[2]) * (total - t) + u16::from(b[2]) * t) / total) as u8,
        ((u16::from(a[3]) * (total - t) + u16::from(b[3]) * t) / total) as u8,
    ]
}
