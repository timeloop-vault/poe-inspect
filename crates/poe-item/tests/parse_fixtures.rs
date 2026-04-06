use poe_item::types::{
    InfluenceKind, ModSlot, ModSource, ModTierKind, Rarity, Section, StatusKind,
};

fn fixture(name: &str) -> String {
    let path = format!("{}/../../fixtures/items/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

fn parse_fixture(name: &str) -> poe_item::types::RawItem {
    let text = fixture(name);
    poe_item::parse(&text).unwrap_or_else(|e| panic!("failed to parse {name}: {e}"))
}

// ─── Header tests ────────────────────────────────────────────────────────────

#[test]
fn rare_weapon_header() {
    let item = parse_fixture("battered-foil-rare-ess-craft.txt");
    assert_eq!(item.header.item_class, "Thrusting One Hand Swords");
    assert_eq!(item.header.rarity, Rarity::Rare);
    assert_eq!(item.header.name1, "Mind Scalpel");
    assert_eq!(item.header.name2.as_deref(), Some("Battered Foil"));
}

#[test]
fn unique_ring_header() {
    let item = parse_fixture("unique-ring-ventors-gamble.txt");
    assert_eq!(item.header.item_class, "Rings");
    assert_eq!(item.header.rarity, Rarity::Unique);
    assert_eq!(item.header.name1, "Ventor's Gamble");
    assert_eq!(item.header.name2.as_deref(), Some("Gold Ring"));
}

#[test]
fn normal_staff_header() {
    let item = parse_fixture("normal-staff-elder.txt");
    assert_eq!(item.header.rarity, Rarity::Normal);
    assert_eq!(item.header.name1, "Imperial Staff");
    // Normal items have only one name line
    assert!(item.header.name2.is_none());
}

#[test]
fn gem_header() {
    let item = parse_fixture("leap-slam.txt");
    assert_eq!(item.header.item_class, "Skill Gems");
    assert_eq!(item.header.rarity, Rarity::Gem);
    assert_eq!(item.header.name1, "Leap Slam");
    assert!(item.header.name2.is_none());
}

#[test]
fn currency_header() {
    let item = parse_fixture("coffin.txt");
    assert_eq!(item.header.item_class, "Stackable Currency");
    assert_eq!(item.header.rarity, Rarity::Currency);
    assert_eq!(item.header.name1, "Filled Coffin");
    assert!(item.header.name2.is_none());
}

// ─── Section identification tests ────────────────────────────────────────────

#[test]
fn rare_weapon_sections() {
    let item = parse_fixture("battered-foil-rare-ess-craft.txt");

    // Expected sections: properties, requirements, sockets, item level,
    //                    implicit mods, explicit mods
    let section_kinds: Vec<&str> = item
        .sections
        .iter()
        .map(|s| match s {
            Section::Requirements(_) => "requirements",
            Section::Sockets(_) => "sockets",
            Section::ItemLevel(_) => "item_level",
            Section::Modifiers(_) => "modifiers",
            Section::Properties { .. } => "properties",
            Section::Generic(_) => "generic",
            _ => "other",
        })
        .collect();

    assert_eq!(
        section_kinds,
        vec![
            "properties",
            "requirements",
            "sockets",
            "item_level",
            "modifiers",
            "modifiers"
        ]
    );
}

#[test]
fn item_level_parsed() {
    let item = parse_fixture("battered-foil-rare-ess-craft.txt");
    let il = item
        .sections
        .iter()
        .find_map(|s| match s {
            Section::ItemLevel(n) => Some(*n),
            _ => None,
        })
        .expect("should have item level");
    assert_eq!(il, 84);
}

#[test]
fn requirements_parsed() {
    let item = parse_fixture("battered-foil-rare-ess-craft.txt");
    let reqs = item
        .sections
        .iter()
        .find_map(|s| match s {
            Section::Requirements(r) => Some(r),
            _ => None,
        })
        .expect("should have requirements");

    assert_eq!(reqs.len(), 4);
    assert_eq!(reqs[0].key, "Level");
    assert_eq!(reqs[0].value, "70");
    assert_eq!(reqs[1].key, "Str");
    assert_eq!(reqs[2].key, "Dex");
    assert_eq!(reqs[3].key, "Int");
}

#[test]
fn sockets_parsed() {
    let item = parse_fixture("battered-foil-rare-ess-craft.txt");
    let sockets = item
        .sections
        .iter()
        .find_map(|s| match s {
            Section::Sockets(s) => Some(s.as_str()),
            _ => None,
        })
        .expect("should have sockets");
    assert_eq!(sockets, "R-R-G");
}

// ─── Modifier header tests ──────────────────────────────────────────────────

#[test]
fn implicit_mod_parsed() {
    let item = parse_fixture("battered-foil-rare-ess-craft.txt");
    let mod_sections: Vec<_> = item
        .sections
        .iter()
        .filter_map(|s| match s {
            Section::Modifiers(m) => Some(m),
            _ => None,
        })
        .collect();

    // First mod section = implicit, second = explicit
    assert_eq!(mod_sections.len(), 2);

    let implicit = &mod_sections[0];
    assert_eq!(implicit.groups.len(), 1);
    assert_eq!(implicit.groups[0].header.slot, ModSlot::Implicit);
    assert_eq!(implicit.groups[0].header.tags, vec!["Damage", "Critical"]);
    assert_eq!(
        implicit.groups[0].body_lines[0],
        "+25% to Global Critical Strike Multiplier (implicit)"
    );
}

#[test]
fn explicit_mods_parsed() {
    let item = parse_fixture("battered-foil-rare-ess-craft.txt");
    let mod_sections: Vec<_> = item
        .sections
        .iter()
        .filter_map(|s| match s {
            Section::Modifiers(m) => Some(m),
            _ => None,
        })
        .collect();

    let explicit = &mod_sections[1];
    assert_eq!(explicit.groups.len(), 5);

    // First mod: prefix "Icy" tier 8
    let icy = &explicit.groups[0];
    assert_eq!(icy.header.slot, ModSlot::Prefix);
    assert_eq!(icy.header.name.as_deref(), Some("Icy"));
    assert_eq!(icy.header.tier, Some(ModTierKind::Tier(8)));
    assert_eq!(
        icy.header.tags,
        vec!["Damage", "Elemental", "Cold", "Attack"]
    );
    assert_eq!(icy.body_lines.len(), 1);

    // Second mod: master crafted prefix
    let vagans = &explicit.groups[1];
    assert_eq!(vagans.header.source, ModSource::MasterCrafted);
    assert_eq!(vagans.header.slot, ModSlot::Prefix);
    assert_eq!(vagans.header.name.as_deref(), Some("Vagan's"));
    assert!(vagans.header.tier.is_none());

    // Fifth mod: suffix "of Acclaim" tier 4
    let acclaim = &explicit.groups[4];
    assert_eq!(acclaim.header.slot, ModSlot::Suffix);
    assert_eq!(acclaim.header.name.as_deref(), Some("of Acclaim"));
    assert_eq!(acclaim.header.tier, Some(ModTierKind::Tier(4)));
}

#[test]
fn multi_line_mod_body() {
    let item = parse_fixture("battered-foil-rare-ess-craft.txt");
    let mod_sections: Vec<_> = item
        .sections
        .iter()
        .filter_map(|s| match s {
            Section::Modifiers(m) => Some(m),
            _ => None,
        })
        .collect();

    // "of Poison" mod has 3 body lines (stat + stat + reminder)
    let poison = &mod_sections[1].groups[3];
    assert_eq!(poison.header.name.as_deref(), Some("of Poison"));
    assert_eq!(poison.body_lines.len(), 3);
    assert!(poison.body_lines[2].starts_with('('));
}

// ─── Influence and status tests ──────────────────────────────────────────────

#[test]
fn elder_influence_standalone() {
    let item = parse_fixture("normal-staff-elder.txt");
    let influence = item
        .sections
        .iter()
        .find_map(|s| match s {
            Section::Influence(i) => Some(i),
            _ => None,
        })
        .expect("should have influence section");
    assert_eq!(influence, &[InfluenceKind::Elder]);
}

#[test]
fn trailing_influence_markers() {
    let item = parse_fixture("rare-boots-eater-exarch.txt");
    // The explicit mod section should have trailing influence markers
    let mod_sections: Vec<_> = item
        .sections
        .iter()
        .filter_map(|s| match s {
            Section::Modifiers(m) => Some(m),
            _ => None,
        })
        .collect();

    // Last mod section has trailing influence markers
    let last = mod_sections.last().expect("should have mod sections");
    assert_eq!(
        last.trailing_influences,
        vec![InfluenceKind::SearingExarch, InfluenceKind::EaterOfWorlds]
    );
}

#[test]
fn corrupted_status() {
    let item = parse_fixture("rare-amulet-talisman-corrupted.txt");
    let status = item
        .sections
        .iter()
        .find_map(|s| match s {
            Section::Status(k) => Some(*k),
            _ => None,
        })
        .expect("should have status");
    assert_eq!(status, StatusKind::Corrupted);
}

#[test]
fn synthesised_influence() {
    let item = parse_fixture("rare-ring-synthesised.txt");
    let influence = item
        .sections
        .iter()
        .find_map(|s| match s {
            Section::Influence(i) => Some(i),
            _ => None,
        })
        .expect("should have influence section");
    assert_eq!(influence, &[InfluenceKind::Synthesised]);
}

#[test]
fn fractured_influence() {
    let item = parse_fixture("rare-axe-fractured.txt");
    let influence = item
        .sections
        .iter()
        .find_map(|s| match s {
            Section::Influence(i) => Some(i),
            _ => None,
        })
        .expect("should have influence section");
    assert_eq!(influence, &[InfluenceKind::Fractured]);
}

// ─── Influence implicit mods (Exarch/Eater) ─────────────────────────────────

#[test]
fn exarch_eater_implicit_mods() {
    let item = parse_fixture("rare-boots-eater-exarch.txt");
    let mod_sections: Vec<_> = item
        .sections
        .iter()
        .filter_map(|s| match s {
            Section::Modifiers(m) => Some(m),
            _ => None,
        })
        .collect();

    // First mod section: influence implicit mods
    let influence_implicits = &mod_sections[0];
    assert_eq!(influence_implicits.groups.len(), 2);

    let exarch = &influence_implicits.groups[0];
    assert_eq!(exarch.header.slot, ModSlot::SearingExarchImplicit);
    assert_eq!(exarch.header.influence_tier.as_deref(), Some("Greater"));

    let eater = &influence_implicits.groups[1];
    assert_eq!(eater.header.slot, ModSlot::EaterOfWorldsImplicit);
    assert_eq!(eater.header.influence_tier.as_deref(), Some("Greater"));
    assert_eq!(eater.header.tags, vec!["Life"]);
}

// ─── Unique modifier tests ──────────────────────────────────────────────────

#[test]
fn unique_mods() {
    let item = parse_fixture("unique-ring-ventors-gamble.txt");
    let mod_sections: Vec<_> = item
        .sections
        .iter()
        .filter_map(|s| match s {
            Section::Modifiers(m) => Some(m),
            _ => None,
        })
        .collect();

    // Implicit section + explicit unique mods section
    assert!(mod_sections.len() >= 2);

    let unique_mods = &mod_sections[1];
    for group in &unique_mods.groups {
        assert_eq!(group.header.slot, ModSlot::Unique);
    }
    assert_eq!(unique_mods.groups.len(), 6);
}

// ─── Master crafted mods ────────────────────────────────────────────────────

#[test]
fn master_crafted_suffix() {
    let item = parse_fixture("rare-boots-eater-exarch.txt");
    let mod_sections: Vec<_> = item
        .sections
        .iter()
        .filter_map(|s| match s {
            Section::Modifiers(m) => Some(m),
            _ => None,
        })
        .collect();

    let explicit = mod_sections.last().expect("should have explicit mods");
    let crafted = explicit
        .groups
        .iter()
        .find(|g| g.header.source == ModSource::MasterCrafted)
        .expect("should have master crafted mod");

    assert_eq!(crafted.header.slot, ModSlot::Suffix);
    assert_eq!(crafted.header.name.as_deref(), Some("of Craft"));
    assert_eq!(crafted.header.tier, Some(ModTierKind::Rank(2)));
}

// ─── Map tests ──────────────────────────────────────────────────────────────

#[test]
fn normal_map_parses() {
    let item = parse_fixture("normal-map-alleyways.txt");
    assert_eq!(item.header.rarity, Rarity::Normal);
    assert_eq!(item.header.name1, "Alleyways Map");

    let monster_level = item
        .sections
        .iter()
        .find_map(|s| match s {
            Section::MonsterLevel(n) => Some(*n),
            _ => None,
        })
        .expect("map should have monster level");
    assert_eq!(monster_level, 68);
}

#[test]
fn rare_map_with_mods() {
    let item = parse_fixture("rare-map-abomination-t17.txt");
    assert_eq!(item.header.rarity, Rarity::Rare);

    let mod_sections: Vec<_> = item
        .sections
        .iter()
        .filter_map(|s| match s {
            Section::Modifiers(m) => Some(m),
            _ => None,
        })
        .collect();

    assert_eq!(mod_sections.len(), 1);
    assert_eq!(mod_sections[0].groups.len(), 6); // 3 prefix + 3 suffix
}

// ─── Enchant/talisman tests ─────────────────────────────────────────────────

#[test]
fn talisman_tier() {
    let item = parse_fixture("rare-amulet-talisman-corrupted.txt");
    let tier = item
        .sections
        .iter()
        .find_map(|s| match s {
            Section::TalismanTier(n) => Some(*n),
            _ => None,
        })
        .expect("should have talisman tier");
    assert_eq!(tier, 3);
}

#[test]
fn enchant_section_detected() {
    let item = parse_fixture("rare-amulet-talisman-corrupted.txt");
    // "Allocates Entropy (enchant)" should be in an Enchants section
    let has_enchant = item.sections.iter().any(|s| match s {
        Section::Enchants(lines) => lines.iter().any(|l| l.contains("(enchant)")),
        _ => false,
    });
    assert!(has_enchant, "enchant should be in Enchants section");
}

#[test]
fn body_armour_mixed_enchant_section() {
    let item = parse_fixture("rare-body-armour-enchanted.txt");
    // Mixed section (some lines without "(enchant)") falls through to Generic.
    // Only pure enchant sections are caught by the grammar.
    let has_enchant_generic = item.sections.iter().any(|s| match s {
        Section::Generic(lines) => lines.iter().any(|l| l.contains("(enchant)")),
        _ => false,
    });
    assert!(
        has_enchant_generic,
        "mixed enchant section should stay generic"
    );
}

// ─── Gem test ───────────────────────────────────────────────────────────────

#[test]
fn gem_sections() {
    let item = parse_fixture("leap-slam.txt");
    let section_kinds: Vec<&str> = item
        .sections
        .iter()
        .map(|s| match s {
            Section::Requirements(_) => "requirements",
            Section::Experience(_) => "experience",
            Section::Properties { .. } => "properties",
            Section::Generic(_) => "generic",
            _ => "other",
        })
        .collect();

    // Gem tags+props (sub-header + properties), requirements, description, stats, experience, usage
    assert_eq!(
        section_kinds,
        vec![
            "properties",
            "requirements",
            "generic",
            "generic",
            "experience",
            "generic"
        ]
    );
}

// ─── Flavor text ────────────────────────────────────────────────────────────

#[test]
fn flavor_text_as_generic() {
    let item = parse_fixture("unique-quiver-soul-strike.txt");
    let flavor = item.sections.iter().find(|s| match s {
        Section::Generic(lines) => lines.iter().any(|l| l.contains("chaotic world")),
        _ => false,
    });
    assert!(flavor.is_some(), "flavor text should be in generic section");
}

// ─── New fixture tests: magic items, flasks, jewels, div cards ──────────────

#[test]
fn divination_card_header() {
    let item = parse_fixture("divination-card-hunters-resolve.txt");
    assert_eq!(item.header.item_class, "Divination Cards");
    assert_eq!(item.header.rarity, Rarity::DivinationCard);
    assert_eq!(item.header.name1, "Hunter's Resolve");
    assert!(item.header.name2.is_none());
    // Stack size is a property section, reward hint and flavor text are generic
    assert!(
        item.sections
            .iter()
            .all(|s| matches!(s, Section::Generic(_) | Section::Properties { .. }))
    );
}

#[test]
fn magic_item_single_name_line() {
    let item = parse_fixture("magic-axe-two-handed.txt");
    assert_eq!(item.header.rarity, Rarity::Magic);
    // Magic items: single name line with base type embedded
    assert_eq!(item.header.name1, "Smouldering Foul Staff");
    assert!(item.header.name2.is_none());
}

#[test]
fn magic_item_has_mods() {
    let item = parse_fixture("magic-axe-two-handed.txt");
    let mod_sections: Vec<_> = item
        .sections
        .iter()
        .filter_map(|s| match s {
            Section::Modifiers(m) => Some(m),
            _ => None,
        })
        .collect();

    // Implicit + explicit mod sections
    assert_eq!(mod_sections.len(), 2);
    assert_eq!(mod_sections[0].groups[0].header.slot, ModSlot::Implicit);
    assert_eq!(mod_sections[1].groups[0].header.slot, ModSlot::Prefix);
    assert_eq!(
        mod_sections[1].groups[0].header.name.as_deref(),
        Some("Smouldering")
    );
}

#[test]
fn magic_flask_properties_and_mods() {
    let item = parse_fixture("magic-flask-life.txt");
    assert_eq!(item.header.rarity, Rarity::Magic);
    assert_eq!(item.header.item_class, "Life Flasks");

    // Flask properties section (Recovers, Consumes, Currently has) is generic
    let has_flask_props = item.sections.iter().any(|s| match s {
        Section::Generic(lines) => lines.iter().any(|l| l.starts_with("Recovers")),
        _ => false,
    });
    assert!(
        has_flask_props,
        "flask properties should be in generic section"
    );

    // Mods are properly parsed with headers
    let mod_sections: Vec<_> = item
        .sections
        .iter()
        .filter_map(|s| match s {
            Section::Modifiers(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(mod_sections.len(), 1);
    assert_eq!(mod_sections[0].groups.len(), 2); // prefix + suffix
}

#[test]
fn normal_flask_no_mods() {
    let item = parse_fixture("magic-flask-utility.txt");
    assert_eq!(item.header.rarity, Rarity::Normal);
    assert_eq!(item.header.name1, "Quicksilver Flask");
    // Normal flask has no mod section
    let has_mods = item
        .sections
        .iter()
        .any(|s| matches!(s, Section::Modifiers(_)));
    assert!(!has_mods);
}

#[test]
fn magic_jewel_parsed() {
    let item = parse_fixture("magic-jewel-cobalt.txt");
    assert_eq!(item.header.rarity, Rarity::Magic);
    assert_eq!(item.header.item_class, "Jewels");

    let mod_sections: Vec<_> = item
        .sections
        .iter()
        .filter_map(|s| match s {
            Section::Modifiers(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(mod_sections.len(), 1);
    assert_eq!(mod_sections[0].groups.len(), 2); // prefix + suffix
}

#[test]
fn cluster_jewel_enchants_and_mods() {
    let item = parse_fixture("magic-cluster-jewel-large.txt");
    assert_eq!(item.header.item_class, "Jewels");

    // Enchant section detected by grammar
    let enchant_section = item.sections.iter().find(|s| match s {
        Section::Enchants(lines) => lines.iter().any(|l| l.contains("(enchant)")),
        _ => false,
    });
    assert!(
        enchant_section.is_some(),
        "cluster jewel enchants should be in Enchants section"
    );

    // Mods are properly parsed
    let mod_sections: Vec<_> = item
        .sections
        .iter()
        .filter_map(|s| match s {
            Section::Modifiers(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(mod_sections.len(), 1);
    assert_eq!(mod_sections[0].groups.len(), 2); // prefix + suffix
}

#[test]
fn normal_cluster_jewel_enchants_only() {
    let item = parse_fixture("magic-cluster-jewel-normal.txt");
    assert_eq!(item.header.rarity, Rarity::Normal);
    // Normal cluster jewel has enchants but no mods
    let has_mods = item
        .sections
        .iter()
        .any(|s| matches!(s, Section::Modifiers(_)));
    assert!(!has_mods);
    let has_enchants = item
        .sections
        .iter()
        .any(|s| matches!(s, Section::Enchants(_)));
    assert!(has_enchants);
}

// ─── Support gem ────────────────────────────────────────────────────────────

#[test]
fn support_gem_parsed() {
    let item = parse_fixture("gem-support-faster-casting.txt");
    assert_eq!(item.header.item_class, "Support Gems");
    assert_eq!(item.header.rarity, Rarity::Gem);
    assert_eq!(item.header.name1, "Faster Casting Support");

    let section_kinds: Vec<&str> = item
        .sections
        .iter()
        .map(|s| match s {
            Section::Requirements(_) => "requirements",
            Section::Experience(_) => "experience",
            Section::Properties { .. } => "properties",
            Section::Generic(_) => "generic",
            _ => "other",
        })
        .collect();
    // tags+props (sub-header + properties), requirements, description, stats, experience, usage
    assert_eq!(
        section_kinds,
        vec![
            "properties",
            "requirements",
            "generic",
            "generic",
            "experience",
            "generic"
        ]
    );
}

// ─── Transfigured gem ───────────────────────────────────────────────────────

#[test]
fn transfigured_gem_status() {
    let item = parse_fixture("gem-skill-transfigured-consecrated-path-of-endurance.txt");
    assert_eq!(item.header.rarity, Rarity::Gem);
    assert_eq!(item.header.name1, "Consecrated Path of Endurance");

    let status = item
        .sections
        .iter()
        .find_map(|s| match s {
            Section::Status(k) => Some(*k),
            _ => None,
        })
        .expect("transfigured gem should have status");
    assert_eq!(status, StatusKind::Transfigured);
}

#[test]
fn transfigured_gem_corrupted_imbued() {
    let item = parse_fixture("gem-skill-transfigured-shock-nova-of-procession.txt");
    assert_eq!(item.header.rarity, Rarity::Gem);
    assert_eq!(item.header.name1, "Shock Nova of Procession");

    let statuses: Vec<StatusKind> = item
        .sections
        .iter()
        .filter_map(|s| match s {
            Section::Status(k) => Some(*k),
            _ => None,
        })
        .collect();
    assert!(statuses.contains(&StatusKind::Corrupted));
    assert!(statuses.contains(&StatusKind::Transfigured));
}

// ─── Belt, helmet, shield ───────────────────────────────────────────────────

#[test]
fn rare_belt_parsed() {
    let item = parse_fixture("rare-belt-crafted.txt");
    assert_eq!(item.header.item_class, "Belts");
    assert_eq!(item.header.name2.as_deref(), Some("Leather Belt"));
    // Implicit + explicit mods
    let mod_count: usize = item
        .sections
        .iter()
        .filter_map(|s| match s {
            Section::Modifiers(m) => Some(m.groups.len()),
            _ => None,
        })
        .sum();
    assert_eq!(mod_count, 6); // 1 implicit + 5 explicit
}

#[test]
fn rare_helmet_parsed() {
    let item = parse_fixture("rare-helmet-crafted.txt");
    assert_eq!(item.header.item_class, "Helmets");
    assert_eq!(item.header.name2.as_deref(), Some("Tribal Circlet"));
}

#[test]
fn rare_shield_parsed() {
    let item = parse_fixture("rare-shield-crafted.txt");
    assert_eq!(item.header.item_class, "Shields");
    assert_eq!(item.header.name2.as_deref(), Some("Mahogany Tower Shield"));
    // Shield has Chance to Block in properties section
    let has_block = item.sections.iter().any(|s| match s {
        Section::Properties { lines, .. } => lines.iter().any(|l| l.key == "Chance to Block"),
        _ => false,
    });
    assert!(has_block);
}

// ─── Unique weapon + unique flask ───────────────────────────────────────────

#[test]
fn unique_bow_parsed() {
    let item = parse_fixture("unique-bow-short-bow.txt");
    assert_eq!(item.header.item_class, "Bows");
    assert_eq!(item.header.rarity, Rarity::Unique);
    assert_eq!(item.header.name1, "Quill Rain");
    assert_eq!(item.header.name2.as_deref(), Some("Short Bow"));

    // 7 unique mods
    let mod_sections: Vec<_> = item
        .sections
        .iter()
        .filter_map(|s| match s {
            Section::Modifiers(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(mod_sections.len(), 1);
    assert_eq!(mod_sections[0].groups.len(), 7);

    // Flavor text
    let has_flavor = item.sections.iter().any(|s| match s {
        Section::Generic(lines) => lines.iter().any(|l| l.contains("Rigwald")),
        _ => false,
    });
    assert!(has_flavor);
}

#[test]
fn unique_flask_parsed() {
    let item = parse_fixture("unique-flask-doedres-flask.txt");
    assert_eq!(item.header.item_class, "Mana Flasks");
    assert_eq!(item.header.rarity, Rarity::Unique);
    assert_eq!(item.header.name1, "Doedre's Elixir");

    // Flask properties (generic) + unique mods + flavor text (generic) + usage (generic)
    let has_flask_props = item.sections.iter().any(|s| match s {
        Section::Generic(lines) => lines.iter().any(|l| l.starts_with("Recovers")),
        _ => false,
    });
    assert!(has_flask_props);

    let mod_sections: Vec<_> = item
        .sections
        .iter()
        .filter_map(|s| match s {
            Section::Modifiers(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(mod_sections.len(), 1);
    assert_eq!(mod_sections[0].groups.len(), 6);

    let has_flavor = item.sections.iter().any(|s| match s {
        Section::Generic(lines) => lines.iter().any(|l| l.contains("Doedre Darktongue")),
        _ => false,
    });
    assert!(has_flavor);
}

// ─── Unidentified status ────────────────────────────────────────────────────

#[test]
fn unidentified_status() {
    let item = parse_fixture("rare-axe-unidentified.txt");
    let status = item
        .sections
        .iter()
        .find_map(|s| match s {
            Section::Status(k) => Some(*k),
            _ => None,
        })
        .expect("should have Unidentified status");
    assert_eq!(status, StatusKind::Unidentified);
}

// ─── Note (trade pricing) ──────────────────────────────────────────────────

#[test]
fn note_section_parsed() {
    let item = parse_fixture("rare-jewel-cobalt-mirrored-corrupted.txt");
    let note = item
        .sections
        .iter()
        .find_map(|s| match s {
            Section::Note(n) => Some(n.as_str()),
            _ => None,
        })
        .expect("should have Note section");
    assert_eq!(note, "~b/o 35 chaos");
}

#[test]
fn mirrored_and_corrupted_statuses() {
    let item = parse_fixture("rare-jewel-cobalt-mirrored-corrupted.txt");
    let statuses: Vec<_> = item
        .sections
        .iter()
        .filter_map(|s| match s {
            Section::Status(k) => Some(*k),
            _ => None,
        })
        .collect();
    assert_eq!(statuses, vec![StatusKind::Mirrored, StatusKind::Corrupted]);
}

// ─── Divination Card rarity ────────────────────────────────────────────────

#[test]
fn divination_card_rarity() {
    let item = parse_fixture("divination-card-emperors-luck.txt");
    assert_eq!(item.header.rarity, Rarity::DivinationCard);
}

// ─── Comprehensive: all .txt fixtures parse without error ───────────────────

#[test]
fn all_fixtures_parse() {
    let dir = format!("{}/../../fixtures/items", env!("CARGO_MANIFEST_DIR"));
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).expect("can't read fixtures/items dir") {
        let entry = entry.expect("can't read dir entry");
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "txt") {
            let name = path.file_name().unwrap().to_string_lossy();
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("failed to read {name}: {e}"));
            let result = poe_item::parse(&text);
            assert!(
                result.is_ok(),
                "fixture {name} failed to parse: {}",
                result.unwrap_err()
            );
            count += 1;
        }
    }
    assert!(count >= 41, "expected at least 41 fixtures, found {count}");
}

// ─── Magic sceptre with Memory Strands property ────────────────────────────

#[test]
fn magic_sceptre_parsed() {
    let item = parse_fixture("magic-sceptre-opal.txt");
    assert_eq!(item.header.item_class, "Sceptres");
    assert_eq!(item.header.rarity, Rarity::Magic);
    assert_eq!(item.header.name1, "Frozen Opal Sceptre of Discharge");
    assert!(item.header.name2.is_none());

    // Properties section includes Memory Strands
    let has_memory_strands = item.sections.iter().any(|s| match s {
        Section::Properties { lines, .. } => lines.iter().any(|l| l.key == "Memory Strands"),
        _ => false,
    });
    assert!(has_memory_strands, "should have Memory Strands property");

    // Implicit + explicit mods
    let mod_sections: Vec<_> = item
        .sections
        .iter()
        .filter_map(|s| match s {
            Section::Modifiers(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(mod_sections.len(), 2);
    assert_eq!(mod_sections[0].groups[0].header.slot, ModSlot::Implicit);
    assert_eq!(mod_sections[1].groups.len(), 2); // prefix + suffix
}

// ─── Corruption implicit mods ──────────────────────────────────────────────

#[test]
fn corruption_implicit_mods() {
    let item = parse_fixture("rare-jewel-viridian-corrupted-implicits.txt");
    assert_eq!(item.header.item_class, "Jewels");
    assert_eq!(item.header.rarity, Rarity::Rare);

    let mod_sections: Vec<_> = item
        .sections
        .iter()
        .filter_map(|s| match s {
            Section::Modifiers(m) => Some(m),
            _ => None,
        })
        .collect();

    // Corruption implicits + explicits
    assert_eq!(mod_sections.len(), 2);

    // First section: corruption implicits
    let corruption = &mod_sections[0];
    assert_eq!(corruption.groups.len(), 2);
    assert_eq!(
        corruption.groups[0].header.slot,
        ModSlot::CorruptionImplicit
    );
    assert_eq!(corruption.groups[0].header.tags, vec!["Damage"]);
    assert_eq!(
        corruption.groups[1].header.slot,
        ModSlot::CorruptionImplicit
    );
    assert_eq!(corruption.groups[1].header.tags, vec!["Curse"]);

    // Second section: explicits
    let explicits = &mod_sections[1];
    assert_eq!(explicits.groups.len(), 3); // 2 prefix + 1 suffix

    // Corrupted status
    let status = item
        .sections
        .iter()
        .find_map(|s| match s {
            Section::Status(k) => Some(*k),
            _ => None,
        })
        .expect("should have Corrupted status");
    assert_eq!(status, StatusKind::Corrupted);
}

// ─── Anointed talisman ─────────────────────────────────────────────────────

#[test]
fn anointed_talisman_parsed() {
    let item = parse_fixture("rare-talisman-anointed-corrupted.txt");
    assert_eq!(item.header.item_class, "Amulets");
    assert_eq!(item.header.rarity, Rarity::Rare);
    assert_eq!(item.header.name2.as_deref(), Some("Ashscale Talisman"));

    // Talisman tier
    let tier = item
        .sections
        .iter()
        .find_map(|s| match s {
            Section::TalismanTier(n) => Some(*n),
            _ => None,
        })
        .expect("should have talisman tier");
    assert_eq!(tier, 1);

    // Anointment enchant in Enchants section (detected by grammar)
    let has_anoint = item.sections.iter().any(|s| match s {
        Section::Enchants(lines) => lines
            .iter()
            .any(|l| l.contains("Allocates Devotion (enchant)")),
        _ => false,
    });
    assert!(has_anoint, "should have anointment enchant");

    // Flavor text in generic section
    let has_flavor = item.sections.iter().any(|s| match s {
        Section::Generic(lines) => lines.iter().any(|l| l.contains("Wolven King")),
        _ => false,
    });
    assert!(has_flavor, "should have flavor text");

    // Implicit + explicits
    let mod_sections: Vec<_> = item
        .sections
        .iter()
        .filter_map(|s| match s {
            Section::Modifiers(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(mod_sections.len(), 2);
    assert_eq!(mod_sections[0].groups[0].header.slot, ModSlot::Implicit);
    assert_eq!(mod_sections[1].groups.len(), 4); // 1 prefix + 3 suffix

    // Corrupted
    let has_corrupted = item
        .sections
        .iter()
        .any(|s| matches!(s, Section::Status(StatusKind::Corrupted)));
    assert!(has_corrupted, "should have Corrupted status");
}

// ─── Heist items ────────────────────────────────────────────────────────────

#[test]
fn contract_magic_parses() {
    let item = parse_fixture("contract-magic-mansion.txt");
    assert_eq!(item.header.item_class, "Contracts");
    assert_eq!(item.header.rarity, Rarity::Magic);
    assert_eq!(
        item.header.name1,
        "Armoured Contract: Mansion of Congealment"
    );

    // Should have generic sections (properties, enchants, flavor, usage), mod section, item level
    let generic_count = item
        .sections
        .iter()
        .filter(|s| matches!(s, Section::Generic(_)))
        .count();
    assert!(
        generic_count >= 3,
        "should have at least 3 generic sections, got {generic_count}"
    );
}

#[test]
fn blueprint_normal_parses() {
    let item = parse_fixture("blueprint-normal-bunker.txt");
    assert_eq!(item.header.item_class, "Blueprints");
    assert_eq!(item.header.rarity, Rarity::Normal);
    assert_eq!(item.header.name1, "Blueprint: Bunker");
}

#[test]
fn blueprint_magic_parses() {
    let item = parse_fixture("blueprint-magic-records-office.txt");
    assert_eq!(item.header.item_class, "Blueprints");
    assert_eq!(item.header.rarity, Rarity::Magic);
    assert_eq!(item.header.name1, "Hexwarded Blueprint: Records Office");
}
