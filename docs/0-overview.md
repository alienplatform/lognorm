# Overview

Applications emit useful structured logs, but collectors often store the whole
JSON object as a string. People then see JSON syntax instead of a readable
message, and cannot query nested fields without parsing each result again.

lognorm turns one complete application record into a readable message, optional
severity and timestamp, and the original structured fields. Recognition is
automatic. Applications do not select a parser or change their logging library.

```text
application output
       |
       v
collector framing       assemble one complete UTF-8 record
       |
       v
lognorm                 decode once, recognize conventions, preserve fields
       |
       v
collector adapter       attach trusted context, encode structured telemetry
       |
       v
storage and export      readable message and queryable typed fields
```

For example, a structured warning can become `Request rejected` in a log list,
while `statusCode: 401` and `req.path: /v1/check` remain queryable. Neither the
message nor the extraction replaces the original application data.

## Product contract

- Common structured logger configurations work without setup.
- Every supported configuration is exercised using the actual logging library.
- Unknown JSON objects still expose their fields.
- Unrecognized text stays readable and unchanged.
- Ambiguous metadata stays unspecified rather than receiving a guessed meaning.
- A malformed or oversized record does not interrupt the caller.
- No network, filesystem, process-global state, or asynchronous runtime is needed.

## Scope

The first release covers structured output from Pino, Winston, Bunyan, Rust
tracing, Python logging with a JSON formatter, structlog, and serialized Loguru.
Ordinary console and text logger output passes through unchanged.

The library does not reconstruct fields from arbitrary custom text. Container
framing, multiline assembly, transport, indexing, retention, and presentation
remain the caller's responsibility. Already structured telemetry does not need
another application-log parsing pass.

## Reading order

1. [Architecture](1-architecture.md)
2. [Recognition and preservation](2-recognition.md)
3. [Public API](3-api.md)
4. [Compatibility and the scientific method](4-testing.md)
5. [Performance](5-performance.md)
6. [Prior art](6-prior-art.md)
7. [Collector and dashboard integration](7-integration.md)
8. [Releases and maintenance](8-releases.md)

These documents describe the desired behavior. A missing implementation is a
gap to close, not a reason to weaken the contract. Open pull requests and issues
track implementation status separately.

The [logger landscape](10-logger-landscape.md) records the selection rationale
and dated popularity evidence. The [codec findings](9-codec-findings.md) connect
source and test review to preservation decisions and regressions.
