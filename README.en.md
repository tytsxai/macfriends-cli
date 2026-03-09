# MacFriends

MacFriends is a standalone macOS-first WeChat relationship checking CLI for Apple Silicon.

This repository is a fresh rewrite with:
- Rust CLI
- Objective-C++ preload agent
- managed WeChat app copy
- Unix domain socket RPC
- locked-version adapter model

At the current stage, the repository is production-shaped and buildable on macOS, but true relationship detection still requires version-specific reverse-engineering data to be implemented in `native/agent`.

## Production readiness gate

A build is not production-ready unless `doctor --json` or `attach --json` reports all of the following:
- `runtime_ready = true`
- `fixture_enabled = false`
- `release_blockers = []`

If `primitive_resolution` is still `unresolved`, the repository remains buildable but is not ready for a real production release.
