# MacFriends

MacFriends is an **independently maintained macOS WeChat relationship-checking CLI** for Apple Silicon, focused on local execution, controlled launch, exportable results, and production-grade operational readiness.

Project profile:
- Rust CLI + Objective-C++ agent
- macOS / Apple Silicon only
- currently locked to `WeChat 4.1.8 (arm64)`
- local-first, no cloud dependency
- built around readiness gates, diagnostics, rollback, and maintainability

## Current capabilities

The repository currently provides:
- a full CLI: `doctor`, `status`, `prepare`, `launch`, `attach`, `profile`, `contacts`, `scan`, `export`, `detach`, `cleanup`, `serve`
- a local web console with status, action buttons, log viewing, and HTTP API endpoints
- a single-version adapter manifest for `WeChat 4.1.8 + arm64 + signature_scan`
- managed app-copy preparation, ad-hoc signing, Unix domain socket IPC, and result export
- version gating, persisted target status, runtime readiness gates, and stable error codes
- local install, rollback backup slots, and packaging scripts

## Production readiness gate

A build is not production-ready unless `doctor --json` or `attach --json` reports all of the following:
- `runtime_ready = true`
- `fixture_enabled = false`
- `release_blockers = []`

If `primitive_resolution` is still `unresolved`, the project remains buildable but is not ready for a real production release.

## Install and operations

```bash
make install-local
make package
```

See `docs/install.md`, `docs/operations.md`, `docs/architecture.md`, and `docs/troubleshooting.md` for the full workflow.

`macfriends status` is the recommended day-to-day entrypoint. It summarizes lifecycle, run state, latest scan snapshots, result/log/socket paths, release blockers, and next actions.

Run `macfriends serve --open` to start the local browser console on `127.0.0.1:8765`.

## License

MIT. See `LICENSE`.
