//! Behavioral and generative normalization regressions.
use lognorm::{parse, ParseOutcome, Parser, Severity, Timestamp};
use proptest::prelude::*;
use serde_json::{json, Value};

#[test]
fn preserves_exact_numbers_and_all_application_fields() {
    let input = r#" {"msg":"ready","big":184467440737095516160,"tiny":0.123456789012345678901,"nested":{"items":[true,null,1,"λ"]},"body":"{\"msg\":\"inner\"}"} "#;
    let log = parse(input);
    assert_eq!(log.original(), input);
    assert_eq!(log.message(), "ready");
    assert_eq!(log.fields()["big"].to_string(), "184467440737095516160");
    assert_eq!(log.fields()["tiny"].to_string(), "0.123456789012345678901");
    assert_eq!(
        Value::Object(log.fields().clone()),
        serde_json::from_str::<Value>(input).unwrap()
    );
}

#[test]
fn rejects_duplicate_keys_including_escaped_and_nested_keys() {
    for input in [
        r#"{"msg":"a","msg":"b"}"#,
        r#"{"nested":[{"x":1,"\u0078":2}]}"#,
    ] {
        let log = parse(input);
        assert_eq!(log.outcome(), ParseOutcome::DuplicateKeys, "{input}");
        assert_eq!(log.original(), input);
        assert!(log.fields().is_empty());
    }
}

#[test]
fn malformed_or_trailing_data_never_exposes_partial_fields() {
    for input in [
        "{",
        r#"{"msg":"ok",}"#,
        r#"{"msg":"ok"} {}"#,
        r#"{"msg":"\uD800"}"#,
        r#"{"n":NaN}"#,
    ] {
        let log = parse(input);
        assert_eq!(log.outcome(), ParseOutcome::InvalidJson, "{input}");
        assert!(log.fields().is_empty());
        assert_eq!(log.original(), input);
        assert_eq!(log.severity(), None);
    }
}

#[test]
fn nested_structures_are_never_silently_stringified() {
    let input = format!("{{\"nested\":{}0{}}}", "[".repeat(200), "]".repeat(200));
    let log = parse(&input);
    assert_eq!(log.outcome(), ParseOutcome::InvalidJson);
    assert_eq!(log.original(), input);
    assert!(log.fields().is_empty());
}

#[test]
fn text_arrays_and_stringified_json_are_passthrough() {
    for input in ["ready λ", "[1,2]", "null", r#""{\"msg\":\"inner\"}""#] {
        let log = parse(input);
        assert_eq!(log.outcome(), ParseOutcome::Text);
        assert_eq!(log.message(), input);
        assert!(log.fields().is_empty());
    }
}

#[test]
fn limits_count_bytes_and_preserve_oversized_original() {
    let log = Parser::with_max_bytes(3).parse("λλ");
    assert_eq!(log.outcome(), ParseOutcome::InputLimit);
    assert_eq!(log.original(), "λλ");
    assert_eq!(
        Parser::with_max_bytes(2).parse("λ").outcome(),
        ParseOutcome::Text
    );
}

#[test]
fn conflicting_metadata_is_not_guessed() {
    let input = r#"{"msg":"one","message":"two","level":"WARN","severity_text":"ERROR","time":"2024-01-01T00:00:00Z","timestamp":"2024-01-02T00:00:00Z"}"#;
    let log = parse(input);
    assert_eq!(log.message(), input);
    assert_eq!(log.severity(), None);
    assert_eq!(log.timestamp(), None);
    assert_eq!(log.fields().len(), 6);
}

#[test]
fn numeric_levels_require_known_scale() {
    assert_eq!(parse(r#"{"level":30,"quantity":8}"#).severity(), None);
    assert_eq!(
        parse(r#"{"level":30,"msg":"ready","time":1700000000123}"#).severity(),
        Some(Severity::Info)
    );
    assert_eq!(
        parse(r#"{"levelno":30,"message":"ready"}"#).severity(),
        Some(Severity::Warn)
    );
    assert_eq!(
        parse(r#"{"severity_number":13}"#).severity(),
        Some(Severity::Warn)
    );
    assert_eq!(parse(r#"{"severity_number":99}"#).severity(), None);
}

#[test]
fn timestamps_preserve_fraction_offsets_and_negative_epoch() {
    let log = parse(r#"{"time":"1970-01-01T01:00:00.123456789+01:00"}"#);
    assert_eq!(log.timestamp(), Timestamp::new(0, 123456789));
    assert_eq!(
        parse(r#"{"time_ms":-1}"#).timestamp(),
        Timestamp::new(-1, 999000000)
    );
    assert_eq!(parse(r#"{"timestamp":1700000000}"#).timestamp(), None);
    assert_eq!(
        parse(r#"{"time":"2024-01-01T00:00:00+25:00"}"#).timestamp(),
        None
    );
    assert_eq!(Timestamp::new(0, 1000000000), None);
}

#[test]
fn decolorization_changes_only_display_projection() {
    let input = "\x1b[31mfailed\x1b[0m\r\n\tλ\x00";
    let log = parse(input);
    assert_eq!(log.message(), "failed\n\tλ");
    assert_eq!(log.original(), input);
    let input = r#"{"msg":"\u001b[31mfailed\u001b[0m"}"#;
    let log = parse(input);
    assert_eq!(log.message(), "failed");
    assert_eq!(log.fields()["msg"], "\x1b[31mfailed\x1b[0m");
}

#[test]
fn private_decoder_token_cannot_spoof_numbers() {
    for input in [
        r#"{"$serde_json::private::Number":"123"}"#,
        r#"{"nested":{"\u0024serde_json::private::Number":"123"}}"#,
    ] {
        let log = parse(input);
        assert_eq!(log.outcome(), ParseOutcome::InvalidJson);
        assert_eq!(log.original(), input);
        assert!(log.fields().is_empty());
    }
}

fn arbitrary_json() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(|n| json!(n)),
        ".{0,40}".prop_map(Value::String)
    ];
    leaf.prop_recursive(5, 64, 8, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..8).prop_map(Value::Array),
            prop::collection::btree_map("[a-z]{1,12}", inner, 0..8)
                .prop_map(|m| Value::Object(m.into_iter().collect()))
        ]
    })
}

proptest! {
    #[test]
    fn arbitrary_objects_preserve_every_value(value in arbitrary_json()) {
        let object = json!({"payload": value});
        let input = object.to_string();
        let log = parse(&input);
        prop_assert_eq!(log.outcome(), ParseOutcome::Structured);
        prop_assert_eq!(Value::Object(log.fields().clone()), object);
        prop_assert_eq!(log.original(), input.as_str());
    }

    #[test]
    fn arbitrary_utf8_never_panics_or_changes_original(input in ".{0,4096}") {
        let log = parse(&input);
        prop_assert_eq!(log.original(), input.as_str());
    }
}

#[test]
fn source_review_numeric_edges_keep_exact_representations() {
    for literal in [
        "9007199254740993",
        "18446744073709551615",
        "184467440737095516160",
        "0.123456789012345678901",
        "1e400",
        "-0.0",
    ] {
        let input = format!("{{\"values\":[{literal}]}}");
        let log = parse(&input);
        assert_eq!(log.outcome(), ParseOutcome::Structured, "{literal}");
        let expected: Value = serde_json::from_str(&input).unwrap();
        assert_eq!(Value::Object(log.fields().clone()), expected, "{literal}");
        let number = log.fields()["values"][0].to_string();
        // serde_json normalizes the exponent spelling, but not its precision.
        assert_eq!(
            number.to_lowercase().replace("e+", "e"),
            literal,
            "{literal}"
        );
    }
}

#[test]
fn literal_dotted_keys_do_not_collide_with_nested_keys() {
    let log = parse(r#"{"req.path":"/literal","req":{"path":"/nested"}}"#);
    assert_eq!(log.fields()["req.path"], "/literal");
    assert_eq!(log.fields()["req"]["path"], "/nested");
}

#[test]
fn unicode_escapes_are_strict_and_bom_is_passthrough() {
    assert_eq!(parse(r#"{"msg":"\uD83D\uDE80"}"#).message(), "🚀");
    for input in ["", "[]", "[{},{}]", "\u{feff}{\"msg\":\"ready\"}"] {
        let log = parse(input);
        assert_eq!(log.outcome(), ParseOutcome::Text);
        assert_eq!(log.original(), input);
        assert_eq!(log.timestamp(), None);
    }
    assert_eq!(
        parse("{\"msg\":\"line\nbreak\"}").outcome(),
        ParseOutcome::InvalidJson
    );
    assert_eq!(parse(r#"{"msg":"line\nbreak"}"#).message(), "line\nbreak");
}

#[test]
fn timestamp_precision_and_timezone_are_never_invented() {
    for timestamp in [
        "2024-01-01T00:00:00.1234567891Z",
        "2024-01-01T00:00:00",
        "2024-02-30T00:00:00Z",
    ] {
        let input = json!({"timestamp": timestamp}).to_string();
        assert_eq!(parse(&input).timestamp(), None, "{timestamp}");
    }
    let log = parse(
        r#"{"time":"2024-01-01T01:00:00+01:00","timestamp":"2024-01-01T00:00:00Z","level":"warning","severity":"WARN"}"#,
    );
    assert!(log.timestamp().is_some());
    assert_eq!(log.severity(), Some(Severity::Warn));
}

#[test]
fn decoder_depth_boundary_is_explicit_and_lossless() {
    // The outer root counts as a container in serde_json's depth budget.
    for (depth, outcome) in [
        (126, ParseOutcome::Structured),
        (127, ParseOutcome::InvalidJson),
        (128, ParseOutcome::InvalidJson),
    ] {
        let input = format!("{{\"nested\":{}0{}}}", "[".repeat(depth), "]".repeat(depth));
        let log = parse(&input);
        assert_eq!(log.outcome(), outcome, "depth={depth}");
        assert_eq!(log.original(), input);
        if outcome == ParseOutcome::Structured {
            assert!(log.fields()["nested"].is_array());
        } else {
            assert!(log.fields().is_empty());
        }
    }
}

#[test]
fn known_seconds_timestamps_accept_exact_exponents() {
    for (literal, expected) in [
        ("1e-6", Timestamp::new(0, 1000)),
        ("-1e-6", Timestamp::new(-1, 999999000)),
        ("1e-9", Timestamp::new(0, 1)),
        ("1.234e+3", Timestamp::new(1234, 0)),
        ("0e400", Timestamp::new(0, 0)),
        ("1e-10", None),
        ("1e400", None),
    ] {
        let input = format!(
            r#"{{"text":"example","record":{{"message":"example","level":{{"name":"WARNING","no":30}},"time":{{"timestamp":{literal}}}}}}}"#
        );
        assert_eq!(parse(&input).timestamp(), expected, "{literal}");
    }
}

#[test]
fn terminal_sequences_do_not_leak_payload_or_destroy_unicode() {
    for (input, expected) in [
        ("\x1b(Bready", "ready"),
        ("\x1bP1;2|hidden\x1b\\ready", "ready"),
        (
            "\x1b]8;;https://example.org\x1b\\ready\x1b]8;;\x1b\\",
            "ready",
        ),
        ("\x1bλready", "λready"),
        ("\x1b[31λready", "λready"),
        ("ready\tλ\n", "ready\tλ\n"),
    ] {
        let log = parse(input);
        assert_eq!(log.message(), expected, "{input:?}");
        assert_eq!(log.original(), input);
    }
}

#[test]
fn unicode_control_payloads_are_suppressed_and_cleanup_state_is_per_record() {
    assert_eq!(parse("\x1b]0;hidden λ\x07ready").message(), "ready");
    assert_eq!(parse("\x1bP1|hidden λ\x1b\\ready").message(), "ready");
    assert_eq!(parse("\x1b[31").message(), "");
    assert_eq!(parse("ready").message(), "ready");
}

proptest! {
    #[test]
    fn valid_color_codes_preserve_visible_unicode_and_spacing(input in "[A-Za-z0-9λ🚀\\t\\n]{0,128}") {
        let colored = format!("\x1b[31m{input}\x1b[0m");
        let log = parse(&colored);
        prop_assert_eq!(log.message(), input.as_str());
        prop_assert_eq!(log.original(), colored.as_str());
    }
}

proptest! {
    #[test]
    fn arbitrary_types_at_recognition_paths_preserve_the_complete_record(
        value in arbitrary_json(),
        key in prop::sample::select(vec![
            "msg", "message", "fields", "record", "text", "target", "event",
            "level", "severity", "severity_number", "levelno", "time", "timestamp",
            "time_ms", "pid", "hostname",
        ]),
    ) {
        let mut object = json!({
            "msg": "ready", "level": "INFO", "time": 1700000000123_i64,
            "target": "example", "fields": {"message": "ready"},
            "text": "ready", "record": {
                "message": "ready", "level": {"name": "INFO", "no": 20},
                "time": {"timestamp": 1700000000.123}
            }
        });
        object[key] = value;
        let input = object.to_string();
        let log = parse(&input);
        prop_assert_eq!(log.outcome(), ParseOutcome::Structured);
        prop_assert_eq!(Value::Object(log.fields().clone()), object);
        prop_assert_eq!(log.original(), input.as_str());
    }
}

#[test]
fn hclog_aliases_participate_in_consensus_without_dropping_fields() {
    let input = r#"{"@message":"ready","@level":"warn","@timestamp":"2023-11-14T22:13:20.123000Z","count":3}"#;
    let log = parse(input);
    assert_eq!(log.message(), "ready");
    assert_eq!(log.severity(), Some(Severity::Warn));
    assert_eq!(log.timestamp(), Timestamp::new(1700000000, 123000000));
    assert_eq!(log.fields()["count"], 3);
    let conflicting = r#"{"@message":"one","@level":"warn","msg":"two","level":"error"}"#;
    let log = parse(conflicting);
    assert_eq!(log.message(), conflicting);
    assert_eq!(log.severity(), None);
    assert_eq!(log.fields().len(), 4);
    let unknown = r#"{"@message":"ready","@level":"custom"}"#;
    assert_eq!(parse(unknown).message(), unknown);
}
