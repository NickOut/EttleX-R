//! Canonical JSON serialisation with deterministic key ordering.
//!
//! All MCP responses are serialised through `canonical_json` so that
//! byte-identical inputs produce byte-identical outputs regardless of
//! the original key-insertion order.

use serde_json::Value;
use std::collections::BTreeMap;

/// Recursively sort all JSON object keys alphabetically, producing a
/// canonical representation that is byte-identical for equivalent values.
pub fn canonical_json(value: &Value) -> String {
    let ordered = to_ordered(value);
    serde_json::to_string(&ordered).unwrap_or_default()
}

fn to_ordered(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let ordered: BTreeMap<String, Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), to_ordered(v)))
                .collect();
            Value::Object(ordered.into_iter().collect())
        }
        Value::Array(arr) => Value::Array(arr.iter().map(to_ordered).collect()),
        other => other.clone(),
    }
}
