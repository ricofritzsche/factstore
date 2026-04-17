use serde_json::Value;

pub(crate) fn payload_subset_matches(payload_predicate: &Value, payload: &Value) -> bool {
    match (payload_predicate, payload) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::Number(left), Value::Number(right)) => left == right,
        (Value::String(left), Value::String(right)) => left == right,
        (Value::Object(left), Value::Object(right)) => left.iter().all(|(key, left_value)| {
            right
                .get(key)
                .is_some_and(|right_value| payload_subset_matches(left_value, right_value))
        }),
        (Value::Array(left), Value::Array(right)) => left.iter().all(|left_value| {
            right
                .iter()
                .any(|right_value| payload_subset_matches(left_value, right_value))
        }),
        _ => false,
    }
}
