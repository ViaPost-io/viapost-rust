# Changelog

All notable changes follow [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.1.1] - 2026-09-16

### Fixed

- Bound transitive ICU, IDNA and yoke resolution to versions compatible with the documented Rust
  1.85 MSRV.
- Validate a lockfile-free consumer on the MSRV in CI and before every release.

## [0.1.0] - 2026-09-16

### Added

- Async `ViaPost` client for Rust 1.85+ using Tokio, reqwest and rustls.
- Typed resources for send, messages, domains, templates, webhooks, automations and usage.
- Content-aware message details and authenticated RFC 5322 source downloads.
- Webhook endpoint updates, delivery inspection, tests, replays and secret rotation.
- Safe GET/HEAD retry policy, idempotency keys and pagination parameters.
- HTTPS-by-default transport, 8 MiB decoded response limit and redacted credentials.
- Vendored OpenAPI contract with deterministic drift verification.
- GitHub-first release archives, checksums and build provenance.

[Unreleased]: https://github.com/ViaPost-io/viapost-rust/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/ViaPost-io/viapost-rust/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/ViaPost-io/viapost-rust/releases/tag/v0.1.0
