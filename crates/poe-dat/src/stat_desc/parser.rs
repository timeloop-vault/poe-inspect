use pest::Parser;
use pest_derive::Parser;

use super::types::*;

#[derive(Parser)]
#[grammar = "stat_desc/grammar.pest"]
struct StatDescParser;

/// Parse a stat description file from a UTF-8 string.
///
/// The input should already be converted from UTF-16LE to UTF-8.
pub fn parse(input: &str) -> Result<StatDescriptionFile, ParseError> {
    // Normalize each line:
    // 1. Strip trailing whitespace (real files have trailing tabs)
    // 2. Normalize leading whitespace: count tab characters in all leading
    //    whitespace (tabs + spaces), replace with that many tabs.
    //    Handles mixed indentation like "\t \t" → "\t\t"
    // 3. Replace remaining mid-line tabs with spaces (tabs appear as separators
    //    within variant lines, but WHITESPACE only matches spaces)
    let cleaned: String = input
        .lines()
        .map(|line| {
            let trimmed = line.trim_end();
            let leading_ws = trimmed.len() - trimmed.trim_start().len();
            let tab_count = trimmed[..leading_ws]
                .bytes()
                .filter(|&b| b == b'\t')
                .count();
            let content = &trimmed[leading_ws..];
            let tabs: String = "\t".repeat(tab_count);
            if content.contains('\t') {
                format!("{tabs}{}", content.replace('\t', " "))
            } else {
                format!("{tabs}{content}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let file_pair = StatDescParser::parse(Rule::file, &cleaned)
        .map_err(|e| ParseError::Grammar(e.to_string()))?
        .next()
        .ok_or_else(|| ParseError::Grammar("empty parse result".to_string()))?;

    let mut file = StatDescriptionFile {
        includes: Vec::new(),
        no_descriptions: Vec::new(),
        descriptions: Vec::new(),
    };

    for pair in file_pair.into_inner() {
        match pair.as_rule() {
            Rule::include_dir => {
                let path = pair
                    .into_inner()
                    .find(|p| p.as_rule() == Rule::quoted_string)
                    .map(|p| extract_quoted_string(p))
                    .unwrap_or_default();
                file.includes.push(path);
            }
            Rule::no_description => {
                let stat_id = pair
                    .into_inner()
                    .find(|p| p.as_rule() == Rule::stat_id)
                    .map(|p| p.as_str().to_string())
                    .unwrap_or_default();
                file.no_descriptions.push(stat_id);
            }
            Rule::no_identifiers => {}
            Rule::description => {
                file.descriptions.push(parse_description(pair)?);
            }
            Rule::EOI => {}
            other => {
                return Err(ParseError::UnexpectedRule(format!("{other:?}")));
            }
        }
    }

    Ok(file)
}

fn parse_description(pair: pest::iterators::Pair<'_, Rule>) -> Result<StatDescription, ParseError> {
    let mut stat_ids = Vec::new();
    let mut languages = Vec::new();
    let mut current_lang: Option<String> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::stat_header => {
                for p in inner.into_inner() {
                    if p.as_rule() == Rule::stat_id {
                        stat_ids.push(p.as_str().to_string());
                    }
                }
            }
            Rule::lang_block => {
                let lang_block = parse_lang_block(inner, current_lang.take())?;
                languages.push(lang_block);
            }
            Rule::lang_switch => {
                let lang_name = inner
                    .into_inner()
                    .find(|p| p.as_rule() == Rule::quoted_string)
                    .map(|p| extract_quoted_string(p))
                    .unwrap_or_default();
                current_lang = Some(lang_name);
            }
            _ => {}
        }
    }

    Ok(StatDescription {
        stat_ids,
        languages,
    })
}

fn parse_lang_block(
    pair: pest::iterators::Pair<'_, Rule>,
    language: Option<String>,
) -> Result<LangBlock, ParseError> {
    let mut variants = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::variant_line {
            variants.push(parse_variant_line(inner)?);
        }
    }

    Ok(LangBlock { language, variants })
}

fn parse_variant_line(pair: pest::iterators::Pair<'_, Rule>) -> Result<Variant, ParseError> {
    let mut ranges = Vec::new();
    let mut format_string = String::new();
    let mut transforms = Vec::new();
    let mut is_canonical = false;
    let mut canonical_stat = None;
    let mut reminder_id = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::range => {
                ranges.push(parse_range(inner.as_str())?);
            }
            Rule::quoted_string => {
                format_string = extract_quoted_string(inner);
            }
            Rule::transform => {
                transforms.push(parse_transform(inner)?);
            }
            Rule::canonical_line => {
                is_canonical = true;
            }
            Rule::canonical_stat => {
                canonical_stat = inner
                    .into_inner()
                    .find(|p| p.as_rule() == Rule::stat_index)
                    .and_then(|p| p.as_str().parse().ok());
            }
            Rule::reminderstring => {
                reminder_id = inner
                    .into_inner()
                    .find(|p| p.as_rule() == Rule::identifier)
                    .map(|p| p.as_str().to_string());
            }
            _ => {}
        }
    }

    Ok(Variant {
        ranges,
        format_string,
        transforms,
        is_canonical,
        canonical_stat,
        reminder_id,
    })
}

fn parse_range(s: &str) -> Result<Range, ParseError> {
    if s == "#" {
        return Ok(Range::Any);
    }

    if let Some(rest) = s.strip_prefix('!') {
        let n: i64 = rest
            .parse()
            .map_err(|_| ParseError::InvalidRange(s.to_string()))?;
        return Ok(Range::Not(n));
    }

    if s.contains('|') {
        // Split on '|' and take first + last segment as lo/hi bounds.
        // Handles normal "N|M", "#|N", "N|#" and malformed "1|1|#".
        let parts: Vec<&str> = s.split('|').collect();
        let first = parts[0];
        let last = parts[parts.len() - 1];
        let lo = if first == "#" {
            Bound::Unbounded
        } else {
            Bound::Value(
                first
                    .parse()
                    .map_err(|_| ParseError::InvalidRange(s.to_string()))?,
            )
        };
        let hi = if last == "#" {
            Bound::Unbounded
        } else {
            Bound::Value(
                last.parse()
                    .map_err(|_| ParseError::InvalidRange(s.to_string()))?,
            )
        };
        Ok(Range::Between(lo, hi))
    } else {
        let n: i64 = s
            .parse()
            .map_err(|_| ParseError::InvalidRange(s.to_string()))?;
        Ok(Range::Exact(n))
    }
}

fn parse_transform(pair: pest::iterators::Pair<'_, Rule>) -> Result<Transform, ParseError> {
    let mut kind = None;
    let mut stat_index = 0;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::transform_name => {
                kind = Some(match inner.as_str() {
                    "negate" => TransformKind::Negate,
                    "negate_and_double" => TransformKind::NegateAndDouble,
                    "double" => TransformKind::Double,
                    "milliseconds_to_seconds" => TransformKind::MillisecondsToSeconds,
                    "milliseconds_to_seconds_0dp" => TransformKind::MillisecondsToSeconds0dp,
                    "milliseconds_to_seconds_1dp" => TransformKind::MillisecondsToSeconds1dp,
                    "milliseconds_to_seconds_2dp" => TransformKind::MillisecondsToSeconds2dp,
                    "milliseconds_to_seconds_2dp_if_required" => {
                        TransformKind::MillisecondsToSeconds2dpIfRequired
                    }
                    "deciseconds_to_seconds" => TransformKind::DecisecondsToSeconds,
                    "per_minute_to_per_second" => TransformKind::PerMinuteToPerSecond,
                    "per_minute_to_per_second_0dp" => TransformKind::PerMinuteToPerSecond0dp,
                    "per_minute_to_per_second_1dp" => TransformKind::PerMinuteToPerSecond1dp,
                    "per_minute_to_per_second_2dp" => TransformKind::PerMinuteToPerSecond2dp,
                    "per_minute_to_per_second_2dp_if_required" => {
                        TransformKind::PerMinuteToPerSecond2dpIfRequired
                    }
                    "divide_by_two_0dp" => TransformKind::DivideByTwo0dp,
                    "divide_by_three" => TransformKind::DivideByThree,
                    "divide_by_four" => TransformKind::DivideByFour,
                    "divide_by_five" => TransformKind::DivideByFive,
                    "divide_by_six" => TransformKind::DivideBySix,
                    "divide_by_ten_0dp" => TransformKind::DivideByTen0dp,
                    "divide_by_ten_1dp" => TransformKind::DivideByTen1dp,
                    "divide_by_ten_1dp_if_required" => TransformKind::DivideByTen1dpIfRequired,
                    "divide_by_twelve" => TransformKind::DivideByTwelve,
                    "divide_by_fifteen_0dp" => TransformKind::DivideByFifteen0dp,
                    "divide_by_twenty" => TransformKind::DivideByTwenty,
                    "divide_by_twenty_then_double_0dp" => {
                        TransformKind::DivideByTwentyThenDouble0dp
                    }
                    "divide_by_one_hundred" => TransformKind::DivideByOneHundred,
                    "divide_by_one_hundred_2dp" => TransformKind::DivideByOneHundred2dp,
                    "divide_by_one_hundred_and_negate" => {
                        TransformKind::DivideByOneHundredAndNegate
                    }
                    "divide_by_one_hundred_2dp_if_required" => {
                        TransformKind::DivideByOneHundred2dpIfRequired
                    }
                    "divide_by_one_thousand" => TransformKind::DivideByOneThousand,
                    "times_one_point_five" => TransformKind::TimesOnePointFive,
                    "times_twenty" => TransformKind::TimesTwenty,
                    "plus_two_hundred" => TransformKind::PlusTwoHundred,
                    "30%_of_value" => TransformKind::ThirtyPercentOfValue,
                    "60%_of_value" => TransformKind::SixtyPercentOfValue,
                    "permyriad_per_minute_to_%_per_second" => {
                        TransformKind::PermyriadPerMinuteToPercentPerSecond
                    }
                    "old_leech_percent" => TransformKind::OldLeechPercent,
                    "old_leech_permyriad" => TransformKind::OldLeechPermyriad,
                    "multiplicative_damage_modifier" => {
                        TransformKind::MultiplicativeDamageModifier
                    }
                    "mod_value_to_item_class" => TransformKind::ModValueToItemClass,
                    "display_indexable_support" => TransformKind::DisplayIndexableSupport,
                    "display_indexable_skill" => TransformKind::DisplayIndexableSkill,
                    "passive_hash" => TransformKind::PassiveHash,
                    "affliction_reward_type" => TransformKind::AfflictionRewardType,
                    "locations_to_metres" => TransformKind::LocationsToMetres,
                    "tree_expansion_jewel_passive" => TransformKind::TreeExpansionJewelPassive,
                    "weapon_tree_unique_base_type_name" => {
                        TransformKind::WeaponTreeUniqueBaseTypeName
                    }
                    other => TransformKind::Other(other.to_string()),
                });
            }
            Rule::stat_index => {
                stat_index = inner
                    .as_str()
                    .parse()
                    .map_err(|_| ParseError::InvalidStatIndex(inner.as_str().to_string()))?;
            }
            _ => {}
        }
    }

    Ok(Transform {
        kind: kind.ok_or_else(|| ParseError::MissingTransformName)?,
        stat_index,
    })
}

fn extract_quoted_string(pair: pest::iterators::Pair<'_, Rule>) -> String {
    pair.into_inner()
        .find(|p| p.as_rule() == Rule::inner_string)
        .map(|p| p.as_str().to_string())
        .unwrap_or_default()
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("grammar error: {0}")]
    Grammar(String),
    #[error("invalid range: {0}")]
    InvalidRange(String),
    #[error("invalid stat index: {0}")]
    InvalidStatIndex(String),
    #[error("missing transform name")]
    MissingTransformName,
    #[error("unexpected rule: {0}")]
    UnexpectedRule(String),
}
