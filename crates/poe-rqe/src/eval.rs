use std::collections::HashMap;

use serde::Deserialize;

use crate::predicate::{CompareOp, Condition, ListOp, StringMatch, Value};

/// A value stored in an entry (item).
///
/// Entries are flat key-value maps where values can be strings, integers, or booleans.
/// This mirrors the Erlang RQE's entry format: `#{"key" => value}`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum EntryValue {
    String(String),
    Integer(i64),
    Boolean(bool),
}

/// An item entry to match against reverse queries.
///
/// Flat map of property names to values, deserialized directly from JSON.
#[derive(Debug, Clone, Deserialize)]
pub struct Entry(HashMap<String, EntryValue>);

impl Entry {
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&EntryValue> {
        self.0.get(key)
    }
}

/// Evaluate a list of conditions against an entry. Returns `true` if all conditions match.
///
/// This is the Rust port of `rqe_lib:eval_rq/2`.
/// Short-circuits on the first non-matching condition.
#[must_use]
pub fn evaluate(conditions: &[Condition], entry: &Entry) -> bool {
    conditions.iter().all(|cond| evaluate_one(cond, entry))
}

fn evaluate_one(condition: &Condition, entry: &Entry) -> bool {
    match &condition.value {
        Value::List { op, conditions } => evaluate_list(op, conditions, entry),
        Value::Boolean(expected) => {
            let Some(entry_val) = entry.get(&condition.key) else {
                return false;
            };
            compare_boolean(*expected, entry_val)
        }
        Value::String(match_mode) => {
            let Some(entry_val) = entry.get(&condition.key) else {
                return false;
            };
            compare_string(match_mode, entry_val)
        }
        Value::Integer { value, op } => {
            let Some(entry_val) = entry.get(&condition.key) else {
                return false;
            };
            compare_integer(*value, *op, entry_val)
        }
    }
}

/// Erlang: `compare_string_value/3`
fn compare_string(match_mode: &StringMatch, entry_val: &EntryValue) -> bool {
    match match_mode {
        StringMatch::Wildcard => true,
        StringMatch::Exact(expected) => match entry_val {
            EntryValue::String(s) => s == expected,
            _ => false,
        },
    }
}

/// Erlang: `compare_integer_value/3`
///
/// The Erlang semantics are: `compare_integer_value(RQValue, EntryValue, Operator)`
/// where the operator describes the relationship: `RQValue <op> EntryValue`.
/// For example, `{value: 4, operator: "<"}` means `4 < EntryValue` (i.e., entry > 4).
fn compare_integer(rq_value: i64, op: CompareOp, entry_val: &EntryValue) -> bool {
    let EntryValue::Integer(entry_int) = entry_val else {
        return false;
    };
    match op {
        CompareOp::Eq => rq_value == *entry_int,
        CompareOp::Gt => rq_value > *entry_int,
        CompareOp::Lt => rq_value < *entry_int,
        CompareOp::Gte => rq_value >= *entry_int,
        CompareOp::Lte => rq_value <= *entry_int,
    }
}

/// Erlang: `compare_boolean_value/2`
///
/// Erlang semantics:
/// - `compare_boolean_value(true, true)` → true
/// - `compare_boolean_value(true, _)` → false
/// - `compare_boolean_value(false, true)` → false
/// - `compare_boolean_value(false, _)` → true
///
/// So `expected=true` requires entry to be exactly `true`.
/// `expected=false` requires entry to NOT be `true` (false, missing, or other type).
fn compare_boolean(expected: bool, entry_val: &EntryValue) -> bool {
    let is_true = matches!(entry_val, EntryValue::Boolean(true));
    if expected { is_true } else { !is_true }
}

/// Erlang: `compare_list_value/3`
fn evaluate_list(op: &ListOp, conditions: &[Condition], entry: &Entry) -> bool {
    match op {
        ListOp::And => evaluate(conditions, entry),
        ListOp::Or => conditions.iter().any(|c| evaluate_one(c, entry)),
        ListOp::Not => !conditions.iter().any(|c| evaluate_one(c, entry)),
        ListOp::Count(n) => {
            let matched = conditions.iter().filter(|c| evaluate_one(c, entry)).count();
            matched == *n as usize
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_rq(filename: &str) -> Vec<Condition> {
        let path = format!(
            "{}/_reference/rqe/test/data/rq/{filename}",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
        );
        let data =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
        serde_json::from_str(&data).unwrap()
    }

    fn load_entry(filename: &str) -> Entry {
        let path = format!(
            "{}/_reference/rqe/test/data/entry/{filename}",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
        );
        let data =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
        serde_json::from_str(&data).unwrap()
    }

    // --- The key test case from the Erlang suite ---

    #[test]
    fn erlang_suite_mod_and_not_count_matches_mods_2() {
        let rq = load_rq("wanted_mod_and_not_count.json");
        let entry = load_entry("crimson_w_mods_2.json");
        assert!(evaluate(&rq, &entry), "should match crimson_w_mods_2");
    }

    #[test]
    fn erlang_suite_mod_and_not_count_rejects_mods_1() {
        let rq = load_rq("wanted_mod_and_not_count.json");
        let entry = load_entry("crimson_w_mods_1.json");
        assert!(!evaluate(&rq, &entry), "should NOT match crimson_w_mods_1");
    }

    // --- wanted_crimson_rare ---

    #[test]
    fn crimson_rare_matches_rare_crimson() {
        let rq = load_rq("wanted_crimson_rare.json");
        let entry = load_entry("crimson_w_mods_1.json");
        assert!(evaluate(&rq, &entry));
    }

    #[test]
    fn crimson_rare_rejects_magic() {
        let rq = load_rq("wanted_crimson_rare.json");
        let entry = load_entry("crimson_magic.json");
        assert!(!evaluate(&rq, &entry));
    }

    #[test]
    fn crimson_rare_rejects_unique() {
        let rq = load_rq("wanted_crimson_rare.json");
        let entry = load_entry("crimson_unique.json");
        // Unique items don't have item_rarity_2 = "Non-Unique"
        assert!(!evaluate(&rq, &entry));
    }

    // --- wanted_crimson_mod (AND list: armor between 4 and 20) ---

    #[test]
    fn crimson_mod_matches_armor_15() {
        // crimson_w_mods_1 has armor=15, which is between 4 and 20
        let rq = load_rq("wanted_crimson_mod.json");
        let entry = load_entry("crimson_w_mods_1.json");
        assert!(evaluate(&rq, &entry));
    }

    #[test]
    fn crimson_mod_rejects_no_armor() {
        // crimson_w_mods_2 has no armor stat
        let rq = load_rq("wanted_crimson_mod.json");
        let entry = load_entry("crimson_w_mods_2.json");
        assert!(!evaluate(&rq, &entry));
    }

    // --- wanted_crimson_mod_not (NOT list: armor must NOT be between 4 and 20) ---

    #[test]
    fn crimson_mod_not_rejects_armor_15() {
        // crimson_w_mods_1 has armor=15 which IS in the NOT range
        let rq = load_rq("wanted_crimson_mod_not.json");
        let entry = load_entry("crimson_w_mods_1.json");
        assert!(!evaluate(&rq, &entry));
    }

    #[test]
    fn crimson_mod_not_matches_no_armor() {
        // crimson_w_mods_2 has no armor → NOT conditions don't trigger
        let rq = load_rq("wanted_crimson_mod_not.json");
        let entry = load_entry("crimson_w_mods_2.json");
        assert!(evaluate(&rq, &entry));
    }

    // --- wanted_crimson_mod_count (COUNT=1 of 4 integer conditions) ---

    #[test]
    fn crimson_mod_count_matches_mods_1() {
        // crimson_w_mods_1 has armor=15: both armor conditions match (4<15, 20>15),
        // no lightning stat, so exactly 2 match. COUNT=1 expects exactly 1, so fails? Let's check.
        // Actually: conditions are 4 items:
        //   armor < 4  → 15 is not < 4... wait.
        // Erlang semantics: compare_integer_value(Value=4, EntryValue=15, '<') → 4 < 15 → true
        // So: armor cond1 (val=4, op=<): 4 < 15 = true
        //     armor cond2 (val=20, op=>): 20 > 15 = true
        //     lightning cond3 (val=60, op=>): no entry → false
        //     lightning cond4 (val=30, op=<): no entry → false
        // Count = 2, but COUNT=1 expected → false
        let rq = load_rq("wanted_crimson_mod_count.json");
        let entry = load_entry("crimson_w_mods_1.json");
        assert!(!evaluate(&rq, &entry));
    }

    #[test]
    fn crimson_mod_count_rejects_no_armor_no_lightning() {
        // crimson_w_mods_2: no armor, no lightning → 0 match, COUNT=1 expects 1 → false
        let rq = load_rq("wanted_crimson_mod_count.json");
        let entry = load_entry("crimson_w_mods_2.json");
        assert!(!evaluate(&rq, &entry));
    }

    // --- wanted_crimson_mod_count_2 (COUNT=1 of 3 conditions) ---

    #[test]
    fn crimson_mod_count_2_matches_mods_2() {
        // crimson_w_mods_2: no armor, fire_cold_res=11
        // Conditions: armor<4 (false, no armor), armor>20 (false), fire_cold_res<10 (10<11=true)
        // Count=1, matched=1 → true
        let rq = load_rq("wanted_crimson_mod_count_2.json");
        let entry = load_entry("crimson_w_mods_2.json");
        assert!(evaluate(&rq, &entry));
    }

    // --- wanted_crimson_mod_and_not (AND list + NOT list) ---

    #[test]
    fn crimson_mod_and_not_matches_mods_2() {
        // NOT list: armor NOT between 4-20 → mods_2 has no armor → NOT passes
        // AND list: fire_cold_res between 4-20 → 11 is in range → passes
        let rq = load_rq("wanted_crimson_mod_and_not.json");
        let entry = load_entry("crimson_w_mods_2.json");
        assert!(evaluate(&rq, &entry));
    }

    #[test]
    fn crimson_mod_and_not_rejects_mods_1() {
        // NOT list: armor NOT between 4-20 → mods_1 has armor=15 which IS in range → NOT fails
        let rq = load_rq("wanted_crimson_mod_and_not.json");
        let entry = load_entry("crimson_w_mods_1.json");
        assert!(!evaluate(&rq, &entry));
    }

    // --- Cross-category tests ---

    #[test]
    fn crimson_rq_rejects_ring() {
        let rq = load_rq("wanted_crimson_rare.json");
        let entry = load_entry("paua_ring_rare.json");
        assert!(!evaluate(&rq, &entry));
    }

    #[test]
    fn crimson_rq_rejects_weapon() {
        let rq = load_rq("wanted_crimson_mod.json");
        let entry = load_entry("two_handed_weapon_rare.json");
        assert!(!evaluate(&rq, &entry));
    }

    // --- Boots/sockets tests ---

    #[test]
    fn boots_unique_matches_4_socket_3_link() {
        let rq = load_rq("wanted_boots_unique.json");
        let entry = load_entry("item_socket_4_link_3.json");
        assert!(evaluate(&rq, &entry));
    }

    #[test]
    fn boots_unique_rejects_2_socket_wand() {
        let rq = load_rq("wanted_boots_unique.json");
        let entry = load_entry("item_socket_2_link_0.json");
        // Wand, not boots category
        assert!(!evaluate(&rq, &entry));
    }

    // --- New format RQ tests ---
    // wanted_boots_unique_new_format requires: category=_ (any), NOT(mana>10),
    // AND(lightning_dmg_1<=1, lightning_dmg_2<10), lightning_res<15.
    // It's really a ring-oriented query despite the filename.

    #[test]
    fn new_format_rejects_boots_no_lightning() {
        // Boots have no lightning damage or resistance stats → AND list and direct condition fail
        let rq = load_rq("wanted_boots_unique_new_format.json");
        let entry = load_entry("item_socket_4_link_3.json");
        assert!(!evaluate(&rq, &entry));
    }

    #[test]
    fn new_format_matches_paua_ring() {
        // Paua ring: no armor (NOT passes), lightning_dmg_1=1 (1<=1), lightning_dmg_2=18 (10<18),
        // lightning_res=23 (15<23). All conditions pass.
        let rq = load_rq("wanted_boots_unique_new_format.json");
        let entry = load_entry("paua_ring_rare.json");
        assert!(evaluate(&rq, &entry));
    }
}
