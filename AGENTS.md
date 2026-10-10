# Contributing guidelines

Read `docs/0-overview.md` and the relevant numbered specifications before editing.
The docs describe the desired state. Track unfinished implementation separately.

Keep code and prose simple, with short paragraphs and whitespace between ideas.
Use idiomatic Rust, explicit types, and small focused modules. Document public APIs.
The core is synchronous and independent of collectors, transports, and storage.

Preserve original input and structured values. Do not weaken ambiguity handling
or tests to make a new producer pass. Add a real-library reproduction first.
Before extending parsing, review relevant mature codec implementations and tests.
Turn findings into synthetic regressions, documented boundaries, or explicit
decisions as described in docs/4-testing.md and docs/6-prior-art.md. This is in
addition to running real logging libraries in their supported configurations.
Never commit customer records, credentials, private source, or consumer details.

Run formatting, Clippy, tests, producer verification, and package validation.
Use pull requests and Conventional Commit titles. Publishing runs through CI.
