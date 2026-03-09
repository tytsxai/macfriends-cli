# Changelog

## 0.1.0 - 2026-03-09

### Added
- Rebuilt the project as `MacFriends`, a macOS-first standalone CLI workspace.
- Added a Rust CLI with `doctor`, `prepare`, `launch`, `attach`, `profile`, `contacts`, `scan`, `export`, `detach`, and `cleanup`.
- Added a native Objective-C++ agent with Unix domain socket RPC.
- Added a single-version adapter manifest for `WeChat 4.1.8` on `arm64`.
- Added target status persistence and fixed error-code gating for version mismatch and unresolved primitives.
- Added packaging and local install scripts for release workflows.

### Changed
- Removed the legacy Windows-only UI, DLL flow, and COM/driver code.
- Standardized the repository around Apple Silicon, `WeChat 4.1.8`, and `signature_scan` resolver metadata.

### Notes
- The repository now ships a production-shaped macOS project structure.
- Real private primitive resolution is still intentionally unresolved in the adapter and returns explicit failure codes instead of fake results.
