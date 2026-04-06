use pest::Parser;
use pest_derive::Parser;

use crate::types::{
    GemData, Header, InfluenceKind, ItemProperty, ModGroup, ModHeader, ModSection, ModSlot,
    ModSource, ModTierKind, Rarity, RawItem, RawPropertyLine, Requirement, Section, StatusKind,
    VaalGemData,
};

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct ItemParser;

/// Parse Ctrl+Alt+C item text into a [`RawItem`].
///
/// This is Pass 1 only (structural parse). The result contains raw strings
/// that have not been resolved against game data.
///
/// # Errors
///
/// Returns [`ParseError`] if the input doesn't match the expected item format.
pub fn parse(input: &str) -> Result<RawItem, ParseError> {
    // Ensure input ends with newline (grammar expects it).
    let input = if input.ends_with('\n') {
        input.to_string()
    } else {
        format!("{input}\n")
    };

    let pairs = ItemParser::parse(Rule::item, &input).map_err(|e| ParseError::Grammar {
        message: e.to_string(),
    })?;

    let item_pair = pairs
        .into_iter()
        .next()
        .ok_or_else(|| ParseError::Internal("no item rule matched".into()))?;

    walk_item(item_pair)
}

fn walk_item(pair: pest::iterators::Pair<'_, Rule>) -> Result<RawItem, ParseError> {
    // item = { SOI ~ (gem_item | standard_item) ~ NEWLINE* ~ EOI }
    // Unwrap the gem_item or standard_item wrapper, then walk its children.
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::gem_item | Rule::standard_item => return walk_item_inner(inner),
            _ => {}
        }
    }
    Err(ParseError::Internal("no item variant matched".into()))
}

fn walk_item_inner(pair: pest::iterators::Pair<'_, Rule>) -> Result<RawItem, ParseError> {
    let mut header = None;
    let mut sections = Vec::new();
    let mut gem_data: Option<GemData> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            // Both gem_header_section and header_section have the same child structure
            Rule::header_section | Rule::gem_header_section => {
                header = Some(walk_header(inner));
            }
            Rule::section => {
                let section_inner = inner.into_inner().next().unwrap();
                sections.push(walk_section(section_inner)?);
            }
            // ── Gem-specific rules (build GemData incrementally) ──
            Rule::gem_tags_and_properties => {
                let (tags, props) = walk_gem_tags_and_properties(inner);
                gem_data.get_or_insert_with(GemData::default).tags = tags;
                sections.push(Section::Properties {
                    subheader: None,
                    lines: props,
                });
            }
            Rule::gem_description_section => {
                let lines = collect_section_text(inner);
                gem_data.get_or_insert_with(GemData::default).description = Some(lines.join("\n"));
            }
            Rule::gem_stats_section => {
                let (stats, quality_stats) = walk_gem_stats(inner);
                let gd = gem_data.get_or_insert_with(GemData::default);
                gd.stats = stats;
                gd.quality_stats = quality_stats;
            }
            Rule::vaal_section => {
                let vaal = walk_vaal_section(inner);
                gem_data.get_or_insert_with(GemData::default).vaal = Some(Box::new(vaal));
            }
            // ── Shared rules (used by both gem and standard paths) ──
            // Note: gem_usage_section, gem_supported_by_section, and gem_flavor_section
            // are intentionally not matched — they're dropped or not used downstream yet.
            Rule::property_section => sections.push(walk_property_section(inner)),
            Rule::requirements_section => sections.push(walk_requirements(inner)),
            Rule::experience_section => sections.push(walk_experience(inner)),
            Rule::status_section => sections.push(walk_status_section(inner)?),
            _ => {}
        }
    }

    // If the gem grammar path built GemData, push it as a single section
    if let Some(gd) = gem_data {
        sections.push(Section::GemData(gd));
    }

    Ok(RawItem {
        header: header.ok_or_else(|| ParseError::Internal("missing header section".into()))?,
        sections,
    })
}

fn walk_section(pair: pest::iterators::Pair<'_, Rule>) -> Result<Section, ParseError> {
    match pair.as_rule() {
        Rule::requirements_section => Ok(walk_requirements(pair)),
        Rule::sockets_section => Ok(walk_sockets(pair)),
        Rule::item_level_section => walk_item_level(pair),
        Rule::monster_level_section => walk_monster_level(pair),
        Rule::talisman_tier_section => walk_talisman_tier(pair),
        Rule::experience_section => Ok(walk_experience(pair)),
        Rule::mod_section => walk_mod_section(pair),
        Rule::influence_section => Ok(walk_influence_section(pair)),
        Rule::status_section => walk_status_section(pair),
        Rule::note_section => Ok(walk_note_section(pair)),
        Rule::enchant_section => Ok(walk_enchant_section(pair)),
        Rule::property_section => Ok(walk_property_section(pair)),
        _ => Ok(walk_generic_section(pair)),
    }
}

fn walk_header(pair: pest::iterators::Pair<'_, Rule>) -> Header {
    let mut item_class = String::new();
    let mut rarity = Rarity::Unknown;
    let mut names: Vec<String> = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::item_class_line => {
                for field in inner.into_inner() {
                    if field.as_rule() == Rule::item_class_value {
                        item_class = field.as_str().to_string();
                    }
                }
            }
            Rule::gem_rarity_line => {
                rarity = Rarity::Gem;
            }
            Rule::rarity_line => {
                for field in inner.into_inner() {
                    if field.as_rule() == Rule::rarity_value {
                        rarity = Rarity::from(field.as_str());
                    }
                }
            }
            Rule::header_name_line => {
                let text = inner
                    .into_inner()
                    .find(|p| p.as_rule() == Rule::rest_of_line)
                    .map(|p| p.as_str().to_string())
                    .unwrap_or_default();
                names.push(text);
            }
            _ => {}
        }
    }

    Header {
        item_class,
        rarity,
        name1: names.first().cloned().unwrap_or_default(),
        name2: names.get(1).cloned(),
    }
}

fn walk_requirements(pair: pest::iterators::Pair<'_, Rule>) -> Section {
    let mut reqs = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::requirement_line {
            let mut key = String::new();
            let mut value = String::new();
            for field in inner.into_inner() {
                match field.as_rule() {
                    Rule::requirement_key => key = field.as_str().to_string(),
                    Rule::rest_of_line => value = field.as_str().to_string(),
                    _ => {}
                }
            }
            reqs.push(Requirement { key, value });
        }
    }
    Section::Requirements(reqs)
}

fn walk_sockets(pair: pest::iterators::Pair<'_, Rule>) -> Section {
    let text = pair
        .into_inner()
        .find(|p| p.as_rule() == Rule::rest_of_line)
        .map(|p| p.as_str().to_string())
        .unwrap_or_default();
    Section::Sockets(text)
}

fn walk_item_level(pair: pest::iterators::Pair<'_, Rule>) -> Result<Section, ParseError> {
    let n = extract_integer(pair)?;
    Ok(Section::ItemLevel(n))
}

fn walk_monster_level(pair: pest::iterators::Pair<'_, Rule>) -> Result<Section, ParseError> {
    let n = extract_integer(pair)?;
    Ok(Section::MonsterLevel(n))
}

fn walk_talisman_tier(pair: pest::iterators::Pair<'_, Rule>) -> Result<Section, ParseError> {
    let n = extract_integer(pair)?;
    Ok(Section::TalismanTier(n))
}

fn walk_experience(pair: pest::iterators::Pair<'_, Rule>) -> Section {
    let text = pair
        .into_inner()
        .find(|p| p.as_rule() == Rule::rest_of_line)
        .map(|p| p.as_str().to_string())
        .unwrap_or_default();
    Section::Experience(text)
}

fn walk_mod_section(pair: pest::iterators::Pair<'_, Rule>) -> Result<Section, ParseError> {
    let mut groups = Vec::new();
    let mut trailing_influences = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::mod_group => groups.push(walk_mod_group(inner)?),
            Rule::trailing_marker => {
                for marker_inner in inner.into_inner() {
                    if marker_inner.as_rule() == Rule::influence_keyword {
                        if let Some(kind) = InfluenceKind::parse(marker_inner.as_str()) {
                            trailing_influences.push(kind);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(Section::Modifiers(ModSection {
        groups,
        trailing_influences,
    }))
}

fn walk_mod_group(pair: pest::iterators::Pair<'_, Rule>) -> Result<ModGroup, ParseError> {
    let mut header = None;
    let mut body_lines = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::mod_header => header = Some(walk_mod_header(inner)),
            Rule::mod_body_line => {
                let text = inner
                    .into_inner()
                    .find(|p| p.as_rule() == Rule::rest_of_line)
                    .map(|p| p.as_str().to_string())
                    .unwrap_or_default();
                body_lines.push(text);
            }
            _ => {}
        }
    }

    Ok(ModGroup {
        header: header.ok_or_else(|| ParseError::Internal("mod group missing header".into()))?,
        body_lines,
    })
}

fn walk_mod_header(pair: pest::iterators::Pair<'_, Rule>) -> ModHeader {
    let mut source = ModSource::Regular;
    let mut slot = ModSlot::Prefix;
    let mut influence_tier = None;
    let mut name = None;
    let mut tier = None;
    let mut tags = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::mod_source => {
                source = match inner.as_str().trim() {
                    "Fractured" => ModSource::Fractured,
                    "Master Crafted" => ModSource::MasterCrafted,
                    // League mechanic prefixes (e.g., "Foulborn") are cosmetic —
                    // the mod is still a regular source mod.
                    _ => ModSource::Regular,
                };
            }
            Rule::mod_slot => {
                slot = match inner.as_str() {
                    "Implicit" => ModSlot::Implicit,
                    "Suffix" => ModSlot::Suffix,
                    "Unique" => ModSlot::Unique,
                    "Searing Exarch Implicit" => ModSlot::SearingExarchImplicit,
                    "Eater of Worlds Implicit" => ModSlot::EaterOfWorldsImplicit,
                    "Corruption Implicit" => ModSlot::CorruptionImplicit,
                    // "Prefix" and any unknown slot
                    _ => ModSlot::Prefix,
                };
            }
            Rule::mod_influence_tier => {
                influence_tier = inner
                    .into_inner()
                    .find(|p| p.as_rule() == Rule::mod_influence_tier_value)
                    .map(|p| p.as_str().to_string());
            }
            Rule::mod_name => {
                name = inner
                    .into_inner()
                    .find(|p| p.as_rule() == Rule::mod_name_inner)
                    .map(|p| p.as_str().to_string());
            }
            Rule::mod_tier => {
                // mod_tier contains either "Tier: N" or "Rank: N"
                let text = inner.as_str();
                if let Some(n) = inner
                    .into_inner()
                    .find(|p| p.as_rule() == Rule::integer)
                    .and_then(|p| p.as_str().parse::<u32>().ok())
                {
                    tier = if text.contains("Rank") {
                        Some(ModTierKind::Rank(n))
                    } else {
                        Some(ModTierKind::Tier(n))
                    };
                }
            }
            Rule::mod_tags => {
                for tag_inner in inner.into_inner() {
                    if tag_inner.as_rule() == Rule::tag_list {
                        for tag in tag_inner.into_inner() {
                            if tag.as_rule() == Rule::tag {
                                tags.push(tag.as_str().to_string());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    ModHeader {
        source,
        slot,
        influence_tier,
        name,
        tier,
        tags,
    }
}

fn walk_influence_section(pair: pest::iterators::Pair<'_, Rule>) -> Section {
    let mut influences = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::influence_keyword {
            if let Some(kind) = InfluenceKind::parse(inner.as_str()) {
                influences.push(kind);
            }
        }
    }
    Section::Influence(influences)
}

fn walk_status_section(pair: pest::iterators::Pair<'_, Rule>) -> Result<Section, ParseError> {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::status_keyword {
            if let Some(kind) = StatusKind::parse(inner.as_str()) {
                return Ok(Section::Status(kind));
            }
        }
    }
    Err(ParseError::Internal(
        "status section with no recognized keyword".into(),
    ))
}

fn walk_note_section(pair: pest::iterators::Pair<'_, Rule>) -> Section {
    let text = pair
        .into_inner()
        .find(|p| p.as_rule() == Rule::rest_of_line)
        .map(|p| p.as_str().to_string())
        .unwrap_or_default();
    Section::Note(text)
}

fn walk_generic_section(pair: pest::iterators::Pair<'_, Rule>) -> Section {
    let mut lines = Vec::new();
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::generic_line => {
                let text = inner
                    .into_inner()
                    .find(|p| p.as_rule() == Rule::rest_of_line)
                    .map(|p| p.as_str().to_string())
                    .unwrap_or_default();
                lines.push(text);
            }
            Rule::blank_line => lines.push(String::new()),
            _ => {}
        }
    }
    Section::Generic(lines)
}

fn walk_enchant_section(pair: pest::iterators::Pair<'_, Rule>) -> Section {
    let mut lines = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::enchant_line {
            // Reconstruct the full line including the " (enchant)" suffix,
            // since downstream stat resolution needs it for suffix stripping.
            let text = inner.as_str().trim_end_matches('\n').to_string();
            lines.push(text);
        }
    }
    Section::Enchants(lines)
}

fn walk_property_section(pair: pest::iterators::Pair<'_, Rule>) -> Section {
    let mut subheader = None;
    let mut lines = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::property_subheader => {
                let text = inner
                    .into_inner()
                    .find(|p| p.as_rule() == Rule::rest_of_line)
                    .map(|p| p.as_str().to_string())
                    .unwrap_or_default();
                subheader = Some(text);
            }
            Rule::property_line => {
                let mut key = String::new();
                let mut value = String::new();
                for field in inner.into_inner() {
                    match field.as_rule() {
                        Rule::property_key => key = field.as_str().to_string(),
                        Rule::property_value => value = field.as_str().to_string(),
                        _ => {}
                    }
                }
                lines.push(RawPropertyLine { key, value });
            }
            _ => {}
        }
    }

    Section::Properties { subheader, lines }
}

/// Walk `vaal_section`: name + properties + description + stats.
fn walk_vaal_section(pair: pest::iterators::Pair<'_, Rule>) -> VaalGemData {
    let mut name = String::new();
    let mut properties = Vec::new();
    let mut description = None;
    let mut stats = Vec::new();
    let mut quality_stats = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::vaal_name_line => {
                // Grammar matched "Vaal " prefix, rest_of_line has the remainder
                let rest = inner
                    .into_inner()
                    .find(|p| p.as_rule() == Rule::rest_of_line)
                    .map(|p| p.as_str())
                    .unwrap_or_default();
                name = format!("Vaal {rest}");
            }
            Rule::vaal_properties_section => {
                // Mixed format: some lines have ": ", some don't
                for line in inner.into_inner() {
                    if line.as_rule() == Rule::generic_line {
                        let text = line
                            .into_inner()
                            .find(|p| p.as_rule() == Rule::rest_of_line)
                            .map(|p| p.as_str())
                            .unwrap_or_default();
                        if let Some((key, value)) = text.split_once(": ") {
                            let augmented = value.contains("(augmented)");
                            let clean_value = value
                                .replace(" (augmented)", "")
                                .replace("(augmented)", "")
                                .trim()
                                .to_string();
                            properties.push(ItemProperty {
                                name: key.to_string(),
                                value: clean_value,
                                augmented,
                                synthetic: false,
                            });
                        } else {
                            // Non-property line like "Can Store 2 Uses"
                            properties.push(ItemProperty {
                                name: text.to_string(),
                                value: String::new(),
                                augmented: false,
                                synthetic: false,
                            });
                        }
                    }
                }
            }
            Rule::gem_description_section => {
                let lines = collect_section_text(inner);
                description = Some(lines.join("\n"));
            }
            Rule::gem_stats_section => {
                let (s, q) = walk_gem_stats(inner);
                stats = s;
                quality_stats = q;
            }
            _ => {}
        }
    }

    VaalGemData {
        name,
        properties,
        description,
        stats,
        quality_stats,
    }
}

/// Walk `gem_tags_and_properties`: first line is tags, rest are `Key: Value` properties.
fn walk_gem_tags_and_properties(
    pair: pest::iterators::Pair<'_, Rule>,
) -> (Vec<String>, Vec<RawPropertyLine>) {
    let mut tags = Vec::new();
    let mut props = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::gem_tags_line => {
                let text = inner
                    .into_inner()
                    .find(|p| p.as_rule() == Rule::rest_of_line)
                    .map(|p| p.as_str())
                    .unwrap_or_default();
                tags = text.split(", ").map(|s| s.trim().to_string()).collect();
            }
            Rule::property_line => {
                let mut key = String::new();
                let mut value = String::new();
                for field in inner.into_inner() {
                    match field.as_rule() {
                        Rule::property_key => key = field.as_str().to_string(),
                        Rule::property_value => value = field.as_str().to_string(),
                        _ => {}
                    }
                }
                props.push(RawPropertyLine { key, value });
            }
            _ => {}
        }
    }

    (tags, props)
}

/// Walk `gem_stats_section`, returning (stats, quality stats) split at the quality marker.
fn walk_gem_stats(pair: pest::iterators::Pair<'_, Rule>) -> (Vec<String>, Vec<String>) {
    let mut stats = Vec::new();
    let mut quality_stats = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::gem_stat_lines => {
                for line in inner.into_inner() {
                    if line.as_rule() == Rule::gem_stat_line {
                        if let Some(text) = line
                            .into_inner()
                            .find(|p| p.as_rule() == Rule::rest_of_line)
                        {
                            stats.push(text.as_str().to_string());
                        }
                    }
                }
            }
            Rule::gem_quality_block => {
                for line in inner.into_inner() {
                    if line.as_rule() == Rule::generic_line {
                        if let Some(text) = line
                            .into_inner()
                            .find(|p| p.as_rule() == Rule::rest_of_line)
                        {
                            quality_stats.push(text.as_str().to_string());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    (stats, quality_stats)
}

/// Collect all text lines from a section into a Vec<String>.
fn collect_section_text(pair: pest::iterators::Pair<'_, Rule>) -> Vec<String> {
    let mut lines = Vec::new();
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::generic_line | Rule::gem_stat_line | Rule::gem_usage_line => {
                if let Some(text) = inner
                    .into_inner()
                    .find(|p| p.as_rule() == Rule::rest_of_line)
                {
                    lines.push(text.as_str().to_string());
                }
            }
            Rule::blank_line => lines.push(String::new()),
            _ => {}
        }
    }
    lines
}

fn extract_integer(pair: pest::iterators::Pair<'_, Rule>) -> Result<u32, ParseError> {
    pair.into_inner()
        .find(|p| p.as_rule() == Rule::integer)
        .ok_or_else(|| ParseError::Internal("missing integer".into()))?
        .as_str()
        .parse::<u32>()
        .map_err(|e| ParseError::Internal(format!("invalid integer: {e}")))
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("grammar error: {message}")]
    Grammar { message: String },
    #[error("internal error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_gem_fixture(name: &str) -> bool {
        let path = format!("{}/../../fixtures/items/{name}", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(&path).unwrap();
        let input = if text.ends_with('\n') {
            text
        } else {
            format!("{text}\n")
        };
        let pairs = ItemParser::parse(Rule::item, &input).unwrap();
        let item = pairs.into_iter().next().unwrap();
        // Check if it matched gem_item (not standard_item)
        for inner in item.into_inner() {
            if inner.as_rule() == Rule::gem_item {
                return true;
            }
            if inner.as_rule() == Rule::standard_item {
                return false;
            }
        }
        false
    }

    #[test]
    fn gem_grammar_matches_shockwave_totem() {
        assert!(
            parse_gem_fixture("gem-skill-shockwave-totem.txt"),
            "should match gem_item path"
        );
    }

    #[test]
    fn gem_grammar_matches_all_gem_fixtures() {
        let fixtures = [
            "gem-skill-shockwave-totem.txt",
            "gem-skill-portal-corrupted.txt",
            "gem-skill-transfigured-consecrated-path-of-endurance.txt",
            "gem-skill-transfigured-shock-nova-of-procession.txt",
            "gem-skill-transfigured-dual-strike-of-ambidexterity.txt",
            "gem-skill-war-banner.txt",
            "gem-skill-imbued-flicker-strike.txt",
            "gem-support-faster-casting.txt",
            "gem-support-spell-totem-corrupted.txt",
            "gem-support-exceptional-transfusion.txt",
            "gem-support-awakened-enhance.txt",
            "gem-vaal-ice-nova.txt",
        ];
        let mut failed = Vec::new();
        for name in &fixtures {
            if !parse_gem_fixture(name) {
                failed.push(*name);
            }
        }
        assert!(
            failed.is_empty(),
            "these gem fixtures did NOT match gem_item path: {failed:?}"
        );
    }
}
