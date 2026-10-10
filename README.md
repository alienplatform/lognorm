# lognorm

Automatic structured log normalization for Rust.

Turn application output into a readable message, recognized severity and time,
and a complete map of typed application fields. Preserve the original record.
No per-application parser configuration is required.

```rust
use lognorm::{parse, Severity};

let input = r#"{"level":"WARN","msg":"Request rejected","statusCode":401}"#;
let log = parse(input);

assert_eq!(log.message(), "Request rejected");
assert_eq!(log.severity(), Some(Severity::Warn));
assert_eq!(log.fields()["statusCode"], 401);
assert_eq!(log.original(), input);
```

The synchronous core has no runtime, network, or storage dependency. Pass one
complete UTF-8 record after framing. The default input limit is 1 MiB; use
`Parser::with_max_bytes` to change it.

## Recognition

Tested conventions include Pino, Bunyan, Winston JSON, tracing-subscriber JSON,
python-json-logger, structlog JSON, and serialized Loguru. Plain text passes
through. Conflicting recognized metadata stays unresolved. Custom formats can
still retain all JSON fields even when metadata cannot be inferred.

Objects, arrays, booleans, nulls, and exact JSON numbers remain intact. Duplicate
keys and malformed or excessively nested JSON fall back to the original record.
ANSI cleanup affects only the display message. JSON strings inside fields are
not recursively parsed. See [recognition rules](docs/2-recognition.md).

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
cargo bench --bench normalize
cargo test --test allocations -- --nocapture
```

The compatibility command needs Rust, Node, Python, npm, and uv. It installs
locked dependencies and runs 33 real logger configurations. Fast Rust tests
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
