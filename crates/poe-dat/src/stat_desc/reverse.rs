use std::collections::HashMap;

use regex::Regex;

use super::types::*;

/// Result of matching display text against the reverse index.
#[derive(Debug, Clone)]
pub struct StatMatch {
    /// The stat IDs that produced this display text.
    pub stat_ids: Vec<String>,
    /// Raw stat values (transforms reversed where possible).
    pub values: Vec<i64>,
}

/// Reverse index for matching display text to stat IDs and values.
///
/// Built from parsed stat description files. Indexes English variants only.
///
/// Uses a two-level lookup strategy:
/// 1. Template key: replace numbers in display text with markers, hash-match against templates
/// 2. Regex: for ambiguous cases, fall back to compiled regex
pub struct ReverseIndex {
    /// All pattern entries, indexed by position.
    entries: Vec<PatternEntry>,
    /// Map from template key → entry indices.
    /// Template key = format string with placeholders replaced by \x00.
    template_map: HashMap<String, Vec<usize>>,
    /// Precompiled regex for finding number tokens in display text.
    number_re: Regex,
}

struct PatternEntry {
    regex_pattern: String,
    stat_ids: Vec<String>,
    transforms: Vec<Transform>,
    ranges: Vec<Range>,
    /// Maps capture group index → placeholder index in format string
    capture_to_placeholder: Vec<usize>,
}

/// Segments parsed from a format string.
#[derive(Debug, Clone)]
enum Segment {
    Literal(String),
    Placeholder { index: usize, signed: bool },
}

/// Regex for finding number tokens in display text.
/// Matches integers and decimals, optionally signed.
const NUMBER_PATTERN: &str = r"[+-]?\d+(?:\.\d+)?";

impl ReverseIndex {
    /// Build a reverse index from a parsed stat description file.
    pub fn from_file(file: &StatDescriptionFile) -> Self {
        let mut entries = Vec::new();
        let mut template_map: HashMap<String, Vec<usize>> = HashMap::new();

        for desc in &file.descriptions {
            // English = first lang block (language = None)
            let english = desc.languages.iter().find(|l| l.language.is_none());
            let Some(english) = english else { continue };

            for variant in &english.variants {
                let segments = parse_format_string(&variant.format_string);
                let Some(regex_pattern) = build_regex_pattern(&segments) else {
                    continue;
                };

                let capture_to_placeholder: Vec<usize> = segments
                    .iter()
                    .filter_map(|s| match s {
                        Segment::Placeholder { index, .. } => Some(*index),
                        Segment::Literal(_) => None,
                    })
                    .collect();

                let template_key = build_template_key(&segments);
                let idx = entries.len();

                entries.push(PatternEntry {
                    regex_pattern,
                    stat_ids: desc.stat_ids.clone(),
                    transforms: variant.transforms.clone(),
                    ranges: variant.ranges.clone(),
                    capture_to_placeholder,
                });

                template_map.entry(template_key).or_default().push(idx);
            }
        }

        ReverseIndex {
            entries,
            template_map,
            number_re: Regex::new(NUMBER_PATTERN).expect("valid regex"),
        }
    }

    /// Look up stat IDs and raw values from a single line of display text.
    pub fn lookup(&self, display_text: &str) -> Option<StatMatch> {
        // Find all number tokens in the display text
        let number_positions: Vec<(usize, usize)> = self.number_re
            .find_iter(display_text)
            .map(|m| (m.start(), m.end()))
            .collect();

        let n = number_positions.len();
        if n > 10 {
            // Too many numbers — likely not a stat line
            return None;
        }

        // Try replacing all subsets of numbers with marker \x00.
        // Start with replacing ALL numbers (most common case),
        // then try leaving out 1, 2, etc.
        for leave_out in 0..=n {
            if leave_out > 3 {
                break; // Don't try too many combinations
            }
            for skip_set in combinations(n, leave_out) {
                let key = build_display_key(display_text, &number_positions, &skip_set);
                if let Some(indices) = self.template_map.get(&key) {
                    for &idx in indices {
                        // Extract values: the numbers that WERE replaced (not skipped)
                        let values: Vec<&str> = number_positions
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| !skip_set.contains(i))
                            .map(|(_, &(start, end))| &display_text[start..end])
                            .collect();
                        if let Some(m) = self.try_match_with_values(idx, &values) {
                            return Some(m);
                        }
                    }
                }
            }
        }

        None
    }

    fn try_match_with_values(&self, idx: usize, value_strs: &[&str]) -> Option<StatMatch> {
        let entry = &self.entries[idx];

        // Parse displayed values
        let displayed_values: Vec<f64> = value_strs
            .iter()
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();

        if displayed_values.len() != entry.capture_to_placeholder.len() {
            return None;
        }

        // Reverse transforms to get raw stat values
        let mut raw_values: Vec<i64> = displayed_values.iter().map(|&v| v as i64).collect();

        for transform in &entry.transforms {
            // stat_index in the file format is 1-based; placeholder indices are 0-based
            let placeholder_idx = transform.stat_index.saturating_sub(1);
            if let Some(cap_idx) = entry
                .capture_to_placeholder
                .iter()
                .position(|&p| p == placeholder_idx)
            {
                if cap_idx < displayed_values.len() {
                    raw_values[cap_idx] =
                        reverse_transform(&transform.kind, displayed_values[cap_idx]);
                }
            }
        }

        // Validate ranges against raw values
        if !entry.ranges.is_empty() && !raw_values.is_empty() {
            for (range, &value) in entry.ranges.iter().zip(raw_values.iter()) {
                if !range.matches(value) {
                    return None;
                }
            }
        }

        Some(StatMatch {
            stat_ids: entry.stat_ids.clone(),
            values: raw_values,
        })
    }

    /// Fallback: compile regex and try match (for debugging/testing).
    pub fn lookup_regex(&self, display_text: &str) -> Option<StatMatch> {
        for (idx, entry) in self.entries.iter().enumerate() {
            let Ok(regex) = Regex::new(&entry.regex_pattern) else {
                continue;
            };
            let Some(caps) = regex.captures(display_text) else {
                continue;
            };
            let num_captures = caps.len() - 1;
            let value_strs: Vec<&str> = (1..=num_captures)
                .filter_map(|i| caps.get(i).map(|m| m.as_str()))
                .collect();
            if let Some(m) = self.try_match_with_values(idx, &value_strs) {
                return Some(m);
            }
        }
        None
    }

    /// Number of indexed patterns.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Build a template key from format string segments.
/// Replaces each placeholder with \x00 marker.
fn build_template_key(segments: &[Segment]) -> String {
    let mut key = String::new();
    for seg in segments {
        match seg {
            Segment::Literal(text) => key.push_str(text),
            Segment::Placeholder { .. } => key.push('\x00'),
        }
    }
    key
}

/// Build a display key by replacing selected number tokens with \x00.
/// `skip_set` contains indices of number_positions to NOT replace (literal numbers).
fn build_display_key(
    display_text: &str,
    number_positions: &[(usize, usize)],
    skip_set: &[usize],
) -> String {
    let mut key = String::new();
    let mut last_end = 0;
    for (i, &(start, end)) in number_positions.iter().enumerate() {
        key.push_str(&display_text[last_end..start]);
        if skip_set.contains(&i) {
            // Keep original number text (it's literal in the template)
            key.push_str(&display_text[start..end]);
        } else {
            // Replace with marker (it's a placeholder value)
            key.push('\x00');
        }
        last_end = end;
    }
    key.push_str(&display_text[last_end..]);
    key
}

/// Generate all combinations of `k` items from `0..n`.
fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    if k == 0 {
        return vec![vec![]];
    }
    if k > n {
        return vec![];
    }
    let mut result = Vec::new();
    let mut combo = Vec::with_capacity(k);
    combinations_recursive(n, k, 0, &mut combo, &mut result);
    result
}

fn combinations_recursive(
    n: usize,
    k: usize,
    start: usize,
    current: &mut Vec<usize>,
    result: &mut Vec<Vec<usize>>,
) {
    if current.len() == k {
        result.push(current.clone());
        return;
    }
    for i in start..n {
        current.push(i);
        combinations_recursive(n, k, i + 1, current, result);
        current.pop();
    }
}

/// Parse a format string into literal and placeholder segments.
fn parse_format_string(fmt: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut literal = String::new();
    let mut chars = fmt.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '{' {
            // Collect placeholder content until '}'
            let mut inside = String::new();
            for c in chars.by_ref() {
                if c == '}' {
                    break;
                }
                inside.push(c);
            }

            if !literal.is_empty() {
                segments.push(Segment::Literal(std::mem::take(&mut literal)));
            }

            let signed = inside.contains('+');
            let index: usize = inside
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .unwrap_or(0);

            segments.push(Segment::Placeholder { index, signed });
        } else if c == '\\' && chars.peek() == Some(&'n') {
            chars.next();
            literal.push('\n');
        } else {
            literal.push(c);
        }
    }

    if !literal.is_empty() {
        segments.push(Segment::Literal(literal));
    }
    segments
}

/// Build a regex pattern from parsed format string segments.
fn build_regex_pattern(segments: &[Segment]) -> Option<String> {
    if segments.is_empty() {
        return None;
    }

    let mut pattern = String::from("^");
    for seg in segments {
        match seg {
            Segment::Literal(text) => {
                pattern.push_str(&regex::escape(text));
            }
            Segment::Placeholder { signed, .. } => {
                if *signed {
                    // :+d — always shows sign
                    pattern.push_str(r"([+-]?\d+(?:\.\d+)?)");
                } else {
                    // plain {N} — minus optional
                    pattern.push_str(r"(-?\d+(?:\.\d+)?)");
                }
            }
        }
    }
    pattern.push('$');
    Some(pattern)
}

/// Reverse a transform to recover the raw stat value from a displayed value.
fn reverse_transform(kind: &TransformKind, displayed: f64) -> i64 {
    #[expect(clippy::cast_possible_truncation)]
    match kind {
        TransformKind::Negate => (-displayed) as i64,
        TransformKind::NegateAndDouble => (-displayed / 2.0) as i64,
        TransformKind::Double => (displayed / 2.0) as i64,

        TransformKind::MillisecondsToSeconds
        | TransformKind::MillisecondsToSeconds0dp
        | TransformKind::MillisecondsToSeconds1dp
        | TransformKind::MillisecondsToSeconds2dp
        | TransformKind::MillisecondsToSeconds2dpIfRequired => (displayed * 1000.0) as i64,

        TransformKind::DecisecondsToSeconds => (displayed * 10.0) as i64,

        TransformKind::PerMinuteToPerSecond
        | TransformKind::PerMinuteToPerSecond0dp
        | TransformKind::PerMinuteToPerSecond1dp
        | TransformKind::PerMinuteToPerSecond2dp
        | TransformKind::PerMinuteToPerSecond2dpIfRequired => (displayed * 60.0) as i64,

        TransformKind::DivideByTwo0dp => (displayed * 2.0) as i64,
        TransformKind::DivideByThree => (displayed * 3.0) as i64,
        TransformKind::DivideByFour => (displayed * 4.0) as i64,
        TransformKind::DivideByFive => (displayed * 5.0) as i64,
        TransformKind::DivideBySix => (displayed * 6.0) as i64,
        TransformKind::DivideByTen0dp
        | TransformKind::DivideByTen1dp
        | TransformKind::DivideByTen1dpIfRequired => (displayed * 10.0) as i64,
        TransformKind::DivideByTwelve => (displayed * 12.0) as i64,
        TransformKind::DivideByFifteen0dp => (displayed * 15.0) as i64,
        TransformKind::DivideByTwenty => (displayed * 20.0) as i64,
        TransformKind::DivideByTwentyThenDouble0dp => (displayed / 2.0 * 20.0) as i64,

        TransformKind::DivideByOneHundred
        | TransformKind::DivideByOneHundred2dp
        | TransformKind::DivideByOneHundred2dpIfRequired => (displayed * 100.0) as i64,
        TransformKind::DivideByOneHundredAndNegate => (-displayed * 100.0) as i64,

        TransformKind::DivideByOneThousand => (displayed * 1000.0) as i64,

        TransformKind::TimesOnePointFive => (displayed / 1.5) as i64,
        TransformKind::TimesTwenty => (displayed / 20.0) as i64,
        TransformKind::PlusTwoHundred => (displayed - 200.0) as i64,

        TransformKind::ThirtyPercentOfValue => (displayed / 0.3) as i64,
        TransformKind::SixtyPercentOfValue => (displayed / 0.6) as i64,
        TransformKind::PermyriadPerMinuteToPercentPerSecond => {
            (displayed * 10000.0 * 60.0) as i64
        }

        TransformKind::MultiplicativeDamageModifier => (displayed - 100.0) as i64,

        // Lookup-based transforms — can't reverse, return displayed as-is
        TransformKind::OldLeechPercent
        | TransformKind::OldLeechPermyriad
        | TransformKind::ModValueToItemClass
        | TransformKind::DisplayIndexableSupport
        | TransformKind::DisplayIndexableSkill
        | TransformKind::PassiveHash
        | TransformKind::AfflictionRewardType
        | TransformKind::LocationsToMetres
        | TransformKind::TreeExpansionJewelPassive
        | TransformKind::WeaponTreeUniqueBaseTypeName
        | TransformKind::Other(_) => displayed as i64,
    }
}
