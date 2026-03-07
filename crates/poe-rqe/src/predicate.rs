use serde::{Deserialize, Serialize, Serializer, ser::SerializeMap};

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

// --- Serde serialization to Erlang-compatible JSON ---

impl Serialize for Condition {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("key", &self.key)?;
        match &self.value {
            Value::Boolean(b) => {
                map.serialize_entry("value", b)?;
                map.serialize_entry("type", "boolean")?;
                map.serialize_entry("typeOptions", &None::<()>)?;
            }
            Value::String(StringMatch::Wildcard) => {
                map.serialize_entry("value", "_")?;
                map.serialize_entry("type", "string")?;
                map.serialize_entry("typeOptions", &None::<()>)?;
            }
            Value::String(StringMatch::Exact(s)) => {
                map.serialize_entry("value", s)?;
                map.serialize_entry("type", "string")?;
                map.serialize_entry("typeOptions", &None::<()>)?;
            }
            Value::Integer { value, op } => {
                map.serialize_entry("value", value)?;
                map.serialize_entry("type", "integer")?;
                let op_str = match op {
                    CompareOp::Eq => None,
                    CompareOp::Gt => Some(">"),
                    CompareOp::Lt => Some("<"),
                    CompareOp::Gte => Some(">="),
                    CompareOp::Lte => Some("<="),
                };
                match op_str {
                    Some(o) => map.serialize_entry("typeOptions", &TypeOptionsOut::Op(o))?,
                    None => map.serialize_entry("typeOptions", &None::<()>)?,
                }
            }
            Value::List { op, conditions } => {
                map.serialize_entry("value", conditions)?;
                map.serialize_entry("type", "list")?;
                let type_options = match op {
                    ListOp::And => TypeOptionsOut::Op("and"),
                    ListOp::Or => TypeOptionsOut::Op("or"),
                    ListOp::Not => TypeOptionsOut::Op("not"),
                    ListOp::Count(n) => TypeOptionsOut::Count("count", *n),
                };
                map.serialize_entry("typeOptions", &type_options)?;
            }
        }
        map.end()
    }
}

enum TypeOptionsOut<'a> {
    Op(&'a str),
    Count(&'a str, u32),
}

impl Serialize for TypeOptionsOut<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        match self {
            Self::Op(op) => {
                map.serialize_entry("operator", op)?;
            }
            Self::Count(op, count) => {
                map.serialize_entry("operator", op)?;
                map.serialize_entry("count", count)?;
            }
        }
        map.end()
    }
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
    fn round_trip_all_rq_files() {
        let rq_files = [
            "wanted_crimson_rare.json",
            "wanted_crimson_mod.json",
            "wanted_crimson_mod_not.json",
            "wanted_crimson_mod_count.json",
            "wanted_crimson_mod_count_2.json",
            "wanted_crimson_mod_and_not.json",
            "wanted_mod_and_not_count.json",
            "wanted_boots_unique.json",
            "wanted_boots_unique_new_format.json",
        ];
        for file in &rq_files {
            let path = format!(
                "{}/_reference/rqe/test/data/rq/{file}",
                concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
            );
            let json = std::fs::read_to_string(&path).unwrap();
            let conditions: Vec<Condition> = serde_json::from_str(&json).unwrap();
            // Serialize back to JSON and deserialize again
            let serialized = serde_json::to_string(&conditions).unwrap();
            let round_tripped: Vec<Condition> = serde_json::from_str(&serialized).unwrap();
            assert_eq!(conditions, round_tripped, "round-trip failed for {file}");
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
