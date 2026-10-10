# Architecture

Different collectors must interpret the same application record consistently.
Embedding collector dependencies in a parser makes that difficult and increases
the cost of using it in small runtimes.

## Boundaries

The core is a synchronous Rust library. Its only production responsibilities are
decoding a JSON object, recognizing semantic fields, and exposing the result.
It has no dependency on a telemetry SDK, deployment system, or storage backend.

The caller supplies one complete record as `&str`. The caller owns byte decoding,
stream boundaries, container wrappers, and partial-line reconstruction.

The parser borrows the original input. It decodes JSON at most once and owns the
decoded map. Message, severity, and timestamp are projections of that map. The
map is never flattened or destructively normalized.

## Processing

1. Reject semantic parsing when the configured byte limit is exceeded.
2. Check whether trimmed input looks like a JSON object.
3. Decode with bounded nesting and exact numeric preservation.
4. Recognize supported envelopes and common field conventions.
5. Resolve each metadata category independently.
6. Return the fields and original bytes regardless of metadata recognition.

Text, invalid JSON, duplicate object keys, and unsupported depth return a
classified fallback. Callers can count these outcomes without logging the log
itself. A timestamp failure does not prevent message or field extraction.

## Stateless recognition

Recognition runs per record. A process may mix libraries, text, exceptions, and
structured events. Learning one format from the first record must not determine
the interpretation of all later records.

Known envelope handling is narrow. For example, a serialized Loguru record has
its message and metadata under `record`, while tracing may put its message under
`fields`. Unrelated nested objects are not searched recursively for a message.

## Resource control

`Parser` carries immutable limits and is safe to share across threads. The
default maximum input size is 1 MiB. The JSON decoder retains its recursion
limit. Exceeding a limit returns the original input without decoded fields.

The byte limit bounds parsing, not the caller's transport allocation. Callers
must also limit input buffering. No automatic malformed-JSON repair, recursive
decoding of strings, or regex search across arbitrary input is performed.
