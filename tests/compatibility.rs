//! Fast replay of checked output from pinned real logging libraries.
use lognorm::{parse, ParseOutcome};
use serde_json::Value;

#[test]
fn checked_real_library_records_retain_message_metadata_and_fields() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../compatibility/fixtures.json")).unwrap();
    assert_eq!(cases.len(), 35);
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
            assert_eq!(
                Value::Object(log.fields().clone()),
                serde_json::from_str::<Value>(input).unwrap(),
                "{name}"
            );
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
