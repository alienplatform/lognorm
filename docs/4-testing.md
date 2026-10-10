# Compatibility and the scientific method

Compatibility requires two complementary sources of evidence:

1. Run real logging libraries in different configurations to establish what
   applications emit and whether normalization retains its meaning.
2. Read mature codec and parser implementations, including their tests, to find
   edge cases, failure behavior, and boundaries that producer examples miss.

Neither replaces the other. Handwritten regression samples test specific edge
cases; they cannot establish compatibility with a real logger.

## Source review before implementation

Before implementing or extending a parsing behavior, inspect relevant upstream
code and tests. Start with Vector codecs, VRL, and OpenTelemetry Collector
Stanza. Use the source-review method in [prior art](6-prior-art.md).

Record the pinned revision, exact source or test link, observed behavior, and
our intended behavior. Every relevant finding must produce one of:

- An independently written synthetic regression test with explicit expectations.
- A documented input boundary, with a test at the owning layer where practical.
- An explicit design decision explaining why the behavior does not apply.

Review malformed and truncated records, duplicate keys, numeric precision and
overflow, Unicode and encoding, recursion and size limits, trailing data,
escaping, timestamp units and precision, and conflicting metadata. Also inspect
how errors preserve or discard the original record and partially parsed fields.

Do not adopt upstream behavior merely because it exists. For example, replacing
nested objects with strings can violate our preservation contract. Write our
expected result before changing implementation, and verify that the regression
exposes the missing behavior when applicable.

Keep framing separate from normalization. Container envelopes, byte decoding,
split records, and multiline assembly belong to collectors. The core receives
one complete UTF-8 record. Findings about those earlier stages must clarify
integration boundaries rather than silently expand the core's responsibilities.

## Hypothesis and experiment

Source-derived regressions are in addition to the real-library matrix below.
For example, reviewing a JSON codec does not replace running Pino with numeric
and custom string levels, different timestamps, child context, and real errors.

Each scenario specifies a library version, configuration, logging call, and
expected semantic values. A small producer runs the real library. Capture its
stdout and stderr. Feed complete records through the public API and compare
messages, severity, time, and original typed application fields.

Expected values are authored independently from the parser. Never regenerate
expectations from normalized output. A fixture update cannot silently redefine
what correct behavior means.

## Two test modes

`cargo test --locked` runs fast tests against checked-in producer output. It
requires neither Node nor Python. Fixtures include producer provenance. Fixture refresh replaces machine-specific
source paths with synthetic paths after verifying fresh output byte-for-byte.
Fresh-output tests always use the original emitted records.

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
| Pino | Numeric levels, uppercase formatter, ISO time, child context, errors, mixins, nested key, multiple streams |
| Winston | JSON, timestamp, metadata, errors, text |
| Bunyan | Numeric levels, child fields, errors |
| tracing-subscriber | Nested fields, flattened fields, spans, absent time, disabled target, text |
| log / env_logger | Default text formatter, stdout collection |
| Python logging | Plain formatter, python-json-logger, renamed standard keys |
| structlog | JSON renderer, level and time processors, context, exceptions |
| Loguru | Serialized records, bound extras, exceptions, text |
| hclog | JSON, child context, errors |
| Console | Plain messages, Unicode, multiline text |

Structured producers exercise nested data, arrays, booleans, numeric values,
null, and unique synthetic scenario markers where their format supports them. Where a logger only emits Debug strings for complex values, preserve those
strings and test its native typed scalar fields separately. Do not reconstruct
objects the logger never emitted. Errors are emitted using the
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

## Adversarial transformations and safety

Truncate every structured producer fixture at every UTF-8 character boundary.
Each incomplete object must retain its exact original input and expose no partial
fields, severity, or timestamp. This systematically exercises cuts inside keys,
escapes, numbers, nested values, and real exception records.

Generate alternate decimal and exponent spellings of signed seconds. Compute
expected nanoseconds independently with integer arithmetic. Test byte limits at
exactly the record length and one byte below it, including Unicode records.
Exercise wide arrays, very long numbers, and long incomplete strings below the
default limit, in addition to deep nesting.

Generate number literals and check each decoded value against an independent
decimal library: it is a number with the literal's exact value, or the literal
itself. CI repeats the suite with serde_json's `arbitrary_precision` enabled by
the test build, as another crate could, and results must not change. A separate
test proves that linking lognorm leaves that feature off for dependents.

The fuzz target covers parsing, metadata access, OTLP conversion, and protobuf
encoding. Keep real producer records and synthetic adversarial cases as seeds.
Random byte mutation alone is unlikely to reach as many valid structured states
as the separate JSON-generating property tests.

Malformed input is an expected outcome, not a panic. Production source forbids
unsafe Rust and contains no intentional panic or unchecked unwrap. Timestamp
arithmetic uses checked operations, and JSON decoding retains its recursion
limit. These properties reduce failure risk; they do not prove that a process
cannot crash. Allocation failure, insufficient thread stack, and dependency bugs
remain possible. The byte limit bounds input size, not total memory consumption;
decoded trees and OTLP records require additional allocations. Raising the byte
limit increases that exposure. Caller-side framing and concurrency limits are
also necessary to control total collector memory.

Do not wrap normal parsing in catch_unwind and call that crash isolation. It
cannot recover from aborts or process-wide resource exhaustion. See Rust’s
[allocation failure behavior](https://doc.rust-lang.org/std/alloc/fn.handle_alloc_error.html).
Keep a minimized
reproducer for every discovered failure and turn it into a permanent regression.

## Ongoing validation

The extended-validation workflow runs every day and on pull requests. Twelve
version/runtime profiles exercise three generations of the JavaScript, Python, and Rust
producers, a Bun runtime profile, and two hclog versions. The default locked producer matrix remains a
separate release check. A successful version sweep does not broaden support to
all custom formatters or all intermediate releases.

`python3 compatibility/versions.py` runs every profile locally and requires
Node, Bun, Python, Rust, Go, npm, and uv. Select one with
`--profile node-older`, or save evidence with `--artifacts /tmp/version-results`.
The runner copies only generic producer code into temporary environments and
never edits pinned manifests or refreshes fixtures. It validates fresh output
with the same independently authored expectations as the pinned suite.

Direct logger versions are fixed in `compatibility/versions.json`. Transitive
versions resolve for each sweep, then installs use those generated lockfiles.
CI jobs retain the resolved manifests and lockfiles for 14 days, including
resolution or producer verification failures. A failed resolution may only
provide a partial lockfile; retain its workflow log as well.
Reproduce a dependency resolution by checking out the tested commit, copying its
producer into a temporary runtime directory, replacing its manifests/lockfiles
with the artifacts, and running `compatibility/run.py --runtime <runtime>
--producer-root <temporary-parent>`. A resolution, installation, or producer
failure fails the job rather than silently skipping that profile.

Daily fuzzing runs for 15 minutes; pull requests run for two minutes. Both cover
normalization, OTLP conversion, and protobuf encoding with a 64 KiB fuzz input
cap. Separate deterministic tests cover the larger default byte boundary.
The continuing corpus is restored from a versioned cache and augmented with
checked synthetic seeds. Cache eviction can reset discoveries, so every run
also retains corpus, crash artifacts, toolchain details, and logs for 14 days.
No cache or artifact is a permanent regression: minimize a failure and add an
independent checked test. Fuzz failures fail CI; never rerun merely to get green.

Only independently constructed generic records belong in this repository or its
CI artifacts. Never feed private records or private source into public fuzzing,
fixture refresh, version sweeps, or report generation. Do not keep identifying
message wording, paths, keys, identifiers, or configurations by merely replacing
a name. Reproduce the underlying format with an upstream logger and fresh,
generic test data.

`python3 compatibility/framing.py` exercises native Node console inspection with
an object spanning physical lines, alongside a JSON console call. Each physical
text line remains intact without invented fields; the JSON call preserves its
typed fields. Use `--engine bun` to exercise Bun’s different console formatting.
This demonstrates the framing boundary with actual console output
rather than pretending the parser can assemble or reinterpret arbitrary text.
