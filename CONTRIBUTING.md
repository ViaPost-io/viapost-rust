# Contributing

Thank you for improving the ViaPost Rust SDK.

1. Use Rust 1.85.1 or newer and create a focused branch.
2. Add a failing test for behavior changes before implementation.
3. Run the complete local gate:

   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all-targets
   cargo test --doc
   scripts/check-openapi.sh
   cargo audit
   cargo package --locked
   ```

4. Never add real API keys or customer payloads to fixtures.
5. Describe compatibility and security implications in the pull request.

The vendored `openapi.yaml` is immutable for a released SDK version. Update it only alongside the
typed surface and tests, using the canonical public ViaPost contract.

