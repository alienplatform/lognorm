/// Common severity bands, ordered from least to most severe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Detailed diagnostic information.
    Trace,
    /// Debugging information.
    Debug,
    /// Normal operational information.
    Info,
    /// A condition that may need attention.
    Warn,
    /// An operation failed.
    Error,
    /// A fatal condition.
    Fatal,
}

impl Severity {
    /// Canonical uppercase spelling of the severity band.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Fatal => "FATAL",
        }
    }
}

pub(crate) fn name(value: &str) -> Option<Severity> {
    let value = value.trim();
    [
        (Severity::Trace, &["trace", "trc"][..]),
        (Severity::Debug, &["debug", "dbg"]),
        (
            Severity::Info,
            &["info", "information", "informational", "notice"],
        ),
        (Severity::Warn, &["warn", "warning", "wrn"]),
        (Severity::Error, &["error", "err", "exception"]),
        (
            Severity::Fatal,
            &["fatal", "critical", "crit", "panic", "emergency"],
        ),
    ]
    .into_iter()
    .find_map(|(severity, aliases)| {
        aliases
            .iter()
            .any(|alias| value.eq_ignore_ascii_case(alias))
            .then_some(severity)
    })
}

pub(crate) fn pino(value: i64) -> Option<Severity> {
    match value {
        10 => Some(Severity::Trace),
        20 => Some(Severity::Debug),
        30 => Some(Severity::Info),
        40 => Some(Severity::Warn),
        50 => Some(Severity::Error),
        60 => Some(Severity::Fatal),
        _ => None,
    }
}

pub(crate) fn python(value: i64) -> Option<Severity> {
    match value {
        10 => Some(Severity::Debug),
        20 => Some(Severity::Info),
        30 => Some(Severity::Warn),
        40 => Some(Severity::Error),
        50 => Some(Severity::Fatal),
        _ => None,
    }
}

pub(crate) fn otel(value: i64) -> Option<Severity> {
    match value {
        1..=4 => Some(Severity::Trace),
        5..=8 => Some(Severity::Debug),
        9..=12 => Some(Severity::Info),
        13..=16 => Some(Severity::Warn),
        17..=20 => Some(Severity::Error),
        21..=24 => Some(Severity::Fatal),
        _ => None,
    }
}
