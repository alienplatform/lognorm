//! Fast replay of checked output from pinned real logging libraries.
use std::str::FromStr;

use bigdecimal::BigDecimal;
use lognorm::{parse, ParseOutcome};
use serde_json::Value;

// Numbers compare by decimal value: serde_json's own spelling of a number
// depends on whether some crate in the build enabled arbitrary_precision.
fn same_json(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => {
            BigDecimal::from_str(&left.to_string()).unwrap()
                == BigDecimal::from_str(&right.to_string()).unwrap()
        }
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len() && left.iter().zip(right).all(|(a, b)| same_json(a, b))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .all(|(key, value)| right.get(key).is_some_and(|other| same_json(value, other)))
        }
        _ => left == right,
    }
}

#[test]
fn checked_real_library_records_retain_message_metadata_and_fields() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../compatibility/fixtures.json")).unwrap();
    assert_eq!(cases.len(), 43);
    for case in cases {
        let name = case["scenario"].as_str().unwrap();
        let input = case["input"].as_str().unwrap();
        let expected = &case["expected"];
        let log = parse(input);
        assert_eq!(log.original(), input, "{name}");
        assert_eq!(
            log.severity().map(|s| s.as_str()),
            expected["severity"].as_str(),
            "{name}"
        );
        assert_eq!(
            log.timestamp().is_some(),
            expected["timestamp"].as_bool().unwrap(),
            "{name}"
        );
        if expected["structured"] == true {
            assert_eq!(log.outcome(), ParseOutcome::Structured, "{name}");
            assert_eq!(
                log.message(),
                expected["message"].as_str().unwrap(),
                "{name}"
            );
            let fields = Value::Object(log.fields().clone());
            let source = serde_json::from_str::<Value>(input).unwrap();
            assert!(same_json(&fields, &source), "{name}: {fields} != {source}");
            let mut payload = &log.fields()[expected["payloadPath"][0].as_str().unwrap()];
            for key in expected["payloadPath"].as_array().unwrap().iter().skip(1) {
                payload = &payload[key.as_str().unwrap()];
            }
            if expected["payloadIsString"] == true {
                assert!(payload.as_str().unwrap().contains("statusCode"), "{name}");
            } else {
                assert_eq!(payload["nested"]["statusCode"], 401, "{name}");
                assert_eq!(payload["items"][0], true, "{name}");
                assert!(payload["items"][1].is_null(), "{name}");
                assert_eq!(payload["items"][3], "λ", "{name}");
            }
        } else {
            assert_eq!(log.outcome(), ParseOutcome::Text, "{name}");
            assert_eq!(log.message(), input, "{name}");
            assert!(log.fields().is_empty(), "{name}");
        }
    }
}
