//! Adversarial transformations of real records and independent numeric oracles.
use bigdecimal::BigDecimal;
use lognorm::{parse, ParseOutcome, Parser, Timestamp};
use proptest::prelude::*;
use serde_json::{json, Value};
use std::str::FromStr;

#[test]
fn every_truncated_real_json_record_falls_back_without_partial_metadata() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../compatibility/fixtures.json")).unwrap();
    for case in cases {
        if case["expected"]["structured"] != true {
            continue;
        }
        let input = case["input"].as_str().unwrap().trim();
        for (end, _) in input.char_indices().skip(1) {
            let truncated = &input[..end];
            let log = parse(truncated);
            assert_eq!(
                log.outcome(),
                ParseOutcome::InvalidJson,
                "{} at {end}",
                case["scenario"]
            );
            assert_eq!(log.original(), truncated);
            assert!(log.fields().is_empty());
            assert!(log.severity().is_none());
            assert!(log.timestamp().is_none());
        }
    }
}

#[test]
fn default_limit_handles_wide_objects_long_numbers_and_incomplete_strings() {
    let wide = format!("{{\"values\":[{}0]}}", "0,".repeat(100_000));
    let log = parse(&wide);
    assert_eq!(log.outcome(), ParseOutcome::Structured);
    assert_eq!(log.fields()["values"].as_array().unwrap().len(), 100_001);
    drop(log);

    // Beyond the f64 range: the record falls back with its original intact.
    let long_number = format!("{{\"n\":{}}}", "9".repeat(100_000));
    let log = parse(&long_number);
    assert_eq!(log.outcome(), ParseOutcome::InvalidJson);
    assert_eq!(log.original(), long_number);
    drop(log);

    // Within range but more digits than f64 holds: the exact literal is kept.
    let long_decimal = format!("0.{}", "1".repeat(100_000));
    let input = format!("{{\"n\":{long_decimal}}}");
    let log = parse(&input);
    assert_eq!(log.outcome(), ParseOutcome::Structured);
    assert_eq!(log.fields()["n"], long_decimal.as_str());

    let incomplete = format!("{{\"msg\":\"{}", "λ".repeat(100_000));
    assert_eq!(parse(&incomplete).outcome(), ParseOutcome::InvalidJson);
    let oversized = format!("{{\"msg\":\"{}\"}}", "x".repeat(1024 * 1024));
    let log = parse(&oversized);
    assert_eq!(log.outcome(), ParseOutcome::InputLimit);
    assert_eq!(log.message(), oversized);
}

proptest! {
    // Each number is either a JSON number with exactly the literal's decimal
    // value, or the literal itself. An independent decimal library is the oracle.
    #[test]
    fn numbers_are_exact_or_kept_as_their_literal(
        negative in any::<bool>(),
        integer in "(0|[1-9][0-9]{0,30})",
        fraction in proptest::option::of("[0-9]{1,30}"),
        exponent in proptest::option::of(-30_i32..30),
    ) {
        let mut literal = format!("{}{integer}", if negative { "-" } else { "" });
        if let Some(fraction) = &fraction {
            literal = format!("{literal}.{fraction}");
        }
        if let Some(exponent) = exponent {
            literal = format!("{literal}e{exponent}");
        }
        let input = format!("{{\"n\":{literal},\"after\":[7,\"8\"]}}");
        let log = parse(&input);
        prop_assert_eq!(log.outcome(), ParseOutcome::Structured);
        match &log.fields()["n"] {
            Value::Number(number) => prop_assert_eq!(
                BigDecimal::from_str(&number.to_string()).unwrap(),
                BigDecimal::from_str(&literal).unwrap(),
                "{} became {}", literal, number
            ),
            Value::String(text) => prop_assert_eq!(text, &literal),
            other => prop_assert!(false, "unexpected value {other}"),
        }
        prop_assert_eq!(&log.fields()["after"], &json!([7, "8"]));
    }

    #[test]
    fn seconds_notation_matches_an_independent_integer_oracle(
        // Up to 15 significant digits always fit f64 exactly, as do the shortest
        // float renderings real loggers emit. Full i64 coefficients often do not.
        coefficient in prop_oneof![any::<i64>(), -10_i64.pow(15)..10_i64.pow(15)],
        fractional_digits in 0_u32..=9,
        exponent_notation in any::<bool>(),
    ) {
        let literal = if exponent_notation {
            format!("{coefficient}e-{fractional_digits}")
        } else {
            let magnitude = i128::from(coefficient).abs();
            let divisor = 10_i128.pow(fractional_digits);
            let sign = if coefficient < 0 { "-" } else { "" };
            format!("{sign}{}.{:0width$}", magnitude / divisor, magnitude % divisor,
                width = fractional_digits as usize)
        };
        let input = format!(
            r#"{{"text":"ready","record":{{"message":"ready","level":{{"name":"INFO"}},"time":{{"timestamp":{literal}}}}}}}"#
        );
        let total_nanos = i128::from(coefficient) * 10_i128.pow(9 - fractional_digits);
        let expected = Timestamp::new(
            total_nanos.div_euclid(1_000_000_000).try_into().unwrap(),
            total_nanos.rem_euclid(1_000_000_000) as u32,
        );
        // A literal with more digits than f64 holds stays exact text in the
        // fields and the timestamp is left unresolved rather than rounded.
        // Shortest renderings can tie, so use serde_json's, as fields do.
        let fits_f64 = serde_json::Number::from_f64(literal.parse().unwrap())
            .is_some_and(|number| {
                BigDecimal::from_str(&number.to_string()).unwrap()
                    == BigDecimal::from_str(&literal).unwrap()
            });
        prop_assert_eq!(parse(&input).timestamp(), expected.filter(|_| fits_f64));
    }

    #[test]
    fn byte_limit_boundary_is_exact_for_every_generated_record(message in ".{0,128}") {
        let input = json!({"msg": message, "level": "INFO"}).to_string();
        let within = Parser::with_max_bytes(input.len());
        let below = Parser::with_max_bytes(input.len() - 1);
        prop_assert_eq!(within.parse(&input).outcome(), ParseOutcome::Structured);
        let rejected = below.parse(&input);
        prop_assert_eq!(rejected.outcome(), ParseOutcome::InputLimit);
        prop_assert_eq!(rejected.original(), input.as_str());
        prop_assert!(rejected.fields().is_empty());
        prop_assert!(rejected.severity().is_none());
    }
}
