use pest::Parser;
use pest_derive::Parser;

use crate::types::{
    Header, InfluenceKind, ModGroup, ModHeader, ModSection, ModSlot, ModSource, ModTierKind,
    Rarity, RawItem, Requirement, Section, StatusKind,
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
    let mut header = None;
    let mut sections = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::header_section => header = Some(walk_header(inner)),
            Rule::section => {
                let section_inner = inner.into_inner().next().unwrap();
                sections.push(walk_section(section_inner)?);
            }
            _ => {}
        }
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
                    _ => ModSource::MasterCrafted,
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
