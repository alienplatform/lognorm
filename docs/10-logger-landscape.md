# Logger selection and coverage

Choose formats from actual application ecosystems, then verify the library and
configuration that emits them. Popularity guides where to invest; it cannot
establish compatibility or justify guessing a format from one field.

## Dated popularity evidence

The following GitHub star counts were read from the repositories' GitHub API on
2026-10-10. They are a visibility signal, not a usage ranking. Package downloads
include transitive installs and automation; stars measure neither active
production use nor the distribution of output configurations. Built-in loggers
have no comparable package or repository metric.

| Ecosystem | Project | Stars | Role |
| --- | --- | ---: | --- |
| JavaScript / TypeScript | [Winston](https://github.com/winstonjs/winston) | 24,523 | Configurable formats and transports |
| JavaScript / TypeScript | [Pino](https://github.com/pinojs/pino) | 18,239 | Structured JSON logger |
| JavaScript / TypeScript | [Bunyan](https://github.com/trentm/node-bunyan) | 7,206 | Established JSON convention |
| Rust | [tracing](https://github.com/tokio-rs/tracing) | 6,913 | Events and spans; subscriber determines output |
| Rust | [log](https://github.com/rust-lang/log) | 2,554 | Facade; does not define a wire format |
| Rust | [slog](https://github.com/slog-rs/slog) | 1,707 | Structured ecosystem; drain determines output |
| Rust | [env_logger](https://github.com/rust-cli/env_logger) | 1,064 | Common text formatter for log |
| Python | [Loguru](https://github.com/Delgan/loguru) | 24,142 | Text or serialized record envelope |
| Python | [structlog](https://github.com/hynek/structlog) | 4,970 | Processor pipeline; renderer determines output |
| Python | [python-json-logger](https://github.com/nhairs/python-json-logger) | 276 | JSON formatter for standard logging |

The python-json-logger count belongs to its current maintainer's repository;
repository moves make it particularly unsuitable for comparing total adoption.
Python's standard `logging` and JavaScript's `console` remain important regardless
of these counts. This sample is not an exhaustive market census or evidence that
every listed library is among the most used by any particular customer base.

## Initial selection

Pino and Bunyan exercise numeric levels with different time conventions. Winston
exercises a configurable formatting pipeline and transport metadata. Run the
Node libraries themselves: TypeScript applications commonly emit through those
same runtime implementations, so the wire format is the relevant boundary.

For Rust, test tracing-subscriber JSON with nested and flattened event fields,
spans, and absent timestamps. Also run log through env_logger. A facade alone
cannot determine output, and a formatter's Debug string must remain a string.
Native typed scalar fields are checked separately. Add a structured slog drain
as a future real-library scenario before claiming compatibility with it.

For Python, test standard logging, python-json-logger field renaming and
exceptions, structlog processors and JSON rendering, and Loguru's serialized
record envelope. Test plain output as well as structured configurations.

A library's inclusion does not mean every custom formatter can be automatically
recognized. Arbitrary field renaming, custom severity scales, and human-formatted
text may only support field preservation or passthrough. The compatibility
matrix states exactly which configurations were executed.

## Expansion method

A new report starts with the producer library, version, formatter or renderer,
and relevant options. Add a synthetic real-library scenario and independently
specified expectations. Read mature codec code and tests for related edge cases.
Keep unsupported configurations explicit until the experiment passes.

Revisit selection with fresh repository and registry evidence when planning a
release. Add versions and configurations based on real reports and ecosystem
changes, rather than increasing a nominal library count with invented fixtures.
See [testing](4-testing.md) and [source review findings](9-codec-findings.md).
