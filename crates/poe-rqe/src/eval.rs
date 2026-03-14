use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::predicate::{CompareOp, Condition, ListOp, StringMatch, Value};

/// A value stored in an entry (item).
///
/// Entries are flat key-value maps where values can be strings, integers, or booleans.
/// This mirrors the Erlang RQE's entry format: `#{"key" => value}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EntryValue {
    String(String),
    Integer(i64),
    Boolean(bool),
}

/// An item entry to match against reverse queries.
///
/// Flat map of property names to values, deserialized directly from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

pub fn evaluate_one(condition: &Condition, entry: &Entry) -> bool {
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

    // --- Product marketplace entries ---

    fn electronics_entry() -> Entry {
        serde_json::from_str(
            r#"{"category": "Electronics", "in_stock": true, "on_sale": false, "price": 299, "weight": 2, "rating": 4, "color": "Black"}"#,
        )
        .unwrap()
    }

    fn clothing_entry() -> Entry {
        serde_json::from_str(
            r#"{"category": "Clothing", "in_stock": true, "on_sale": true, "price": 49, "weight": 1, "rating": 5, "color": "Red"}"#,
        )
        .unwrap()
    }

    fn book_entry() -> Entry {
        serde_json::from_str(
            r#"{"category": "Books", "in_stock": false, "on_sale": false, "price": 15, "weight": 1, "rating": 3}"#,
        )
        .unwrap()
    }

    // --- String matching ---

    #[test]
    fn string_exact_match() {
        let rq: Vec<Condition> = serde_json::from_str(
            r#"[{"key": "category", "value": "Electronics", "type": "string", "typeOptions": null}]"#,
        ).unwrap();
        assert!(evaluate(&rq, &electronics_entry()));
    }

    #[test]
    fn string_exact_mismatch() {
        let rq: Vec<Condition> = serde_json::from_str(
            r#"[{"key": "category", "value": "Electronics", "type": "string", "typeOptions": null}]"#,
        ).unwrap();
        assert!(!evaluate(&rq, &clothing_entry()));
    }

    #[test]
    fn wildcard_matches_any_value() {
        let rq: Vec<Condition> = serde_json::from_str(
            r#"[{"key": "category", "value": "_", "type": "string", "typeOptions": null}]"#,
        )
        .unwrap();
        assert!(evaluate(&rq, &electronics_entry()));
        assert!(evaluate(&rq, &clothing_entry()));
        assert!(evaluate(&rq, &book_entry()));
    }

    // --- Boolean semantics ---

    #[test]
    fn boolean_true_matches_true() {
        let rq: Vec<Condition> = serde_json::from_str(
            r#"[{"key": "in_stock", "value": true, "type": "boolean", "typeOptions": null}]"#,
        )
        .unwrap();
        assert!(evaluate(&rq, &electronics_entry())); // in_stock=true
    }

    #[test]
    fn boolean_true_rejects_false() {
        let rq: Vec<Condition> = serde_json::from_str(
            r#"[{"key": "in_stock", "value": true, "type": "boolean", "typeOptions": null}]"#,
        )
        .unwrap();
        assert!(!evaluate(&rq, &book_entry())); // in_stock=false
    }

    #[test]
    fn boolean_false_matches_non_true() {
        // Erlang semantics: expected=false matches anything that isn't true
        let rq: Vec<Condition> = serde_json::from_str(
            r#"[{"key": "on_sale", "value": false, "type": "boolean", "typeOptions": null}]"#,
        )
        .unwrap();
        assert!(evaluate(&rq, &electronics_entry())); // on_sale=false
        assert!(evaluate(&rq, &book_entry())); // on_sale=false
    }

    #[test]
    fn boolean_false_rejects_true() {
        let rq: Vec<Condition> = serde_json::from_str(
            r#"[{"key": "on_sale", "value": false, "type": "boolean", "typeOptions": null}]"#,
        )
        .unwrap();
        assert!(!evaluate(&rq, &clothing_entry())); // on_sale=true
    }

    // --- Integer operators (Erlang semantics: rq_value <op> entry_value) ---

    #[test]
    fn integer_lt() {
        // rq_value=100 < entry_value → entry must be > 100
        let rq: Vec<Condition> = serde_json::from_str(
            r#"[{"key": "price", "value": 100, "type": "integer", "typeOptions": {"operator": "<"}}]"#,
        ).unwrap();
        assert!(evaluate(&rq, &electronics_entry())); // 100 < 299
        assert!(!evaluate(&rq, &book_entry())); // 100 < 15 = false
    }

    #[test]
    fn integer_gt() {
        // rq_value=100 > entry_value → entry must be < 100
        let rq: Vec<Condition> = serde_json::from_str(
            r#"[{"key": "price", "value": 100, "type": "integer", "typeOptions": {"operator": ">"}}]"#,
        ).unwrap();
        assert!(evaluate(&rq, &clothing_entry())); // 100 > 49
        assert!(!evaluate(&rq, &electronics_entry())); // 100 > 299 = false
    }

    #[test]
    fn integer_lte() {
        let rq: Vec<Condition> = serde_json::from_str(
            r#"[{"key": "rating", "value": 4, "type": "integer", "typeOptions": {"operator": "<="}}]"#,
        ).unwrap();
        assert!(evaluate(&rq, &electronics_entry())); // 4 <= 4
        assert!(evaluate(&rq, &clothing_entry())); // 4 <= 5
        assert!(!evaluate(&rq, &book_entry())); // 4 <= 3 = false
    }

    #[test]
    fn integer_gte() {
        let rq: Vec<Condition> = serde_json::from_str(
            r#"[{"key": "rating", "value": 4, "type": "integer", "typeOptions": {"operator": ">="}}]"#,
        ).unwrap();
        assert!(evaluate(&rq, &electronics_entry())); // 4 >= 4
        assert!(!evaluate(&rq, &clothing_entry())); // 4 >= 5 = false
        assert!(evaluate(&rq, &book_entry())); // 4 >= 3
    }

    #[test]
    fn integer_eq() {
        let rq: Vec<Condition> = serde_json::from_str(
            r#"[{"key": "weight", "value": 1, "type": "integer", "typeOptions": null}]"#,
        )
        .unwrap();
        assert!(!evaluate(&rq, &electronics_entry())); // 1 == 2 = false
        assert!(evaluate(&rq, &clothing_entry())); // 1 == 1
    }

    // --- Missing key fails ---

    #[test]
    fn missing_key_fails() {
        let rq: Vec<Condition> = serde_json::from_str(
            r#"[{"key": "nonexistent", "value": "anything", "type": "string", "typeOptions": null}]"#,
        ).unwrap();
        assert!(!evaluate(&rq, &electronics_entry()));
    }

    // --- AND list ---

    #[test]
    fn and_list_all_pass() {
        // price between 10 and 500 (Erlang: 10 < entry AND 500 > entry)
        let rq: Vec<Condition> = serde_json::from_str(
            r#"[{"key": "list", "value": [
                {"key": "price", "value": 10, "type": "integer", "typeOptions": {"operator": "<"}},
                {"key": "price", "value": 500, "type": "integer", "typeOptions": {"operator": ">"}}
            ], "type": "list", "typeOptions": {"operator": "and"}}]"#,
        )
        .unwrap();
        assert!(evaluate(&rq, &electronics_entry())); // 10 < 299 AND 500 > 299
    }

    #[test]
    fn and_list_partial_fail() {
        // price between 10 and 40 — electronics at 299 fails the upper bound
        let rq: Vec<Condition> = serde_json::from_str(
            r#"[{"key": "list", "value": [
                {"key": "price", "value": 10, "type": "integer", "typeOptions": {"operator": "<"}},
                {"key": "price", "value": 40, "type": "integer", "typeOptions": {"operator": ">"}}
            ], "type": "list", "typeOptions": {"operator": "and"}}]"#,
        )
        .unwrap();
        assert!(!evaluate(&rq, &electronics_entry())); // 40 > 299 = false
    }

    // --- NOT list ---

    #[test]
    fn not_list_passes_when_none_match() {
        // NOT(on_sale=true) — electronics has on_sale=false, so NOT passes
        let rq: Vec<Condition> = serde_json::from_str(
            r#"[{"key": "list", "value": [
                {"key": "on_sale", "value": true, "type": "boolean", "typeOptions": null}
            ], "type": "list", "typeOptions": {"operator": "not"}}]"#,
        )
        .unwrap();
        assert!(evaluate(&rq, &electronics_entry()));
    }

    #[test]
    fn not_list_fails_when_any_match() {
        // NOT(on_sale=true) — clothing has on_sale=true, so NOT fails
        let rq: Vec<Condition> = serde_json::from_str(
            r#"[{"key": "list", "value": [
                {"key": "on_sale", "value": true, "type": "boolean", "typeOptions": null}
            ], "type": "list", "typeOptions": {"operator": "not"}}]"#,
        )
        .unwrap();
        assert!(!evaluate(&rq, &clothing_entry()));
    }

    // --- COUNT list ---

    #[test]
    fn count_list_exact_match() {
        // COUNT=1: exactly 1 of these conditions must match
        // price < 50 (50 < entry), weight < 2 (2 < entry)
        // Electronics: price=299 (50<299=true), weight=2 (2<2=false) → count=1 = true
        let rq: Vec<Condition> = serde_json::from_str(
            r#"[{"key": "list", "value": [
                {"key": "price", "value": 50, "type": "integer", "typeOptions": {"operator": "<"}},
                {"key": "weight", "value": 2, "type": "integer", "typeOptions": {"operator": "<"}}
            ], "type": "list", "typeOptions": {"operator": "count", "count": 1}}]"#,
        )
        .unwrap();
        assert!(evaluate(&rq, &electronics_entry()));
    }

    #[test]
    fn count_list_wrong_count() {
        // COUNT=1 but both match for clothing: price=49 (50<49=false), weight=1 (2<1=false) → count=0
        let rq: Vec<Condition> = serde_json::from_str(
            r#"[{"key": "list", "value": [
                {"key": "price", "value": 50, "type": "integer", "typeOptions": {"operator": "<"}},
                {"key": "weight", "value": 2, "type": "integer", "typeOptions": {"operator": "<"}}
            ], "type": "list", "typeOptions": {"operator": "count", "count": 1}}]"#,
        )
        .unwrap();
        assert!(!evaluate(&rq, &clothing_entry())); // count=0, expected 1
    }

    // --- Cross-category rejection ---

    #[test]
    fn cross_category_rejection() {
        let rq: Vec<Condition> = serde_json::from_str(
            r#"[
                {"key": "category", "value": "Electronics", "type": "string", "typeOptions": null},
                {"key": "in_stock", "value": true, "type": "boolean", "typeOptions": null}
            ]"#,
        )
        .unwrap();
        assert!(!evaluate(&rq, &book_entry())); // Books != Electronics
    }
}
