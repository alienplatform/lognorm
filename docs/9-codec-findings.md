# Codec source findings and regression decisions

This review read selected implementations and tests; it did not execute the
upstream suites. Synthetic inputs below are independently authored proposals,
not copied fixtures. They supplement real-logger compatibility tests.

Pinned revisions:

- Vector: `88da8cbbb166c6173af2b99ade67db25893b9f07`.
- VRL: `f040d67833601f7af658b1b0f714bd04b9fa4b67`.
- OpenTelemetry Collector contrib (Stanza):
  `c766745fdaed961d2559b1e2d5d9c468c85d3574`.

## Complete records and framing

Vector's [JSON decoder and tests](https://github.com/vectordotdev/vector/blob/88da8cbbb166c6173af2b99ade67db25893b9f07/lib/codecs/src/decoding/format/json.rs)
explicitly test skipping empty frames, rejecting malformed JSON, splitting a
root array into several events, and accepting scalar null in its Vector
namespace. Legacy namespace decoding inserts the wall clock timestamp only
when the timestamp key is absent; it does not validate an existing application
timestamp at that step.

Our decisions differ: one call always represents one record; arrays and scalars
pass through, empty input remains available, and no timestamp is invented.
Synthetic regressions: empty string; `[]`; `[{},{}]`; `null`; an object with an
invalid timestamp string. Assert original equality and no fabricated time.

Vector's [newline framer and tests](https://github.com/vectordotdev/vector/blob/88da8cbbb166c6173af2b99ade67db25893b9f07/lib/codecs/src/decoding/framing/newline_delimited.rs)
retain an unterminated final fragment during ordinary decoding, emit it at EOF,
and skip an oversized line before continuing with subsequent lines. Its options
also describe a truncation mode. This is a collector boundary, not normalization.
Collector tests should split a record across chunks, include CRLF and EOF without
a newline, and place a valid record after an oversized record. Truncating JSON
must not accidentally join its remainder to the next record. A byte truncation
can cut a multibyte character; encoding policy belongs before this API.

## UTF-8, BOM, escaping, and trailing data

Vector's JSON tests exercise both replacement of invalid UTF-8 and strict
rejection. VRL's [JSON function and tests](https://github.com/vectordotdev/vrl/blob/f040d67833601f7af658b1b0f714bd04b9fa4b67/src/stdlib/parse_json.rs)
also test lossy UTF-8 by default, strict mode, and BOM stripping. The core here
accepts a Rust string: invalid UTF-8 cannot enter its API. Collectors must choose
and document byte decoding; replacement cannot preserve original bytes.

Suggested core regressions: non-ASCII message, escaped newline versus a literal
newline inside JSON, valid surrogate pair, unpaired surrogate escape, and
truncated escape. Assert strict decoding or raw fallback and exact original.
For a leading U+FEFF, document an explicit choice: accept it only as a leading
encoding marker while preserving original, or leave it raw. Do not silently trim
arbitrary control characters. BOM behavior is a proposed decision, not an
upstream behavior inherited automatically.

Vector and VRL normally call serde_json's full-document `from_str`/`from_slice`.
Stanza's [JSON implementation](https://github.com/open-telemetry/opentelemetry-collector-contrib/blob/c766745fdaed961d2559b1e2d5d9c468c85d3574/pkg/stanza/operator/parser/jsonparser/parser.go)
uses Unmarshal normally, but a single Decoder.Decode call with `parse_ints`.
That branch contains no explicit EOF check. Possible acceptance of a trailing
second document is a source-derived hypothesis, not an executed finding.
Our regression should require raw fallback for two concatenated objects and an
object followed by non-whitespace; surrounding JSON whitespace remains valid.
Never repair escapes heuristically after a failed JSON parse.

## Numbers and nesting

VRL's JSON tests explicitly expect a large signed integer to remain an integer
and the first integer above the signed 64-bit range to become a floating-point
value. Its optional `max_depth` converts deeper objects/arrays into JSON strings.
Those are incompatible with this project's preservation contract.

Stanza's [JSON tests](https://github.com/open-telemetry/opentelemetry-collector-contrib/blob/c766745fdaed961d2559b1e2d5d9c468c85d3574/pkg/stanza/operator/parser/jsonparser/parser_test.go)
cover ordinary floats, optional integer parsing, nested objects, and mixed and
nested arrays. The implementation defaults numbers to float64. With
`parse_ints`, recursive conversion tries int64, then float64, then string on
conversion failure. Values outside int64 can therefore lose integer precision
even in that mode. This follows the code; specific overflow cases were not run.

Synthetic preservation regressions should include `9007199254740993`,
`18446744073709551615`, a larger integer, `0.123456789012345678901`, `1e400`,
negative zero, and nested arrays containing these values. Exact number
representation must survive or the record must fall back according to explicit
limits; never silently round or stringify a numeric field. Exercise depth just
below, at, and above the configured limit, including arrays. Exceeding a resource
limit preserves original and produces the documented limit outcome.

## Duplicate keys

The reviewed JSON entry points materialize map-like objects without an explicit
duplicate-key validator in these functions. That observation does not establish
all upstream decoder behavior, especially through dependencies. No inspected
case demonstrated duplicate rejection. Our independent requirement is stronger:
reject duplicate keys anywhere rather than silently select one value.

Synthetic regressions: duplicate root key, duplicate key inside an object inside
an array, escaped-equivalent keys (`"x"` and `"\u0078"`), and a literal dotted
key next to a nested object. The first three require raw fallback; the last is
valid and must retain both distinct structures without flattening collisions.

## Severity and timestamps

Stanza's [severity implementation](https://github.com/open-telemetry/opentelemetry-collector-contrib/blob/c766745fdaed961d2559b1e2d5d9c468c85d3574/pkg/stanza/operator/helper/severity.go)
and [tests](https://github.com/open-telemetry/opentelemetry-collector-contrib/blob/c766745fdaed961d2559b1e2d5d9c468c85d3574/pkg/stanza/operator/helper/severity_test.go)
show case-insensitive string mapping, unknown text retained with default severity,
optional text overwrite, integer mappings, and whole-valued float mappings.
The implementation rejects fractional floats and reports a missing configured
source field. It reads one configured path; it does not resolve conflicting
application aliases.

Our synthetic cases: unknown custom severity remains a field; fractional numeric
level produces no severity; ordinary business numeric level has no inferred
scale; two recognized aliases that disagree produce None; equivalent aliases
agree. Missing severity is valid. Every original field remains present.

Stanza's [time tests](https://github.com/open-telemetry/opentelemetry-collector-contrib/blob/c766745fdaed961d2559b1e2d5d9c468c85d3574/pkg/stanza/operator/helper/time_test.go)
exercise explicit seconds/milliseconds/microseconds/nanoseconds layouts over
strings and integers, fractional layouts, and an explicit tolerance for
floating-point nanosecond loss. VRL's [timestamp tests](https://github.com/vectordotdev/vrl/blob/f040d67833601f7af658b1b0f714bd04b9fa4b67/src/stdlib/parse_timestamp.rs)
exercise explicit format strings, timezone context, explicit timezone override,
DST-related examples, invalid timezone, and null rejection. These are configured
interpretations, not automatic timestamp schema detection.

Our regressions should assert exact subsecond values without an intermediate
float conversion, signed time before the epoch, equivalent RFC 3339 offsets,
invalid dates, excessive precision, overflow, explicit unit fields, and
conflicting versus equivalent timestamps. A timezone-free local time must not
inherit process timezone. Preserve ambiguous time fields and return no time.
Numeric magnitude alone does not identify a business timestamp's unit.

## Implemented regression evidence

`tests/normalization.rs` exercises exact numeric representations, malformed and
trailing data, nested and escaped-equivalent duplicate keys, nesting fallback,
Unicode escapes, BOM passthrough, dotted-key preservation, timestamp precision,
timezones, and metadata conflicts. `tests/otlp.rs` separately verifies the
explicit conversion policy when OTLP cannot represent a source value exactly.
These tests use independently authored synthetic inputs.

The BOM decision is passthrough. Decoder recursion exhaustion is classified as
`InvalidJson`; the configurable byte limit is classified as `InputLimit`.
Neither exposes a partial object. Byte decoding and framing remain collector
responsibilities and are not established by these core tests.

## Evidence limits and next steps

No upstream tests or performance benchmarks were executed in this review.
Only the linked files and selected test cases were inspected; this is not a
claim of full upstream coverage. Add the synthetic regressions above before the
corresponding behavior, then run the independent producer matrix. Collector
framing and invalid-byte cases remain integration tests outside this core.
