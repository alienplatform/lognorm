#![no_main]

use libfuzzer_sys::fuzz_target;
use prost::Message;

fuzz_target!(|input: &str| {
    let parsed = lognorm::parse(input);
    assert_eq!(parsed.original(), input);
    if parsed.outcome() == lognorm::ParseOutcome::Structured {
        let independently_decoded: serde_json::Value = serde_json::from_str(input).unwrap();
        assert_eq!(
            serde_json::Value::Object(parsed.fields().clone()),
            independently_decoded
        );
    } else {
        assert!(parsed.fields().is_empty());
        assert!(parsed.severity().is_none());
        assert!(parsed.timestamp().is_none());
    }
    let record = lognorm::otlp::log_record(&parsed);
    assert_eq!(record.severity_number == 0, parsed.severity().is_none());
    let _encoded = record.encode_to_vec();
});
