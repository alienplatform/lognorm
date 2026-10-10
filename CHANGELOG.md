# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/alienplatform/lognorm/compare/v0.1.0...v0.2.0) - 2026-10-10

### Fixed

- [**breaking**] keep exact numbers without enabling serde_json arbitrary_precision ([#3](https://github.com/alienplatform/lognorm/pull/3))

### Upgrading from 0.1.0

lognorm no longer enables serde_json's `arbitrary_precision` feature. In 0.1.0,
Cargo feature unification turned it on for every crate in a dependent's build,
and decimals inside untagged, internally tagged, and flattened types elsewhere
in the program stopped decoding from text. What changes in `fields()`:

- Integers beyond the 64-bit range and decimals with more digits than `f64`
  holds are exact literal strings, for example `"184467440737095516160"`.
  Recognition still treats them as numbers.
- Number spelling is canonical: `1.50` becomes `1.5` and `1E5` becomes
  `100000.0`. Values are unchanged.
- Numbers outside the `f64` range, such as `1e400`, make the record fall back as
  invalid JSON with `original()` intact.
- Numeric seconds timestamps with more digits than `f64` holds are left
  unresolved.

`original()` still returns the exact record. Results are the same whether or
not another crate enables `arbitrary_precision`.

## [0.1.0](https://github.com/alienplatform/lognorm/releases/tag/v0.1.0) - 2026-10-10

### Added

- normalize structured logs automatically ([#1](https://github.com/alienplatform/lognorm/pull/1))

### Other

- initialize repository
