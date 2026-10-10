//! Linking lognorm must not change how other crates in the build decode JSON.
//!
//! Cargo unifies dependency features. If lognorm enabled serde_json's
//! `arbitrary_precision`, numbers buffered by serde (untagged, internally
//! tagged, and flattened types) would arrive as a private map, and these
//! ordinary dependent types would fail to decode.
use serde::Deserialize;

/// The shape of a JSON Schema bound, which schema libraries model as untagged.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum Bound {
    Number(f64),
    Text(String),
}

#[derive(Debug, Deserialize, PartialEq)]
struct Schema {
    minimum: Bound,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "kind")]
enum Event {
    Resize { replicas: u32, ratio: f64 },
}

#[derive(Debug, Deserialize, PartialEq)]
struct Envelope {
    id: u64,
    #[serde(flatten)]
    rest: serde_json::Map<String, serde_json::Value>,
}

#[test]
fn untagged_internally_tagged_and_flattened_numbers_still_decode() {
    // Keep lognorm linked into this test binary.
    assert_eq!(lognorm::parse(r#"{"msg":"ready"}"#).message(), "ready");

    let schema: Schema = serde_json::from_str(r#"{"minimum":0.0}"#).unwrap();
    assert_eq!(schema.minimum, Bound::Number(0.0));

    let event: Event =
        serde_json::from_str(r#"{"kind":"Resize","replicas":3,"ratio":0.5}"#).unwrap();
    assert_eq!(
        event,
        Event::Resize {
            replicas: 3,
            ratio: 0.5
        }
    );

    let envelope: Envelope = serde_json::from_str(r#"{"id":7,"weight":1.50}"#).unwrap();
    assert_eq!(envelope.id, 7);
    assert_eq!(envelope.rest["weight"], 1.5);
    assert_eq!(envelope.rest["weight"].to_string(), "1.5");
}
