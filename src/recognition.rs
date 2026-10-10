use serde_json::{Map, Value};

use crate::{severity, timestamp, Severity, Timestamp};

type Path = &'static [&'static str];

pub(crate) fn at<'a>(fields: &'a Map<String, Value>, path: &[&str]) -> Option<&'a Value> {
    let (first, rest) = path.split_first()?;
    let mut value = fields.get(*first)?;
    for key in rest {
        value = value.as_object()?.get(*key)?;
    }
    Some(value)
}

fn tracing(fields: &Map<String, Value>) -> bool {
    fields.get("target").is_some_and(Value::is_string)
        && fields
            .get("level")
            .and_then(Value::as_str)
            .and_then(severity::name)
            .is_some()
}

fn loguru(fields: &Map<String, Value>) -> bool {
    fields.get("text").is_some_and(Value::is_string)
        && at(fields, &["record", "level", "name"]).is_some_and(Value::is_string)
        && at(fields, &["record", "message"]).is_some_and(Value::is_string)
}

fn pino(fields: &Map<String, Value>) -> bool {
    (fields.get("msg").is_some_and(Value::is_string)
        || fields.get("message").is_some_and(Value::is_string))
        && (fields
            .get("time")
            .is_some_and(|time| time.is_number() || time.is_string())
            || (fields.get("pid").is_some_and(Value::is_u64)
                && fields.get("hostname").is_some_and(Value::is_string)))
}

pub(crate) fn message_path(fields: &Map<String, Value>) -> Option<Path> {
    let candidates: [(Path, bool); 5] = [
        (&["msg"], true),
        (&["message"], true),
        (&["fields", "message"], tracing(fields)),
        (&["record", "message"], loguru(fields)),
        (
            &["event"],
            fields
                .get("level")
                .and_then(Value::as_str)
                .and_then(severity::name)
                .is_some(),
        ),
    ];

    let mut selected = None;
    let mut message = None;
    for (path, enabled) in candidates {
        if !enabled {
            continue;
        }
        if let Some(value) = at(fields, path).and_then(Value::as_str) {
            if message.is_some_and(|previous| previous != value) {
                return None;
            }
            selected = Some(path);
            message = Some(value);
        }
    }
    selected
}

struct Consensus<T> {
    value: Option<T>,
    conflicting: bool,
}

impl<T: Copy + Eq> Consensus<T> {
    fn new() -> Self {
        Self {
            value: None,
            conflicting: false,
        }
    }

    fn add(&mut self, value: Option<T>) {
        if let Some(value) = value {
            self.conflicting |= self.value.is_some_and(|previous| previous != value);
            self.value = Some(value);
        }
    }

    fn finish(self) -> Option<T> {
        if self.conflicting {
            None
        } else {
            self.value
        }
    }
}

pub(crate) fn severity(fields: &Map<String, Value>) -> Option<Severity> {
    let mut result = Consensus::new();
    for key in [
        "level",
        "severity",
        "severityText",
        "severity_text",
        "levelname",
        "level_name",
        "logLevel",
        "log_level",
        "log.level",
    ] {
        result.add(
            fields
                .get(key)
                .and_then(Value::as_str)
                .and_then(severity::name),
        );
    }
    result.add(
        at(fields, &["log", "level"])
            .and_then(Value::as_str)
            .and_then(severity::name),
    );
    for key in ["severityNumber", "severity_number"] {
        result.add(
            fields
                .get(key)
                .and_then(Value::as_i64)
                .and_then(severity::otel),
        );
    }
    for key in ["levelno", "level_number"] {
        result.add(
            fields
                .get(key)
                .and_then(Value::as_i64)
                .and_then(severity::python),
        );
    }
    if pino(fields) {
        result.add(
            fields
                .get("level")
                .and_then(Value::as_i64)
                .and_then(severity::pino),
        );
    }
    if loguru(fields) {
        result.add(
            at(fields, &["record", "level", "name"])
                .and_then(Value::as_str)
                .and_then(severity::name),
        );
        result.add(
            at(fields, &["record", "level", "no"])
                .and_then(Value::as_i64)
                .and_then(severity::python),
        );
    }
    result.finish()
}

pub(crate) fn timestamp(fields: &Map<String, Value>) -> Option<Timestamp> {
    let mut result = Consensus::new();
    for key in ["timestamp", "time", "ts", "@timestamp", "asctime"] {
        result.add(fields.get(key).and_then(timestamp::rfc3339));
    }
    if pino(fields) {
        result.add(fields.get("time").and_then(timestamp::milliseconds));
    }
    for key in ["timestamp_ms", "time_ms"] {
        result.add(fields.get(key).and_then(timestamp::milliseconds));
    }
    if loguru(fields) {
        result.add(at(fields, &["record", "time", "timestamp"]).and_then(timestamp::seconds));
    }
    result.finish()
}
