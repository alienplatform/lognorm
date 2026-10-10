# Local validation observations

These observations describe a development run, not the desired-state contract
or a production rollout. Source and dependency versions are recorded in git and
in the three producer lockfiles plus the root Cargo.lock.

## Environment

Apple M4 Max, aarch64 macOS. Rust 1.90.0 for the benchmarks and primary checks.
Node 25.2.1, Python 3.10, uv 0.8.11. CI separately checks Linux with Node 22 and
Python 3.12. Benchmark results from these environments are not interchangeable.

## Compatibility and transport

`python3 compatibility/run.py --refresh` passed 35 configurations. Fresh producer
output was verified before sanitizing machine-specific paths in checked fixtures.
Expected semantics are authored in the harness rather than obtained from parser
output. Follow-up changes also test actual exception fields, child context, and
tracing's native typed scalar fields.

`cargo test --all-features --locked` passed preservation, Unicode, duplicate-key,
numeric precision, ambiguity, timestamp, size/depth, and generative regressions.
The OTLP test sends protobuf records from all producer fixtures over local HTTP
and independently decodes the body and application attributes at the receiver.

This proves the local library and OTLP boundary. No live collector, storage
query, external account, or dashboard was exercised by this test.

A subsequent automated review exposed exponent-form seconds and non-color ANSI
edge cases. Both first failed direct regressions. A real Loguru time patch emits
an exponent-form timestamp, now included in the live matrix. Display cleanup
now uses anstyle-parse's public state machine. The throughput and fuzz observations below were refreshed after those fixes.

## Allocation probe

`cargo test --test allocations -- --nocapture`:

| Case | Allocations | Allocated bytes |
| --- | ---: | ---: |
| Empty | 0 | 0 |
| Plain text | 0 | 0 |
| Oversized fallback | 0 | 0 |
| Small structured record | 8 | 671 |
| ANSI cleanup | 1 | 15 |

These include parsing and obtaining the display message. They exclude OTLP
conversion. Allocation counts are checked in an isolated test executable.

## Throughput experiment

Command: `cargo bench --all-features --bench normalize -- --warm-up-time 0.2
--measurement-time 1 --sample-size 30`. Inputs are defined in
`benches/normalize.rs`, including real producer records from checked fixtures.
RUSTC and RUSTDOC were explicitly set to the stable toolchain's executables:
this machine's PATH otherwise resolves rustc to a nightly build even when
invoking stable cargo. Values below are rounded Criterion estimate intervals.

| Case | Core parsing | Parsing plus OTLP conversion |
| --- | ---: | ---: |
| Plain text | 38.8–41.6 ns | 72.7–75.4 ns |
| Malformed JSON | 199–208 ns | 248–259 ns |
| Small structured record | 1.04–1.16 µs | 1.12–1.18 µs |
| Nested request | 1.50–1.55 µs | 1.89–2.00 µs |
| Large exception | 7.77–8.51 µs | 7.81–8.25 µs |
| ANSI cleanup | 89.4–94.4 ns | 142–148 ns |
| Pino fixture | 2.03–2.18 µs | 3.29–3.43 µs |
| tracing fixture | 1.60–1.67 µs | 3.37–3.45 µs |
| Loguru fixture | 4.12–4.25 µs | 9.18–9.76 µs |
| Loguru exponent time | 3.87–3.93 µs | 9.06–9.52 µs |
| structlog fixture | 1.92–1.94 µs | 3.39–3.57 µs |
| Oversized fallback | 10.7–11.0 ns | Not measured |

These are exploratory observations, not regression thresholds. Groups run
separately, so noise can make their intervals overlap. Oversized fallback checks
length, not all bytes; its byte throughput would be misleading. Conversion
includes body and attribute allocation but excludes protobuf serialization and
network transport. No VRL comparison was run, so these numbers establish no
advantage over VRL. Comparisons to earlier runs with different compilers or
corpora are not valid regression evidence.

## Fuzz experiment

A 30-second libFuzzer smoke run, seeded with 35 real producer records and 11
independent edge cases, executed 2,080,388 inputs without a crash or preservation
assertion failure. Commands:

```sh
python3 fuzz/seed.py fuzz/corpus/release-0.1.0
cargo fuzz run normalize fuzz/corpus/release-0.1.0 -- \
  -max_total_time=30 -max_len=8192 -verbosity=0 -print_final_stats=1
```

The seeder requires an empty directory and never deletes earlier discoveries.
The target checks exact original input, complete decoded-object equality on
structured results, and absence of partial fields or metadata on fallback.
This is a short smoke run, not evidence that all inputs are safe. The checked
target, seed command, and lockfile support longer reproducible runs.
