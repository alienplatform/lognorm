# Prior art

Many tools decode logs, but fewer expose automatic, lossless interpretation as
a small reusable library. The decision to maintain normalization must be based
on source-level comparison rather than package descriptions.

## Ongoing source-review method

Prior-art review is part of development, not only an initial dependency survey.
Read implementations and their tests before adding parsing behavior. Inspect
Vector's decoding codecs, VRL's parsing functions, and Stanza's JSON, timestamp,
and severity operators. Follow their calls into shared helpers when those
helpers determine limits, coercion, precision, or error handling.

For each relevant finding, record:

- The upstream project, pinned commit, and exact implementation or test link.
- The input condition and observed result, including any data loss or fallback.
- The intended result here and why it fits the preservation and ambiguity rules.
- The regression test, integration boundary, or design decision that follows.

Maintain this evidence alongside the relevant tests or numbered specification.
Distinguish behavior established by reading code from behavior verified by
running an upstream test or reproducer. Do not claim an entire upstream test
suite was reviewed after inspecting a few cases.

Create synthetic cases independently. Do not copy upstream source or test
fixtures without reviewing their license and retaining any required notices.
No customer data is needed for these experiments.

This work supplements actual logger execution. Codec tests establish robustness
and expose design tradeoffs; producer tests establish compatibility with real
library versions and configurations. Both are required by the
[testing method](4-testing.md).

## lnav

[lnav](https://github.com/tstack/lnav) is a mature C++ viewer with automatic
format recognition and declarative built-in definitions. Its
[Pino definition](https://github.com/tstack/lnav/blob/313080f43be95bc54de409a9c16048b201f6cb7f/src/formats/pino_log.json)
maps numeric severity, selects `msg`, and declares millisecond timestamps.
Its [Bunyan definition](https://github.com/tstack/lnav/blob/313080f43be95bc54de409a9c16048b201f6cb7f/src/formats/bunyan_log.json)
selects ISO time and nested source locations. The
[env_logger definition](https://github.com/tstack/lnav/blob/313080f43be95bc54de409a9c16048b201f6cb7f/src/formats/env_logger_log.json)
extracts default text fields with a regular expression.

These definitions demonstrate that shipped knowledge can provide automatic
behavior. The viewer engine includes functionality beyond a collector's needs.
Use its conventions as references. Copying definitions requires preserving the
applicable BSD-2-Clause notices; this project writes its own implementation.

## hl

[hl](https://github.com/pamburus/hl) is a Rust viewer. Its
[model code](https://github.com/pamburus/hl/blob/0df0e54b200e11709dc92709b3297e30cb673e56/src/model.rs) separates
raw decoding from semantic interpretation and uses borrowed raw values. Its
[defaults](https://github.com/pamburus/hl/blob/0df0e54b200e11709dc92709b3297e30cb673e56/etc/defaults/config.toml)
define message and timestamp aliases and severity maps.

The library exports some parser types, but the full record model is not an
isolated normalization API. Its root manifest includes terminal, compression,
paging, configuration, filesystem, and build dependencies. Default numeric
mapping targets systemd priority rather than Pino levels. This is a useful
performance and architecture reference, not a direct dependency.

## Vector and VRL

[VRL](https://github.com/vectordotdev/vrl) is an embeddable Rust transformation
engine. Its standard library provides JSON, logfmt, syslog, Grok, and timestamp
functions. The inspected [JSON decoder](https://github.com/vectordotdev/vrl/blob/f040d67833601f7af658b1b0f714bd04b9fa4b67/src/stdlib/parse_json.rs)
uses serde_json. Its optional depth handling converts deeper structures into
strings rather than rejecting the record, a preservation tradeoff. It does not remove the need to define message aliases, severity
scales, envelope detection, or ambiguity behavior.

Vector's internal [codecs crate](https://github.com/vectordotdev/vector/blob/master/lib/codecs/Cargo.toml)
is unpublished and depends on several workspace event/configuration crates.
Taking that crate would introduce a larger event model than this API requires.

A VRL alternative ships built-in source programs, compiles once at startup,
and runs the same producer tests. End users need not write VRL. Compare measured
costs and preservation before rejecting or adopting this approach. Upstream
code is MPL-2.0; no source is copied into this implementation.

## OpenTelemetry Stanza

The Go collector's [parser operators](https://github.com/open-telemetry/opentelemetry-collector-contrib/blob/c766745fdaed961d2559b1e2d5d9c468c85d3574/pkg/stanza/docs/types/parsers.md)
preserve typed JSON and compose body, time, severity, and trace extraction.
Semantic operators generally require configured field paths and time layouts.
Its [severity support](https://github.com/open-telemetry/opentelemetry-collector-contrib/blob/c766745fdaed961d2559b1e2d5d9c468c85d3574/pkg/stanza/docs/types/severity.md)
separates mapping from decoding. This separation is useful, but embedding the
collector solely for field recognition adds process and deployment costs.

## Other systems

Fluent Bit can automatically merge JSON application output, but parser and time
configuration still determine interpretation. Datadog's public agent parsers
focus on container and transport envelopes; backend integration pipelines
provide application remapping. Quickwit document mapping and OpenObserve
ingestion are storage-oriented, with configured transforms rather than an
isolated automatic library.

[cor](https://github.com/alexsavio/cor-cli/blob/b2b043dc6990f8f9ad6347c0a8b4d16ca99ab3bd/src/parser.rs) is a smaller
Rust viewer with automatic aliases. It converts errors to strings and flattens
objects for display. [loglens-core](https://github.com/Caelrith/loglens-core/blob/main/src/parsers/mod.rs)
automatically tries JSON, access logs, and logfmt, but JSON decoding itself does
not supply application metadata normalization.

## Decision

Reuse a standard JSON decoder and implement a small, conservative semantic
layer. Preserve the complete application object and test conventions using real
libraries. Keep the architecture open to a different execution engine if the
same compatibility and performance experiments demonstrate a better choice.

The claim is not that automatic parsing is new. The required combination is a
small embeddable API, automatic conventions, data preservation, and independently
reproducible compatibility evidence.
