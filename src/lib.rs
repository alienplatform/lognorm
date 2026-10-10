//! Automatic normalization of complete application log records.
//!
//! ```
//! use lognorm::{parse, Severity};
//!
//! let line = r#"{"level":"WARN","msg":"Request rejected","statusCode":401}"#;
//! let log = parse(line);
//!
//! assert_eq!(log.message(), "Request rejected");
//! assert_eq!(log.severity(), Some(Severity::Warn));
//! assert_eq!(log.fields()["statusCode"], 401);
//! assert_eq!(log.original(), line);
//! ```

mod decode;
mod display;
mod recognition;
mod severity;
mod timestamp;

#[cfg(feature = "otlp")]
pub mod otlp;

use std::borrow::Cow;

use serde_json::{Map, Value};

pub use severity::Severity;
pub use timestamp::Timestamp;

/// Why a record was decoded or passed through unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseOutcome {
    /// A JSON object was decoded and its fields preserved.
    Structured,
    /// Ordinary text, a JSON scalar, or a JSON array.
    Text,
    /// Input looked like a JSON object but could not be decoded safely.
    InvalidJson,
    /// An object contained duplicate keys and could not be preserved as a map.
    DuplicateKeys,
    /// Input exceeded the configured byte limit.
    InputLimit,
}

/// A decoded record with semantic projections and exact original input.
#[derive(Debug)]
pub struct ParsedLog<'a> {
    original: &'a str,
    fields: Map<String, Value>,
    message_path: Option<&'static [&'static str]>,
    clean_message: Option<String>,
    severity: Option<Severity>,
    timestamp: Option<Timestamp>,
    outcome: ParseOutcome,
}

impl ParsedLog<'_> {
    /// Readable message, or the original input when no unambiguous message exists.
    ///
    /// ANSI escapes and non-display controls are removed from this projection,
    /// except when the input exceeds the configured size limit.
    /// Newlines and tabs are retained. Original bytes and fields are untouched.
    pub fn message(&self) -> &str {
        self.clean_message
            .as_deref()
            .unwrap_or_else(|| self.source_message())
    }

    fn source_message(&self) -> &str {
        self.message_path
            .and_then(|path| recognition::at(&self.fields, path))
            .and_then(Value::as_str)
            .unwrap_or(self.original)
    }

    /// Unambiguous recognized severity, without a stream-based fallback.
    pub fn severity(&self) -> Option<Severity> {
        self.severity
    }

    /// Unambiguous source timestamp, without consulting the clock.
    pub fn timestamp(&self) -> Option<Timestamp> {
        self.timestamp
    }

    /// Complete decoded root object, including fields used by other accessors.
    ///
    /// Objects remain nested. On fallback this map is empty.
    pub fn fields(&self) -> &Map<String, Value> {
        &self.fields
    }

    /// Exact input, including whitespace and terminal control sequences.
    pub fn original(&self) -> &str {
        self.original
    }

    /// Classification for metrics and diagnostics without emitting input data.
    pub fn outcome(&self) -> ParseOutcome {
        self.outcome
    }
}

/// Stateless parser with immutable resource limits.
#[derive(Debug, Clone)]
pub struct Parser {
    max_bytes: usize,
}

impl Default for Parser {
    fn default() -> Self {
        Self::with_max_bytes(1024 * 1024)
    }
}

impl Parser {
    /// Construct a parser with a maximum input size in UTF-8 bytes.
    pub const fn with_max_bytes(max_bytes: usize) -> Self {
        Self { max_bytes }
    }

    /// Normalize one complete application record, classifying malformed input as fallback.
    ///
    /// The byte limit bounds input size, not total allocations. Allocation failure
    /// and insufficient thread stack are not recoverable parsing outcomes.
    pub fn parse<'a>(&self, input: &'a str) -> ParsedLog<'a> {
        let (fields, outcome) = if input.len() > self.max_bytes {
            (Map::new(), ParseOutcome::InputLimit)
        } else if !input.trim().starts_with('{') {
            (Map::new(), ParseOutcome::Text)
        } else {
            match decode::object(input) {
                Ok(fields) => (fields, ParseOutcome::Structured),
                Err(outcome) => (Map::new(), outcome),
            }
        };

        let mut log = ParsedLog {
            original: input,
            message_path: if fields.is_empty() {
                None
            } else {
                recognition::message_path(&fields)
            },
            clean_message: None,
            severity: if fields.is_empty() {
                None
            } else {
                recognition::severity(&fields)
            },
            timestamp: if fields.is_empty() {
                None
            } else {
                recognition::timestamp(&fields)
            },
            fields,
            outcome,
        };

        if outcome != ParseOutcome::InputLimit {
            if let Cow::Owned(message) = display::clean(log.source_message()) {
                log.clean_message = Some(message);
            }
        }

        log
    }
}

/// Normalize one complete record using the default 1 MiB input limit.
pub fn parse(input: &str) -> ParsedLog<'_> {
    Parser::default().parse(input)
}
