# Changelog

## 0.1.1-beta - 2026-05-19

### Added (Documentation)

- **`llms.txt`** — AI-search-engine index covering positioning, what it does / doesn't do, common questions, and long-tail search phrases (中英文).
- **README — keyword block + English summary** at the top so non-Chinese searchers can find the tool.
- **README — 6-question FAQ** clarifying that this is *not* a "who deleted me on WeChat" one-click hack tool; explaining the controlled-copy + ad-hoc-signing model; calling out the Apple-Silicon-only + WeChat-4.1.8-only safety rails.
- **README header — Release badge + nav row** (llms.txt / Issues / License).

### Notes

Documentation-only release. No code, no agent, no command surface changes since 0.1.0.

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
