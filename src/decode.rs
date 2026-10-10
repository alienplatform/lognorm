use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

use crate::ParseOutcome;

const DUPLICATE_KEY: &str = "lognorm duplicate object key";
const NUMBER_TOKEN: &str = "$serde_json::private::Number";

struct Preserved(Value);

impl<'de> Deserialize<'de> for Preserved {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(PreservedVisitor)
    }
}

struct PreservedVisitor;

impl<'de> Visitor<'de> for PreservedVisitor {
    type Value = Preserved;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without duplicate object keys")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Preserved, E> {
        Ok(Preserved(Value::Bool(value)))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Preserved, E> {
        Ok(Preserved(Value::Number(value.into())))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Preserved, E> {
        Ok(Preserved(Value::Number(value.into())))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Preserved, E> {
        Number::from_f64(value)
            .map(|number| Preserved(Value::Number(number)))
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Preserved, E> {
        Ok(Preserved(Value::String(value.to_owned())))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Preserved, E> {
        Ok(Preserved(Value::String(value)))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Preserved, E> {
        Ok(Preserved(Value::Null))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Preserved, A::Error> {
        let mut values = Vec::new();
        while let Some(Preserved(value)) = sequence.next_element()? {
            values.push(value);
        }
        Ok(Preserved(Value::Array(values)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Preserved, A::Error> {
        let mut values = Map::new();
        while let Some(key) = object.next_key::<String>()? {
            // serde_json exposes exact numbers through a synthetic map. Input
            // containing this reserved key is rejected before deserialization.
            if key == NUMBER_TOKEN {
                let number = object.next_value::<String>()?;
                let number = number.parse::<Number>().map_err(de::Error::custom)?;
                return Ok(Preserved(Value::Number(number)));
            }
            if values.contains_key(&key) {
                return Err(de::Error::custom(DUPLICATE_KEY));
            }
            let Preserved(value) = object.next_value()?;
            values.insert(key, value);
        }
        Ok(Preserved(Value::Object(values)))
    }
}

pub(crate) fn object(input: &str) -> Result<Map<String, Value>, ParseOutcome> {
    // Reject serde_json's reserved arbitrary-precision representation in input.
    // Decoding escaped key spellings is handled by the lexical guard below.
    if reserved_key(input) {
        return Err(ParseOutcome::InvalidJson);
    }
    let mut deserializer = serde_json::Deserializer::from_str(input);
    let value = Preserved::deserialize(&mut deserializer).map_err(|error| {
        if error.to_string().starts_with(DUPLICATE_KEY) {
            ParseOutcome::DuplicateKeys
        } else {
            ParseOutcome::InvalidJson
        }
    })?;
    deserializer.end().map_err(|_| ParseOutcome::InvalidJson)?;
    match value.0 {
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
