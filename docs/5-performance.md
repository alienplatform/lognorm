# Performance

Small per-record costs become substantial at high volume. Optimizations must be
measured against realistic records without weakening data preservation.

## Measurements

Criterion benchmarks cover plain text, malformed JSON, small structured events,
nested request logs, large exceptions, and input-limit fallback. Report both
records per second and bytes per second. Run optimized builds with fixed inputs.

A separate allocation probe counts allocation calls and bytes for each case.
Measure empty/text paths separately from structured parsing. No global logging
or telemetry is enabled during measurement.

Record machine, architecture, compiler, dependency lockfile, command, corpus,
and sample settings. Numbers from different machines are not regression
thresholds. Store observations outside desired-state specifications.

## Design constraints

- Decode JSON once per record.
- Borrow raw input and selected decoded message strings.
- Allocate display cleanup only when control sequences are present.
- Do not copy the whole map to extract a handful of fields.
- Do not compile regular expressions or transformation programs per record.
- Do not learn a single format for a mixed-output process.
- Bound parsing work by input size and nesting depth.

Exact number preservation, duplicate-key detection, and cleanup have real costs.
Benchmarks include those features rather than timing an unrealistic fast path.

## Alternatives

Compare a native implementation and a compiled-once VRL program against the same
corpus and semantic expectations before claiming one is faster. Include adapter
conversion, binary size, and dependencies. JSON-only throughput does not measure
the complete normalization path.

A reusable borrowed-value representation is considered only when profiling
shows decoded allocations dominate. SIMD parsing is considered only with
representative input sizes and target architectures. Neither is assumed faster.
