# lognorm

Automatic structured log normalization for Rust.

Turn application output into a readable message, recognized severity and time,
and a complete map of typed application fields. Preserve the original record.
No per-application parser configuration is required.

## Features

- **Automatic detection.** Recognizes common logger conventions without choosing a parser.
- **Readable messages.** Extracts the message and removes terminal escape sequences for display.
- **Structured fields.** Keeps the complete JSON object, including nested objects, arrays, and exact numbers.
- **Severity and timestamps.** Recognizes standard levels and source timestamps, with nanosecond precision.
- **Original input preserved.** Access the exact record alongside its normalized values.
- **Safe fallbacks.** Plain text passes through; malformed, duplicate-key, or oversized JSON retains its original input.
- **Optional OTLP output.** Convert normalized logs into OpenTelemetry protobuf records.
- **Small synchronous API.** No async runtime, network, or storage required. Plain-text and oversized fallback paths allocate nothing.

## Supported logging libraries

Tested against output from the actual libraries across **35 configurations**:

**JavaScript and TypeScript**

- [Pino](https://github.com/pinojs/pino) — JSON output, child loggers, errors, renamed message fields, and timestamp options.
- [Winston](https://github.com/winstonjs/winston) — JSON output, timestamps, metadata, and errors.
- [Bunyan](https://github.com/trentm/node-bunyan) — JSON output, child loggers, and errors.

**Rust**

- [tracing-subscriber](https://github.com/tokio-rs/tracing) — JSON output with nested or flattened event fields, span context, and optional timestamps.

**Python**

- [python-json-logger](https://github.com/nhairs/python-json-logger) — standard `logging` with JSON formatting, renamed fields, and exceptions.
- [structlog](https://github.com/hynek/structlog) — JSON output, timestamps, and exceptions.
- [Loguru](https://github.com/Delgan/loguru) — serialized JSON output and exceptions.

Plain-text output from console logging, Python `logging`, `env_logger`, and other
text loggers passes through without reconstructing structured fields.
Unknown JSON objects still expose all their fields, even when their message,
severity, or timestamp conventions are not recognized.

Support covers the tested configurations, not every custom formatter. Conflicting
metadata stays unresolved, and JSON strings inside fields are not recursively
parsed. See the [recognition rules](docs/2-recognition.md) and
[compatibility tests](compatibility/).

## Quick start

```rust
use lognorm::{parse, Severity};

let input = r#"{"level":"WARN","msg":"Request rejected","statusCode":401,"req":{"path":"/v1/check"}}"#;
let log = parse(input);

assert_eq!(log.message(), "Request rejected");
assert_eq!(log.severity(), Some(Severity::Warn));
assert_eq!(log.fields()["statusCode"], 401);
assert_eq!(log.fields()["req"]["path"], "/v1/check");
assert_eq!(log.original(), input);
```

Your log viewer can show **Request rejected** while retaining `statusCode` and
`req.path` as structured fields for filtering and queries.

Pass one complete UTF-8 record after framing. The default input limit is 1 MiB;
use `Parser::with_max_bytes` to change it. See the [API guide](docs/3-api.md).

Requires Rust 1.88 or newer. The first release is in development and has not yet
been published to crates.io.

## OTLP

Enable the optional `otlp` feature to build OpenTelemetry protobuf log records:

```rust,ignore
let parsed = lognorm::parse(line);
let mut record = lognorm::otlp::log_record(&parsed);
record.observed_time_unix_nano = observed_time;
// Attach resource/scope context and send with your collector's OTLP exporter.
```

The body contains readable text. The `app` attribute contains nested application
fields, separate from trusted collector context. Unsupported OTLP numeric values
use exact strings; JSON null uses an empty AnyValue. The caller supplies fallback
metadata, observed time, batching, and transport. See [integration](docs/7-integration.md).

## Verification

```sh
cargo test --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
python3 compatibility/run.py
cargo bench --all-features --bench normalize
cargo test --test allocations -- --nocapture
# With nightly Rust and cargo-fuzz installed, choose an empty corpus directory:
python3 fuzz/seed.py fuzz/corpus/new-run
cargo fuzz run normalize fuzz/corpus/new-run -- -max_total_time=30
```

The compatibility command needs Rust, Node, Python, npm, and uv. It installs
locked dependencies and runs 35 real logger configurations. Fast Rust tests
replay sanitized producer fixtures. `python3 compatibility/run.py --refresh`
explicitly refreshes verified fixtures; review their diff before committing.

The method combines upstream codec source/test review, real-library execution,
independent preservation checks, generative regressions, and OTLP HTTP round
trips. A local OTLP test does not establish production storage or dashboard
integration. See [testing](docs/4-testing.md) and [codec findings](docs/9-codec-findings.md).

## Design and contribution

The numbered [design documents](docs/0-overview.md) describe the desired state.
Read [AGENTS.md](AGENTS.md) before contributing. Submit a synthetic real-library
reproduction for a new format; never submit customer logs or credentials.

Licensed under either [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT).
