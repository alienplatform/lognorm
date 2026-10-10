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
    let (mantissa, exponent) = unsigned.split_once(['e', 'E']).unwrap_or((unsigned, "0"));
    let fraction_len = mantissa
        .split_once('.')
        .map_or(0, |(_, fraction)| fraction.len());
    let significant = mantissa.trim_end_matches('0');
    let trailing_zeros = mantissa.len() - significant.len();
    let coefficient = significant
        .bytes()
        .filter(|byte| *byte != b'.')
        .try_fold(0_i128, |total, digit| {
            total.checked_mul(10)?.checked_add(i128::from(digit - b'0'))
        })?;
    if coefficient == 0 {
        return Timestamp::new(0, 0);
    }
    let scale = exponent
        .parse::<i64>()
        .ok()?
        .checked_sub(i64::try_from(fraction_len).ok()?)?
        .checked_add(i64::try_from(trailing_zeros).ok()?)?
        .checked_add(9)?;
    // After removing trailing zeros, a negative scale has a nonzero sub-nanosecond
    // fraction. Reject it rather than round; checked arithmetic also bounds work.
    let factor = 10_i128.checked_pow(u32::try_from(scale).ok()?)?;
    let total = coefficient.checked_mul(factor)?;
    let total = if negative {
        total.checked_neg()?
    } else {
        total
    };
    Timestamp::new(
        total.div_euclid(1_000_000_000).try_into().ok()?,
        total.rem_euclid(1_000_000_000) as u32,
    )
}
