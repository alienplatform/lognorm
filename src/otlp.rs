//! Optional OTLP protobuf adapter. Transport and collector context stay with callers.
//!
//! Application fields are nested under `app`; the body is readable text. JSON
//! null is represented by an empty OTLP AnyValue. Integers outside i64 and decimal
//! numbers that cannot round-trip through f64 are exported as exact strings.
//! All source values remain available in [`ParsedLog::fields`].
use opentelemetry_proto::tonic::{
    common::v1::{any_value::Value as OtlpValue, AnyValue, ArrayValue, KeyValue, KeyValueList},
    logs::v1::LogRecord,
};
use serde_json::{Map, Value};

use crate::{ParsedLog, Severity};

/// Build a log record with normalized body, application fields, and metadata.
///
/// The caller must set observed time and attach resource/scope information.
/// Missing, negative, or overflowing timestamps become OTLP's unspecified zero;
/// callers can supply their collection timestamp when that happens. Missing
/// severity is zero (unspecified). No infrastructure attributes are invented.
///
/// ```
/// let parsed = lognorm::parse(r#"{"msg":"ready","level":"INFO","count":3}"#);
/// let record = lognorm::otlp::log_record(&parsed);
/// assert_eq!(record.severity_number, 9);
/// assert_eq!(record.attributes[0].key, "app");
/// ```
pub fn log_record(log: &ParsedLog<'_>) -> LogRecord {
    let time_unix_nano = log
        .timestamp()
        .and_then(|time| {
            u64::try_from(time.unix_seconds())
                .ok()?
                .checked_mul(1_000_000_000)?
                .checked_add(u64::from(time.nanoseconds()))
        })
        .unwrap_or(0);
    let severity_number = match log.severity() {
        None => 0,
        Some(Severity::Trace) => 1,
        Some(Severity::Debug) => 5,
        Some(Severity::Info) => 9,
        Some(Severity::Warn) => 13,
        Some(Severity::Error) => 17,
        Some(Severity::Fatal) => 21,
    };
    LogRecord {
        time_unix_nano,
        severity_number,
        severity_text: log
            .severity()
            .map(Severity::as_str)
            .unwrap_or("")
            .to_owned(),
        body: Some(AnyValue {
            value: Some(OtlpValue::StringValue(log.message().to_owned())),
        }),
        attributes: if log.fields().is_empty() {
            Vec::new()
        } else {
            vec![KeyValue {
                key: "app".to_owned(),
                value: Some(object(log.fields())),
            }]
        },
        ..LogRecord::default()
    }
}

fn object(fields: &Map<String, Value>) -> AnyValue {
    AnyValue {
        value: Some(OtlpValue::KvlistValue(KeyValueList {
            values: fields
                .iter()
                .map(|(key, value)| KeyValue {
                    key: key.clone(),
                    value: Some(value_to_otlp(value)),
                })
                .collect(),
        })),
    }
}

fn value_to_otlp(value: &Value) -> AnyValue {
    let value = match value {
        Value::Null => None,
        Value::Bool(value) => Some(OtlpValue::BoolValue(*value)),
        Value::String(value) => Some(OtlpValue::StringValue(value.clone())),
        Value::Array(values) => Some(OtlpValue::ArrayValue(ArrayValue {
            values: values.iter().map(value_to_otlp).collect(),
        })),
        Value::Object(fields) => return object(fields),
        Value::Number(number) => {
            if let Some(integer) = number.as_i64() {
                Some(OtlpValue::IntValue(integer))
            } else if !number.is_u64()
                && number.as_f64().is_some_and(|float| {
                    serde_json::Number::from_f64(float).as_ref() == Some(number)
                })
            {
                number.as_f64().map(OtlpValue::DoubleValue)
            } else {
                Some(OtlpValue::StringValue(number.to_string()))
            }
        }
    };
    AnyValue { value }
}
