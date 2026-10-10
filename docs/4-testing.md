# Compatibility and the scientific method

Handwritten JSON samples cannot prove compatibility with a real logger. A test
must establish what the producer actually emits and whether the parser retains
its meaning.

## Hypothesis and experiment

Each scenario specifies a library version, configuration, logging call, and
expected semantic values. A small producer runs the real library. Capture its
stdout and stderr. Feed complete records through the public API and compare
messages, severity, time, and original typed application fields.

Expected values are authored independently from the parser. Never regenerate
expectations from normalized output. A fixture update cannot silently redefine
what correct behavior means.

## Two test modes

`cargo test --locked` runs fast tests against checked-in producer output. It
requires neither Node nor Python. Fixtures include producer provenance.

`python3 compatibility/run.py` installs locked producer dependencies, runs the
actual producers, and validates their fresh output through the Rust library.
Missing tools, missing output, unexpected record counts, and producer failures
are test failures, not skips. A separate explicit command refreshes fixtures.

Node and Python lockfiles pin transitive dependencies. Rust producers use their
own lockfile. Current pinned versions form the release compatibility contract.
Upgrading a library is a reviewed change. Additional version sweeps run
separately and cannot replace pinned tests.

## Initial matrix

| Producer | Configurations |
| --- | --- |
| Pino | Numeric levels, uppercase formatter, ISO time, child context, errors |
| Winston | JSON, timestamp, metadata, errors, text |
| Bunyan | Numeric levels, child fields, errors |
| tracing-subscriber | Nested fields, flattened fields, spans, absent time, text |
| Python logging | Plain formatter, python-json-logger, renamed standard keys |
| structlog | JSON renderer, level and time processors, context, exceptions |
| Loguru | Serialized records, bound extras, exceptions, text |
| Console | Plain messages, Unicode, multiline text |

Every structured producer emits nested data, arrays, booleans, numeric values,
null, and a unique synthetic scenario marker. Errors are emitted using the
library's real exception support rather than a manually invented error object.

## Independent preservation checks

Decode the producer's JSON independently in the test harness and compare the
complete object to `fields()`. This catches losses outside the selected metadata
fields. Verify the exact original input separately.

Regression tests cover conflicting aliases, missing fields, custom severity,
business JSON, duplicate keys, malformed JSON, invalid timestamps, large
integers, fractional numbers, nesting limits, ANSI sequences, and input limits.
Property tests generate arbitrary JSON objects and verify preservation and
panic-free fallback. Fuzzing targets untrusted UTF-8 records.

## End-to-end evidence

A transport test feeds actual producer output through normalization into OTLP,
serializes it, receives and decodes it independently, and verifies the readable
body, nested attributes, severity, and timestamp. Collector integrations also
verify storage queries and dashboard field rendering in their own repositories.
A library-level OTLP round trip alone must not be called a production dashboard
test.

## New reports

Ask for logger, version, configuration, and a sanitized example. Reproduce with
synthetic values, add a producer scenario that fails, then fix recognition. Keep
the regression permanently. Never commit customer logs, credentials, internal
identifiers, or private source excerpts.
