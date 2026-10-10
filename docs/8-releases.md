# Releases and maintenance

Parser changes can silently change queries and displayed messages. Releases
must make semantic changes reviewable and reproduce compatibility evidence.

## Pull requests

Changes go through pull requests with formatting, Clippy, tests, rustdoc,
producer verification, and package validation. Automated review is enabled.
Reviewers inspect ambiguous cases and preservation, not just passing snapshots.

No real user logs or private implementation details belong in the repository.
Examples use synthetic data and reserved example domains. Dependency credentials
remain in CI secrets and never appear in files, command output, or artifacts.

## Publishing

Release-plz prepares version and changelog changes in a release pull request.
Publishing runs from the default branch with a scoped registry secret. A release
must not publish code from an untrusted pull request. The initial release is
published only after CI and review have passed.

The package includes library sources, licenses, README, and design docs. Producer
environments, captured logs, and build artifacts need not ship in the crate.
Package validation compiles the actual archive rather than only the checkout.

## Compatibility changes

Document new recognized configurations, conflict-policy changes, and changes to
timestamp interpretation. Preserve source fields so consumers can migrate
queries independently from display improvements.

The producer corpus is the compatibility record. Version upgrades regenerate
producer output explicitly and keep semantic assertions independently authored.
Performance observations identify their environment and are not portable
guarantees.

Publishing uses `release_always = false`: only merging a release-plz release PR
triggers publication. Ordinary implementation merges must not publish a crate.
The workflow uses organization secrets `CRATES_IO_TOKEN` for the registry and
`REPO_ACCESS_TOKEN` for release PRs whose pushes need to trigger CI. Secret values
are never read locally or placed in public files.
