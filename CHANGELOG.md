# Changelog

## 0.1.0 - 2026-03-09

### Added
- Initial public structure of `MacFriends` as a macOS-first standalone CLI workspace.
- Rust CLI commands for environment checks, managed launch flow, RPC interaction, scan export, and runtime cleanup.
- Objective-C++ native agent with Unix domain socket RPC.
- Single-version adapter manifest for `WeChat 4.1.8` on `arm64`.
- Runtime readiness gates, target-status persistence, packaging workflow, and local installation scripts.

### Changed
- Standardized the codebase around Apple Silicon, `WeChat 4.1.8`, and `signature_scan` resolver metadata.

### Notes
- The repository ships a production-oriented macOS project structure.
- Real private primitive resolution is still intentionally unresolved in the adapter and returns explicit failure codes instead of fake results.
