use std::{cell::Cell, fmt};

use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

use crate::ParseOutcome;

const DUPLICATE_KEY: &str = "lognorm duplicate object key";

// serde_json reports numbers as this synthetic map only when some crate in the
// build enables its `arbitrary_precision` feature. lognorm never enables it:
// Cargo unifies features, and the flag changes how unrelated crates decode
// JSON (for example, decimals inside untagged enums stop decoding).
const NUMBER_TOKEN: &str = "$serde_json::private::Number";

/// Number literals of one input, in document order.
///
/// serde_json's number type does not keep source text unless the global
/// `arbitrary_precision` feature is on, so each decoded number takes its exact
/// literal from here. The scan is lazy, linear, and allocation-free.
struct Literals<'a> {
    input: &'a str,
    position: Cell<usize>,
}

impl<'a> Literals<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            position: Cell::new(0),
        }
    }

    fn next(&self) -> Option<&'a str> {
        let bytes = self.input.as_bytes();
        let mut index = self.position.get();
        while let Some(&byte) = bytes.get(index) {
            match byte {
                b'"' => {
                    // Skip strings, including keys, so digits inside them are ignored.
                    index += 1;
                    while let Some(offset) = bytes
                        .get(index..)
                        .and_then(|rest| memchr::memchr2(b'"', b'\\', rest))
                    {
                        index += offset;
                        if bytes.get(index) == Some(&b'"') {
                            break;
                        }
                        index += 2;
                    }
                    index += 1;
                }
                b'-' | b'0'..=b'9' => {
                    let start = index;
                    while bytes.get(index).is_some_and(|byte| {
                        matches!(byte, b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E')
                    }) {
                        index += 1;
                    }
                    self.position.set(index);
                    // Both bounds are ASCII bytes, so they are character boundaries.
                    return self.input.get(start..index);
                }
                _ => index += 1,
            }
        }
        self.position.set(index);
        None
    }
}

/// Classify a JSON number literal without losing information.
///
/// Integers that fit `u64` or `i64` and decimals whose shortest `f64` rendering
/// has the same decimal value become numbers. Other finite literals, such as
/// integers beyond 64 bits or decimals with more digits than `f64` holds, keep
/// their exact literal as a string. Literals outside the `f64` range are
/// rejected, matching serde_json's default decoder.
fn number(literal: &str) -> Result<Value, &'static str> {
    let float = literal.parse::<f64>().map_err(|_| "invalid JSON number")?;
    if !float.is_finite() {
        return Err("number out of range");
    }
    if !literal.contains(['.', 'e', 'E']) {
        match (literal.parse::<u64>(), literal.parse::<i64>()) {
            (Ok(integer), _) => return Ok(Value::Number(integer.into())),
            (_, Ok(integer)) if integer != 0 => return Ok(Value::Number(integer.into())),
            // `-0` keeps its sign as the float -0.0, as serde_json decodes it.
            (_, Ok(_)) => {}
            _ => return Ok(Value::String(literal.to_owned())),
        }
    }
    let number = Number::from_f64(float).ok_or("number out of range")?;
    if same_decimal(literal, &number.to_string()) {
        Ok(Value::Number(number))
    } else {
        Ok(Value::String(literal.to_owned()))
    }
}

/// Whether two finite JSON number spellings have the same decimal value.
///
/// Spelling differences such as `1e-06`, `1e-6`, and `0.000001` are equal.
fn same_decimal(left: &str, right: &str) -> bool {
    decimal_parts(left).is_some_and(|parts| decimal_parts(right) == Some(parts))
}

fn decimal_parts(text: &str) -> Option<(bool, String, i64)> {
    let negative = text.starts_with('-');
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let (mantissa, exponent) = unsigned.split_once(['e', 'E']).unwrap_or((unsigned, "0"));
    let fraction_len = mantissa
        .split_once('.')
        .map_or(0, |(_, fraction)| fraction.len());
    let mut digits: String = mantissa
        .chars()
        .filter(|ch| *ch != '.')
        .skip_while(|ch| *ch == '0')
        .collect();
    if digits.is_empty() {
        return Some((negative, digits, 0));
    }
    let significant_len = digits.trim_end_matches('0').len();
    let trailing_zeros = digits.len() - significant_len;
    digits.truncate(significant_len);
    let exponent = exponent
        .parse::<i64>()
        .ok()?
        .checked_sub(i64::try_from(fraction_len).ok()?)?
        .checked_add(i64::try_from(trailing_zeros).ok()?)?;
    Some((negative, digits, exponent))
}

// serde_json's default float parsing is best-effort rather than correctly
// rounded, so pairing a literal with its decoded float allows a tiny error.
// Different literals are far further apart than this.
fn close(exact: f64, decoded: f64) -> bool {
    let difference = (exact - decoded).abs();
    exact == decoded || difference <= exact.abs() * 1e-12 || difference <= 1e-300
}

/// State for one decoding pass.
struct Pass<'a> {
    literals: Literals<'a>,
    /// Whether numbers kept as literal strings stay strings (the fields) or
    /// become null (the recognition view).
    keep_literal_strings: bool,
    saw_literal_string: Cell<bool>,
}

impl<'a> Pass<'a> {
    fn new(input: &'a str, keep_literal_strings: bool) -> Self {
        Self {
            literals: Literals::new(input),
            keep_literal_strings,
            saw_literal_string: Cell::new(false),
        }
    }
}

#[derive(Clone, Copy)]
struct Preserve<'p, 'a>(&'p Pass<'a>);

impl Preserve<'_, '_> {
    /// Take the next literal, check it against what serde_json decoded, and
    /// classify it. Every number is checked, so a scan that lost its place
    /// fails the record instead of attaching a literal to the wrong field.
    fn number<E: de::Error>(self, decoded: impl FnOnce(&str) -> bool) -> Result<Value, E> {
        let value = match self.0.literals.next() {
            Some(literal) if decoded(literal) => number(literal).map_err(E::custom)?,
            _ => return Err(E::custom("number literal out of order")),
        };
        if value.is_string() {
            self.0.saw_literal_string.set(true);
            if !self.0.keep_literal_strings {
                return Ok(Value::Null);
            }
        }
        Ok(value)
    }
}

impl<'de> DeserializeSeed<'de> for Preserve<'_, '_> {
    type Value = Value;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Preserve<'_, '_> {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without duplicate object keys")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
        self.number(|literal| literal.parse::<i64>() == Ok(value))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
        self.number(|literal| literal.parse::<u64>() == Ok(value))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
        self.number(|literal| {
            literal
                .parse::<f64>()
                .is_ok_and(|exact| close(exact, value))
        })
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(self)? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = object.next_key::<String>()? {
            // Another crate enabled arbitrary_precision. Input containing this
            // reserved key is rejected before deserialization.
            if key == NUMBER_TOKEN {
                // serde_json normalizes exponent spelling (`1E5` becomes `1e+5`).
                let text = object.next_value::<String>()?;
                return self.number(|literal| same_decimal(literal, &text));
            }
            if values.contains_key(&key) {
                return Err(de::Error::custom(DUPLICATE_KEY));
            }
            let value = object.next_value_seed(self)?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}

/// A decoded record.
pub(crate) struct Decoded {
    /// The complete root object.
    pub(crate) fields: Map<String, Value>,
    /// The same object with numbers kept as literal strings replaced by null,
    /// so recognition never mistakes a number for text. Built only when such a
    /// number occurs.
    pub(crate) recognition: Option<Map<String, Value>>,
}

pub(crate) fn object(input: &str) -> Result<Decoded, ParseOutcome> {
    // Reject serde_json's reserved arbitrary-precision representation in input.
    // Decoding escaped key spellings is handled by the lexical guard below.
    if reserved_key(input) {
        return Err(ParseOutcome::InvalidJson);
    }
    let pass = Pass::new(input, true);
    let fields = decode(input, &pass)?;
    let recognition = if pass.saw_literal_string.get() {
        Some(decode(input, &Pass::new(input, false))?)
    } else {
        None
    };
    Ok(Decoded {
        fields,
        recognition,
    })
}

fn decode(input: &str, pass: &Pass<'_>) -> Result<Map<String, Value>, ParseOutcome> {
    let mut deserializer = serde_json::Deserializer::from_str(input);
    let value = Preserve(pass)
        .deserialize(&mut deserializer)
        .map_err(|error| {
            if error.to_string().starts_with(DUPLICATE_KEY) {
                ParseOutcome::DuplicateKeys
            } else {
                ParseOutcome::InvalidJson
            }
        })?;
    deserializer.end().map_err(|_| ParseOutcome::InvalidJson)?;
    match value {
        Value::Object(fields) => Ok(fields),
        _ => Err(ParseOutcome::Text),
    }
}

fn reserved_key(input: &str) -> bool {
    // The private representation is rare. Only inspect strings if its spelling
    // could occur, including Unicode-escaped spellings of the dollar sign.
    if !input.contains("private") && !input.contains("\\u") {
        return false;
    }
    let mut strings = input.char_indices();
    while let Some((start, ch)) = strings.next() {
        if ch != '"' {
            continue;
        }
        let mut escaped = false;
        for (end, ch) in strings.by_ref() {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                if input[end + 1..].trim_start().starts_with(':') {
                    let raw = &input[start..=end];
                    if serde_json::from_str::<String>(raw).ok().as_deref() == Some(NUMBER_TOKEN) {
                        return true;
                    }
                }
                break;
            }
        }
    }
    false
}
