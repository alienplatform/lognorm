# Local validation observations

These observations describe a development run, not the desired-state contract
or a production rollout. Source and dependency versions are recorded in git and
in the three producer lockfiles plus the root Cargo.lock.

## Environment

Apple M4 Max, aarch64 macOS. Rust 1.90.0 for the benchmarks and primary checks.
Node 25.2.1, Python 3.10, uv 0.8.11. CI separately checks Linux with Node 22 and
Python 3.12. Benchmark results from these environments are not interchangeable.

## Compatibility and transport

`python3 compatibility/run.py --refresh` passed 33 configurations. Fresh producer
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

Command: `cargo bench --bench normalize -- --warm-up-time 0.2 --measurement-time
0.5 --sample-size 10`. Inputs are defined in `benches/normalize.rs`.

| Case | Approximate time per record | Approximate records/second |
| --- | ---: | ---: |
| Plain text | 33 ns | 30 million |
| Malformed JSON | 313 ns | 3.2 million |
| Small structured record | 1.25 µs | 800,000 |
| Nested request | 1.83 µs | 550,000 |
| Large exception | 11.3 µs | 89,000 |
| Oversized fallback | 24 ns | 41 million |

The short samples are exploratory. Oversized fallback examines the length, not
all bytes; byte throughput for that case is misleading. These are core parser
measurements, excluding transport and OTLP conversion. No VRL comparison was
run, so these numbers establish no advantage over VRL.

## Fuzz experiment

A 30-second libFuzzer run, seeded with 33 producer records, executed 1,259,959
inputs without a crash or preservation assertion failure. Command:
`cargo fuzz run normalize -- -max_total_time=30 -max_len=8192 -print_final_stats=1`.

The target checks exact original input, complete decoded-object equality on
structured results, and absence of partial fields or metadata on fallback. This
is a short smoke run, not evidence that all inputs are safe. The target and
lockfile support longer reproducible runs.
