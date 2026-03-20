//! Evaluation engine — tests rules against resolved items.
//!
//! This module contains zero `PoE` domain knowledge. All game-specific
//! lookups go through `GameData`. The evaluator is pure logic.

use poe_data::GameData;
use poe_item::types::{ModSlot, ModTierKind, ResolvedItem};

use poe_item::types::ModSource;

use crate::predicate::{
    Cmp, ModSlotKind, ModSourceKind, Predicate, RarityValue, StatCondition, TierKindFilter,
};
use crate::profile::{MatchedRule, Profile, ScoreResult, UnmatchedRule};
use crate::rule::Rule;

impl ModSlotKind {
    /// Check whether a `ModSlot` matches this filter.
    fn matches(self, slot: ModSlot) -> bool {
        match self {
            Self::Prefix => slot == ModSlot::Prefix,
            Self::Suffix => slot == ModSlot::Suffix,
            Self::Implicit => slot == ModSlot::Implicit,
            Self::Affix => slot == ModSlot::Prefix || slot == ModSlot::Suffix,
        }
    }
}

impl ModSourceKind {
    /// Check whether a mod's source matches this filter.
    fn matches(self, source: ModSource) -> bool {
        match self {
            Self::Regular => source == ModSource::Regular,
            Self::Fractured => source == ModSource::Fractured,
            Self::MasterCrafted => source == ModSource::MasterCrafted,
        }
    }
}

/// Evaluate a rule against an item. Returns `true` if the item matches.
pub fn evaluate(item: &ResolvedItem, rule: &Rule, gd: &GameData) -> bool {
    match rule {
        Rule::Pred(p) => eval_predicate(item, p, gd),
        Rule::All { rules } => rules.iter().all(|r| evaluate(item, r, gd)),
        Rule::Any { rules } => rules.iter().any(|r| evaluate(item, r, gd)),
        Rule::Not { rule } => !evaluate(item, rule, gd),
    }
}

fn eval_predicate(item: &ResolvedItem, pred: &Predicate, gd: &GameData) -> bool {
    match pred {
        // ── Header ───────────────────────────────────────────────────
        Predicate::Rarity { op, value } => {
            let item_rarity = RarityValue::from_rarity(item.header.rarity);
            op.eval(&item_rarity, value)
        }

        Predicate::ItemClass { op, value } => match *op {
            Cmp::Eq => item.header.item_class == *value,
            Cmp::Ne => item.header.item_class != *value,
            _ => false,
        },

        Predicate::BaseType { op, value } => match *op {
            Cmp::Eq => item.header.base_type == *value,
            Cmp::Ne => item.header.base_type != *value,
            _ => false,
        },

        Predicate::BaseTypeContains { value } => item.header.base_type.contains(value.as_str()),

        // ── Numeric properties ───────────────────────────────────────
        Predicate::ItemLevel { op, value } => {
            item.item_level.is_some_and(|lvl| op.eval(&lvl, value))
        }

        // ── Mod predicates ───────────────────────────────────────────
        Predicate::ModCount { slot, op, value } => {
            let count = count_mods_matching_slot(item, *slot);
            op.eval(&count, value)
        }

        Predicate::OpenMods { slot, op, value } => {
            let open = open_mod_count(item, *slot, gd);
            op.eval(&open, value)
        }

        Predicate::HasModNamed { name } => item
            .all_mods()
            .any(|m| m.header.name.as_deref() == Some(name.as_str())),

        // ── Stat value predicates ────────────────────────────────────
        Predicate::StatValue { conditions } => eval_stat_value(item, conditions),

        Predicate::StatTier {
            text: _,
            stat_ids,
            kind,
            op,
            value,
            source,
        } => eval_stat_tier(item, stat_ids, *kind, *op, *value, *source),

        Predicate::TierCount {
            kind,
            op,
            value,
            min_count,
            slot,
            source,
        } => eval_tier_count(item, *kind, *op, *value, *min_count, *slot, *source),

        Predicate::RollPercent {
            text: _,
            stat_ids,
            value_index,
            op,
            value,
        } => find_matching_stats(item, stat_ids).any(|sl| {
            sl.values.get(*value_index).is_some_and(|vr| {
                let span = vr.max - vr.min;
                if span == 0 {
                    return op.eval(&100_u32, value);
                }
                let pct = ((vr.current - vr.min) * 100 / span).clamp(0, 100);
                op.eval(&u32::try_from(pct).unwrap_or(0), value)
            })
        }),

        // ── Influence / status ───────────────────────────────────────
        Predicate::HasInfluence { influence } => {
            item.influences.iter().any(|i| influence.matches(*i))
        }

        Predicate::HasStatus { status } => item.statuses.iter().any(|s| status.matches(*s)),

        Predicate::InfluenceCount { op, value } => {
            let count = u32::try_from(item.influences.len()).unwrap_or(u32::MAX);
            op.eval(&count, value)
        }

        // ── Socket / quality predicates ─────────────────────────────
        Predicate::SocketCount { op, value } => {
            let count = item.sockets.as_deref().map_or(0, count_sockets);
            op.eval(&count, value)
        }

        Predicate::LinkCount { op, value } => {
            let max_link = item.sockets.as_deref().map_or(0, max_link_group);
            op.eval(&max_link, value)
        }

        Predicate::Quality { op, value } => {
            let quality = extract_quality(item);
            op.eval(&quality, value)
        }
    }
}

/// Score an item against a profile. Returns detailed results including
/// which rules matched and the total score.
pub fn score(item: &ResolvedItem, profile: &Profile, gd: &GameData) -> ScoreResult {
    // Check filter first
    if let Some(filter) = &profile.filter {
        if !evaluate(item, filter, gd) {
            return ScoreResult {
                applicable: false,
                score: 0.0,
                matched: vec![],
                unmatched: vec![],
            };
        }
    }

    let mut total = 0.0;
    let mut matched = Vec::new();
    let mut unmatched = Vec::new();

    for sr in &profile.scoring {
        if evaluate(item, &sr.rule, gd) {
            total += sr.weight;
            matched.push(MatchedRule {
                label: sr.label.clone(),
                weight: sr.weight,
            });
        } else {
            unmatched.push(UnmatchedRule {
                label: sr.label.clone(),
                weight: sr.weight,
            });
        }
    }

    ScoreResult {
        applicable: true,
        score: total,
        matched,
        unmatched,
    }
}

// ── Helper functions ────────────────────────────────────────────────────────

/// Evaluate a `StatValue` predicate.
///
/// - 1 condition: any stat line on any mod that matches.
/// - 2+ conditions: ALL conditions must be satisfied on the SAME mod.
fn eval_stat_value(item: &ResolvedItem, conditions: &[StatCondition]) -> bool {
    if conditions.is_empty() {
        return false;
    }
    if conditions.len() == 1 {
        let c = &conditions[0];
        return find_matching_stats(item, &c.stat_ids).any(|sl| {
            sl.values
                .get(c.value_index)
                .is_some_and(|v| c.op.eval(&v.current, &c.value))
        });
    }
    // 2+ conditions: all must match on the SAME mod.
    item.all_mods().any(|m| {
        conditions.iter().all(|c| {
            if c.stat_ids.is_empty() {
                return false;
            }
            m.stat_lines.iter().any(|sl| {
                !sl.is_reminder
                    && sl.stat_ids.as_ref().is_some_and(|ids| {
                        ids.iter().any(|id| c.stat_ids.iter().any(|sid| sid == id))
                    })
                    && sl
                        .values
                        .get(c.value_index)
                        .is_some_and(|v| c.op.eval(&v.current, &c.value))
            })
        })
    })
}

/// Count mods matching a slot filter (supports `Affix` = Prefix + Suffix).
fn count_mods_matching_slot(item: &ResolvedItem, slot: ModSlotKind) -> u32 {
    item.all_mods()
        .filter(|m| slot.matches(m.header.slot))
        .count() as u32
}

/// Calculate open mod slots. Returns 0 if we can't determine the max
/// (e.g., missing game data or non-applicable rarity).
fn open_mod_count(item: &ResolvedItem, slot: ModSlotKind, gd: &GameData) -> u32 {
    let rarity_str = format!("{:?}", item.header.rarity);
    let Some(rarity_id) = poe_data::domain::rarity_to_ggpk_id(&rarity_str) else {
        return 0;
    };

    match slot {
        ModSlotKind::Prefix => {
            let max = u32::try_from(gd.max_prefixes(rarity_id).unwrap_or(0)).unwrap_or(0);
            let current = count_mods_matching_slot(item, ModSlotKind::Prefix);
            max.saturating_sub(current)
        }
        ModSlotKind::Suffix => {
            let max = u32::try_from(gd.max_suffixes(rarity_id).unwrap_or(0)).unwrap_or(0);
            let current = count_mods_matching_slot(item, ModSlotKind::Suffix);
            max.saturating_sub(current)
        }
        ModSlotKind::Affix => {
            let open_p = open_mod_count(item, ModSlotKind::Prefix, gd);
            let open_s = open_mod_count(item, ModSlotKind::Suffix, gd);
            open_p + open_s
        }
        ModSlotKind::Implicit => 0, // Implicit count isn't bounded by rarity
    }
}

/// Count total sockets from a socket string like `"R-R-G B"`.
/// Letters are sockets; `-` = linked, ` ` = new group.
fn count_sockets(sockets: &str) -> u32 {
    sockets.chars().filter(char::is_ascii_alphabetic).count() as u32
}

/// Find the largest linked group in a socket string like `"R-R-G B"`.
fn max_link_group(sockets: &str) -> u32 {
    let mut max: u32 = 0;
    let mut current: u32 = 0;
    for c in sockets.chars() {
        if c.is_ascii_alphabetic() {
            if current == 0 {
                current = 1;
            }
        } else if c == '-' {
            current += 1;
        } else {
            // Space or other separator = new group
            max = max.max(current);
            current = 0;
        }
    }
    max.max(current)
}

/// Extract quality value from item properties (e.g., "Quality" → "+20%" → 20).
fn extract_quality(item: &ResolvedItem) -> u32 {
    item.properties
        .iter()
        .find(|p| p.name == "Quality")
        .and_then(|p| {
            p.value
                .trim_start_matches('+')
                .trim_end_matches('%')
                .parse::<u32>()
                .ok()
        })
        .unwrap_or(0)
}

/// Check whether a `ModTierKind` matches a `TierKindFilter`.
fn tier_matches_filter(tier: &ModTierKind, kind: TierKindFilter) -> Option<u32> {
    match (tier, kind) {
        (ModTierKind::Tier(n), TierKindFilter::Tier | TierKindFilter::Either)
        | (ModTierKind::Rank(n), TierKindFilter::Rank | TierKindFilter::Either) => Some(*n),
        _ => None,
    }
}

/// Evaluate a `StatTier` predicate: find the mod providing the stat, check its tier.
fn eval_stat_tier(
    item: &ResolvedItem,
    stat_ids: &[String],
    kind: TierKindFilter,
    op: Cmp,
    value: u32,
    source: Option<ModSourceKind>,
) -> bool {
    if stat_ids.is_empty() {
        return false;
    }
    item.all_mods().any(|m| {
        // Optional source filter
        if let Some(src) = source {
            if !src.matches(m.header.source) {
                return false;
            }
        }
        // Check if this mod has a matching tier
        let Some(tier_num) = m
            .header
            .tier
            .as_ref()
            .and_then(|t| tier_matches_filter(t, kind))
        else {
            return false;
        };
        // Check if this mod provides any of the requested stat IDs
        m.stat_lines.iter().any(|sl| {
            !sl.is_reminder
                && sl
                    .stat_ids
                    .as_ref()
                    .is_some_and(|ids| ids.iter().any(|id| stat_ids.iter().any(|sid| sid == id)))
        }) && op.eval(&tier_num, &value)
    })
}

/// Evaluate a `TierCount` predicate: count mods matching a tier/rank condition.
fn eval_tier_count(
    item: &ResolvedItem,
    kind: TierKindFilter,
    op: Cmp,
    value: u32,
    min_count: u32,
    slot: Option<ModSlotKind>,
    source: Option<ModSourceKind>,
) -> bool {
    let count = item
        .all_mods()
        .filter(|m| {
            // Optional slot filter
            if let Some(s) = slot {
                if !s.matches(m.header.slot) {
                    return false;
                }
            }
            // Optional source filter
            if let Some(src) = source {
                if !src.matches(m.header.source) {
                    return false;
                }
            }
            // Check tier matches filter kind and comparison
            m.header
                .tier
                .as_ref()
                .and_then(|t| tier_matches_filter(t, kind))
                .is_some_and(|n| op.eval(&n, &value))
        })
        .count() as u32;
    count >= min_count
}

/// Iterate all non-reminder stat lines matching any of the given `stat_ids`.
/// Returns no matches if `stat_ids` is empty.
fn find_matching_stats<'a>(
    item: &'a ResolvedItem,
    stat_ids: &'a [String],
) -> impl Iterator<Item = &'a poe_item::types::ResolvedStatLine> {
    item.all_mods()
        .flat_map(|m| &m.stat_lines)
        .filter(move |sl| {
            !sl.is_reminder
                && !stat_ids.is_empty()
                && sl
                    .stat_ids
                    .as_ref()
                    .is_some_and(|ids| ids.iter().any(|id| stat_ids.iter().any(|sid| sid == id)))
        })
}
