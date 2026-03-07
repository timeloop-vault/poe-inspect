use serde::Deserialize;

/// A single condition in a reverse query.
///
/// Mirrors the Erlang RQE's `rq_item` protobuf message.
/// JSON format: `{ "key": "...", "value": ..., "type": "...", "typeOptions": ... }`
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(from = "RawCondition")]
pub struct Condition {
    pub key: String,
    pub value: Value,
}

/// Typed value with comparison semantics.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Boolean(bool),
    String(StringMatch),
    Integer { value: i64, op: CompareOp },
    List { op: ListOp, conditions: Vec<Condition> },
}

/// String matching mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringMatch {
    Exact(String),
    Wildcard,
}

/// Comparison operator for integer values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    Gt,
    Lt,
    Gte,
    Lte,
}

/// List composition operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListOp {
    And,
    Or,
    Not,
    Count(u32),
}

// --- Serde deserialization from Erlang-compatible JSON ---

#[derive(Deserialize)]
struct RawCondition {
    key: String,
    value: serde_json::Value,
    #[serde(rename = "type")]
    type_: String,
    #[serde(rename = "typeOptions")]
    type_options: Option<RawTypeOptions>,
}

#[derive(Deserialize)]
struct RawTypeOptions {
    operator: Option<String>,
    count: Option<u32>,
}

impl From<RawCondition> for Condition {
    fn from(raw: RawCondition) -> Self {
        let value = match raw.type_.as_str() {
            "boolean" => {
                let b = raw.value.as_bool().expect("boolean value expected");
                Value::Boolean(b)
            }
            "string" => {
                let s = raw.value.as_str().expect("string value expected");
                if s == "_" {
                    Value::String(StringMatch::Wildcard)
                } else {
                    Value::String(StringMatch::Exact(s.to_owned()))
                }
            }
            "integer" => {
                let v = raw.value.as_i64().expect("integer value expected");
                let op = match raw.type_options.as_ref().and_then(|o| o.operator.as_deref()) {
                    Some("<") => CompareOp::Lt,
                    Some(">") => CompareOp::Gt,
                    Some("<=") => CompareOp::Lte,
                    Some(">=") => CompareOp::Gte,
                    _ => CompareOp::Eq,
                };
                Value::Integer { value: v, op }
            }
            "list" => {
                let items: Vec<Condition> =
                    serde_json::from_value(raw.value).expect("list of conditions expected");
                let list_op = match raw.type_options.as_ref().and_then(|o| o.operator.as_deref()) {
                    Some("and") => ListOp::And,
                    Some("or") => ListOp::Or,
                    Some("not") => ListOp::Not,
                    Some("count") => {
                        let count = raw
                            .type_options
                            .as_ref()
                            .and_then(|o| o.count)
                            .expect("count value required for count operator");
                        ListOp::Count(count)
                    }
                    other => panic!("unknown list operator: {other:?}"),
                };
                Value::List {
                    op: list_op,
                    conditions: items,
                }
            }
            other => panic!("unknown condition type: {other}"),
        };
        Condition {
            key: raw.key,
            value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_string_condition() {
        let json = r#"{"key": "item_category", "value": "Crimson Jewel", "type": "string", "typeOptions": null}"#;
        let cond: Condition = serde_json::from_str(json).unwrap();
        assert_eq!(cond.key, "item_category");
        assert_eq!(cond.value, Value::String(StringMatch::Exact("Crimson Jewel".into())));
    }

    #[test]
    fn deserialize_wildcard() {
        let json = r#"{"key": "name", "value": "_", "type": "string", "typeOptions": null}"#;
        let cond: Condition = serde_json::from_str(json).unwrap();
        assert_eq!(cond.value, Value::String(StringMatch::Wildcard));
    }

    #[test]
    fn deserialize_integer_with_operator() {
        let json = r#"{"key": "armor", "value": 20, "type": "integer", "typeOptions": {"operator": ">"}}"#;
        let cond: Condition = serde_json::from_str(json).unwrap();
        assert_eq!(cond.value, Value::Integer { value: 20, op: CompareOp::Gt });
    }

    #[test]
    fn deserialize_boolean() {
        let json = r#"{"key": "rarity rare", "value": true, "type": "boolean", "typeOptions": null}"#;
        let cond: Condition = serde_json::from_str(json).unwrap();
        assert_eq!(cond.value, Value::Boolean(true));
    }

    #[test]
    fn deserialize_list_and() {
        let json = r#"{
            "key": "list",
            "value": [
                {"key": "armor", "value": 4, "type": "integer", "typeOptions": {"operator": "<"}},
                {"key": "armor", "value": 20, "type": "integer", "typeOptions": {"operator": ">"}}
            ],
            "type": "list",
            "typeOptions": {"operator": "and"}
        }"#;
        let cond: Condition = serde_json::from_str(json).unwrap();
        match &cond.value {
            Value::List { op, conditions } => {
                assert_eq!(*op, ListOp::And);
                assert_eq!(conditions.len(), 2);
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn deserialize_list_count() {
        let json = r#"{
            "key": "list",
            "value": [
                {"key": "stat_a", "value": 10, "type": "integer", "typeOptions": {"operator": ">"}}
            ],
            "type": "list",
            "typeOptions": {"operator": "count", "count": 1}
        }"#;
        let cond: Condition = serde_json::from_str(json).unwrap();
        match &cond.value {
            Value::List { op, conditions } => {
                assert_eq!(*op, ListOp::Count(1));
                assert_eq!(conditions.len(), 1);
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn deserialize_full_rq_file() {
        let path = format!(
            "{}/_reference/rqe/test/data/rq/wanted_crimson_mod.json",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
        );
        let json = std::fs::read_to_string(&path).expect("test data file should exist at _reference/rqe/");
        let conditions: Vec<Condition> = serde_json::from_str(&json).unwrap();
        assert_eq!(conditions.len(), 3);
    }
}
