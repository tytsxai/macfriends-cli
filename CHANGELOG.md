# Changelog

## Unreleased

暂无。

## 0.1.3 - 2026-07-28

### Added
- Release guard for `make ready` and package generation. Current unresolved adapter primitives now block production-looking artifacts unless `MACFRIENDS_ALLOW_BETA_RELEASE=1` explicitly opts into a beta/testable package.
- `scripts/backup.sh` for cold-backup of runtime/results/bundle/logs (and agent log when present).
- `rust-toolchain.toml` pins stable + rustfmt/clippy for reproducible CI/local builds.
- Operations Go/No-Go checklist for honest production readiness decisions.

### Changed
- Adapter metadata now records `release_channel=beta` and current primitive resolution state.
- CLI, Web, README, install, operations, and llms wording now distinguish beta/testable artifacts from real runtime readiness.
- CI now asserts default release-guard blocking, then packages only with `MACFRIENDS_ALLOW_BETA_RELEASE=1`.
- RPC I/O timeouts: 5s for lightweight methods, 60s for `contacts`/`scan`; timed-out reads surface as `rpc_timeout`.

### Security
- Web write operations now require a per-server session token, broad CORS headers were removed, Web export ignores caller-controlled output paths, run-state PID checks verify process identity, and the native agent socket directory is forced to `0700`.
- Sensitive runtime/result/log files are written owner-private (`0600`); agent socket and agent log are also forced to `0600`.

### Notes
- This remains a **beta** release: real WeChat private primitives are still `unresolved`. Use beta packages only with explicit opt-in; do not present as production-ready friend scanning.

## 0.1.2 - 2026-05-25

### Added
- `macfriends status` as a product-level overview for lifecycle, run state, latest scan snapshots, paths, blockers, and next actions.
- `macfriends serve` local Web console with HTTP APIs for status, prepare, launch, scan, export, detach, cleanup, and logs.
- Chinese command aliases such as `状态`, `准备`, `启动`, `扫描`, `导出`, and `控制台`.
- Chinese user guide at `docs/中文用户指南.md`.
- Runtime compatibility warnings for installed WeChat version, managed copy version, and locked adapter version.
- `/api/compatibility` and CORS preflight handling for the local Web API.
- `llms.txt` AI-search-engine index covering positioning, limits, common questions, and bilingual long-tail search phrases.
- README keyword block, English summary, release badge, navigation links, and FAQ clarifying the controlled-copy model and compatibility rails.

### Changed
- `scripts/install.sh` now supports both source-tree installation and direct installation from an extracted release package.
- Fixture smoke now covers the new status command and Web API basics.
- Web console labels and status text are now Chinese-first for local Chinese users.
- Fixture smoke now exercises all primary Web API endpoints, including expected failure paths.
- README, README.en, docs index, llms.txt, and Cargo package metadata now describe MacFriends with clearer SEO/GEO positioning, truthful compatibility limits, quick-start commands, and production readiness gates.

### Security
- CSV export now neutralizes spreadsheet-formula-looking contact fields before writing local CSV files.

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
