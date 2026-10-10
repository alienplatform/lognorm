use chrono::DateTime;
use serde_json::Value;

/// A Unix timestamp with a signed seconds component and nanosecond fraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timestamp {
    seconds: i64,
    nanos: u32,
}

impl Timestamp {
    /// Construct a timestamp, rejecting fractions of one second or greater.
    pub const fn new(unix_seconds: i64, nanoseconds: u32) -> Option<Self> {
        if nanoseconds < 1_000_000_000 {
            Some(Self {
                seconds: unix_seconds,
                nanos: nanoseconds,
            })
        } else {
            None
        }
    }

    /// Signed whole seconds since the Unix epoch, rounded toward negative infinity.
    pub const fn unix_seconds(self) -> i64 {
        self.seconds
    }

    /// Nanoseconds after the whole second, always less than one billion.
    pub const fn nanoseconds(self) -> u32 {
        self.nanos
    }
}

pub(crate) fn rfc3339(value: &Value) -> Option<Timestamp> {
    let text = value.as_str()?;
    // chrono accepts and truncates fractions beyond nanoseconds. Reject those
    // rather than silently changing the source timestamp's precision.
    if text
        .split_once('.')
        .is_some_and(|(_, tail)| tail.bytes().take_while(u8::is_ascii_digit).count() > 9)
    {
        return None;
    }
    let date = DateTime::parse_from_rfc3339(text).ok()?;
    Timestamp::new(date.timestamp(), date.timestamp_subsec_nanos())
}

pub(crate) fn milliseconds(value: &Value) -> Option<Timestamp> {
    let millis = value.as_i64()?;
    Timestamp::new(
        millis.div_euclid(1000),
        (millis.rem_euclid(1000) as u32) * 1_000_000,
    )
}

pub(crate) fn seconds(value: &Value) -> Option<Timestamp> {
    let number = value.as_number()?.to_string();
    let negative = number.starts_with('-');
    let unsigned = number.strip_prefix('-').unwrap_or(&number);
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    if fraction.len() > 9 || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let whole: i128 = whole.parse().ok()?;
    let nanos: i128 = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<i128>().ok()? * 10_i128.pow(9 - fraction.len() as u32)
    };
    let total = whole.checked_mul(1_000_000_000)?.checked_add(nanos)?;
    let total = if negative { -total } else { total };
    Timestamp::new(
        total.div_euclid(1_000_000_000).try_into().ok()?,
        total.rem_euclid(1_000_000_000) as u32,
    )
}
