use poe_dat::stat_desc;

#[test]
fn parse_real_stat_descriptions() {
    // Git Bash /tmp maps to Windows %TEMP%
    let path = std::env::temp_dir().join("stat_desc_utf8.txt");
    let path = path.to_str().unwrap();
    if !std::path::Path::new(path).exists() {
        eprintln!("Skipping: {path} not found (extract from GGPK first)");
        return;
    }

    let input = std::fs::read_to_string(path).expect("failed to read file");
    match stat_desc::parse(&input) {
        Ok(file) => {
            println!("Includes: {}", file.includes.len());
            println!("No-descriptions: {}", file.no_descriptions.len());
            println!("Descriptions: {}", file.descriptions.len());

            // Spot-check: should have a reasonable number of descriptions
            assert!(
                file.descriptions.len() > 1000,
                "expected >1000 descriptions, got {}",
                file.descriptions.len()
            );

            // Check a few descriptions have stat IDs and variants
            for desc in file.descriptions.iter().take(5) {
                assert!(
                    !desc.stat_ids.is_empty(),
                    "description should have stat IDs"
                );
                assert!(
                    !desc.languages.is_empty(),
                    "description should have at least one language block"
                );
                for lang in &desc.languages {
                    assert!(
                        !lang.variants.is_empty(),
                        "language block should have variants"
                    );
                }
            }

            // Count transforms and check for Other variants
            let mut other_transforms: std::collections::BTreeSet<String> =
                std::collections::BTreeSet::new();
            let mut total_variants = 0usize;
            let mut total_transforms = 0usize;
            for desc in &file.descriptions {
                for lang in &desc.languages {
                    for v in &lang.variants {
                        total_variants += 1;
                        for t in &v.transforms {
                            total_transforms += 1;
                            if let poe_dat::stat_desc::TransformKind::Other(s) = &t.kind {
                                other_transforms.insert(s.clone());
                            }
                        }
                    }
                }
            }
            println!("Total variants: {total_variants}");
            println!("Total transforms: {total_transforms}");
            if !other_transforms.is_empty() {
                println!("Unknown transforms ({}):", other_transforms.len());
                for t in &other_transforms {
                    println!("  {t}");
                }
            }

            println!("\nFirst 5 descriptions:");
            for desc in file.descriptions.iter().take(5) {
                println!(
                    "  stats: {:?}, langs: {}, variants(first): {}",
                    desc.stat_ids,
                    desc.languages.len(),
                    desc.languages[0].variants.len()
                );
                for v in &desc.languages[0].variants {
                    println!(
                        "    ranges: {:?}, fmt: {:?}, transforms: {:?}",
                        v.ranges, v.format_string, v.transforms
                    );
                }
            }
        }
        Err(e) => {
            // Print enough context to debug
            let msg = e.to_string();
            if msg.len() > 2000 {
                panic!("Parse failed: {}...", &msg[..2000]);
            } else {
                panic!("Parse failed: {msg}");
            }
        }
    }
}
