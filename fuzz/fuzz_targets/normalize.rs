#![no_main]

use libfuzzer_sys::fuzz_target;
use prost::Message;
use serde_json::Value;

// lognorm keeps a number as its literal string only when serde_json's number
// type cannot hold it exactly. That literal must appear verbatim in the input
// and decode to the same number. Everything else must match exactly.
fn consistent(input: &str, ours: &Value, independent: &Value) -> bool {
    match (ours, independent) {
        (Value::String(literal), Value::Number(number)) => {
            input.contains(literal.as_str())
                && serde_json::from_str::<Value>(literal).ok()
                    == Some(Value::Number(number.clone()))
        }
        (Value::Array(ours), Value::Array(independent)) => {
            ours.len() == independent.len()
                && ours.iter().zip(independent).all(|(a, b)| consistent(input, a, b))
        }
        (Value::Object(ours), Value::Object(independent)) => {
            ours.len() == independent.len()
                && ours
                    .iter()
                    .all(|(key, value)| independent.get(key).is_some_and(|other| consistent(input, value, other)))
        }
        _ => ours == independent,
    }
}

fuzz_target!(|input: &str| {
    let parsed = lognorm::parse(input);
    assert_eq!(parsed.original(), input);
    if parsed.outcome() == lognorm::ParseOutcome::Structured {
        let independently_decoded: Value = serde_json::from_str(input).unwrap();
        assert!(consistent(
            input,
            &Value::Object(parsed.fields().clone()),
            &independently_decoded
        ));
    } else {
        assert!(parsed.fields().is_empty());
        assert!(parsed.severity().is_none());
        assert!(parsed.timestamp().is_none());
    }
    let record = lognorm::otlp::log_record(&parsed);
    assert_eq!(record.severity_number == 0, parsed.severity().is_none());
    let _encoded = record.encode_to_vec();
});
