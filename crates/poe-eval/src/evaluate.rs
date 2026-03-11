//! Evaluation engine — tests rules against resolved items.
//!
//! This module contains zero `PoE` domain knowledge. All game-specific
//! lookups go through `GameData`. The evaluator is pure logic.

use poe_data::GameData;
use poe_item::types::{ModSlot, ModTierKind, ResolvedItem};

use crate::predicate::{Cmp, ModSlotKind, Predicate, RarityValue, StatCondition};
use crate::profile::{MatchedRule, Profile, ScoreResult, UnmatchedRule};
use crate::rule::Rule;

impl ModSlotKind {
    fn to_mod_slot(self) -> ModSlot {
        match self {
            Self::Prefix => ModSlot::Prefix,
            Self::Suffix => ModSlot::Suffix,
            Self::Implicit => ModSlot::Implicit,
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
            let count = count_mods_in_slot(item, slot.to_mod_slot());
            op.eval(&count, value)
        }

        Predicate::OpenMods { slot, op, value } => {
            let open = open_mod_count(item, *slot, gd);
            op.eval(&open, value)
        }

        Predicate::HasModNamed { name } => item
            .all_mods()
            .any(|m| m.header.name.as_deref() == Some(name.as_str())),

        Predicate::HasStatText { text } => item.all_mods().any(|m| {
            m.stat_lines
                .iter()
                .any(|sl| !sl.is_reminder && sl.display_text.contains(text.as_str()))
        }),

        Predicate::HasStatId { stat_id } => item.all_mods().any(|m| {
            m.stat_lines.iter().any(|sl| {
                sl.stat_ids
                    .as_ref()
                    .is_some_and(|ids| ids.iter().any(|id| id == stat_id))
            })
        }),

        Predicate::ModTier { name, op, value } => item.all_mods().any(|m| {
            m.header.name.as_deref() == Some(name.as_str())
                && m.header.tier.as_ref().is_some_and(|t| match t {
                    ModTierKind::Tier(n) | ModTierKind::Rank(n) => op.eval(n, value),
                })
        }),

        // ── Stat value predicates ────────────────────────────────────
        Predicate::StatValue { conditions } => eval_stat_value(item, conditions),

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
            let count = item
                .sockets
                .as_deref()
                .map_or(0, |s| count_sockets(s));
            op.eval(&count, value)
        }

        Predicate::LinkCount { op, value } => {
            let max_link = item
                .sockets
                .as_deref()
                .map_or(0, |s| max_link_group(s));
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
                        ids.iter()
                            .any(|id| c.stat_ids.iter().any(|sid| sid == id))
                    })
                    && sl
                        .values
                        .get(c.value_index)
                        .is_some_and(|v| c.op.eval(&v.current, &c.value))
            })
        })
    })
}

/// Count mods in a given slot.
fn count_mods_in_slot(item: &ResolvedItem, slot: ModSlot) -> u32 {
    item.all_mods().filter(|m| m.header.slot == slot).count() as u32
}

/// Calculate open mod slots. Returns 0 if we can't determine the max
/// (e.g., missing game data or non-applicable rarity).
fn open_mod_count(item: &ResolvedItem, slot: ModSlotKind, gd: &GameData) -> u32 {
    let rarity_str = format!("{:?}", item.header.rarity);
    let Some(rarity_id) = poe_data::domain::rarity_to_ggpk_id(&rarity_str) else {
        return 0;
    };

    let max = match slot {
        ModSlotKind::Prefix => gd.max_prefixes(rarity_id).unwrap_or(0),
        ModSlotKind::Suffix => gd.max_suffixes(rarity_id).unwrap_or(0),
        ModSlotKind::Implicit => return 0, // Implicit count isn't bounded by rarity
    };

    let mod_slot = slot.to_mod_slot();

    let current = count_mods_in_slot(item, mod_slot);
    let max_u32 = u32::try_from(max).unwrap_or(0);
    max_u32.saturating_sub(current)
}

/// Count total sockets from a socket string like `"R-R-G B"`.
/// Letters are sockets; `-` = linked, ` ` = new group.
fn count_sockets(sockets: &str) -> u32 {
    sockets.chars().filter(|c| c.is_ascii_alphabetic()).count() as u32
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
                && sl.stat_ids.as_ref().is_some_and(|ids| {
                    ids.iter().any(|id| stat_ids.iter().any(|sid| sid == id))
                })
        })
}
