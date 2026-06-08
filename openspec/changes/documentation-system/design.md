# Design

## Documentation Structure

The docs directory is organized by responsibility:

- `README.md`: navigation and ownership map.
- `architecture.md`: system design, runtime flow, data flow, trust boundaries, and extension points.
- `deployment.md`: local source, release package, container/static verification, and server/headless boundaries.
- `configuration.md`: paths, environment variables, manifest fields, CLI/Web options, generated state, and retention.
- `modules.md`: file-level module map and core logic.
- `operations.md`: release gates, backup, rollback, health checks, logs, and runbooks.
- `troubleshooting.md`: symptom-to-action guide and error codes.

## Source Of Truth

Docs should reference concrete files and commands from the repository instead of inventing future architecture:

- CLI contracts come from `crates/cli/src/cli.rs` and `crates/cli/src/model.rs`.
- Runtime paths come from `crates/cli/src/layout.rs`.
- Web API behavior comes from `crates/cli/src/web.rs`.
- Packaging behavior comes from `Makefile`, `scripts/package.sh`, `scripts/install.sh`, and `scripts/release-guard.sh`.
- Native runtime behavior comes from `native/agent/src/*.mm`.
- Compatibility and readiness metadata come from `fixtures/adapter.wechat-macos-arm64.json`.

## Maintenance Rule

Any change that touches commands, paths, JSON fields, adapter metadata, packaging behavior, Web APIs, scan/export behavior, logs, or readiness gates must update the corresponding docs in the same change.
