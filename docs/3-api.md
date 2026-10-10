# Public API

Callers need a small stable API that does not expose detection internals or
force a telemetry dependency.

```rust
let log = lognorm::parse(line);

let message = log.message();
let severity = log.severity();
let timestamp = log.timestamp();
let fields = log.fields();
let original = log.original();
let outcome = log.outcome();
```

`parse` uses default limits. `Parser::default()` provides the same behavior and
can be reused. `Parser::with_max_bytes(n)` changes the input size bound without
choosing a logger or schema.

`ParsedLog` owns decoded fields and borrows original input. Metadata accessors
borrow existing data where possible. No `into_owned` API is needed until a real
consumer requires the parsed record to outlive its input.

`Severity` is the six common bands: trace, debug, info, warn, error, fatal.
Adapters map bands into their target model. Original level text and numbers
remain accessible through `fields()`.

`Timestamp` exposes signed Unix seconds and a validated nanosecond fraction.
It does not assume that every target SDK can represent every supported time.

`ParseOutcome` distinguishes structured JSON, ordinary text, invalid JSON,
duplicate keys, and input-limit fallback. It is non-exhaustive so callers can
handle future outcomes without source incompatibility.

## Caller ownership

The caller determines fallback severity and timestamp. The parser does not know
whether a record came from stdout, stderr, a file, or a network endpoint.

Trusted context is attached outside the library. An application's `service`,
deployment, host, or resource-like fields must not replace the collector's
authenticated context.

Public types and methods have rustdoc examples. Examples run under `cargo test`.
Changes to ambiguity handling and preservation are reviewed as behavioral API
changes, even when function signatures stay the same.
