# Honest Release Path For Unresolved Adapter Primitives

## Why

The current `wechat_4_1_8_arm64` adapter still fails real private primitive calls conservatively with `*_primitive_unresolved`. Build, fixture smoke, and packaging can pass, but that must not be confused with a production-ready release.

## What Changes

- Add machine-readable adapter release metadata for the current primitive state.
- Add a release guard used by `make ready` and packaging.
- Block release-style packaging by default while real primitives are unresolved.
- Allow explicit beta packaging with `MACFRIENDS_ALLOW_BETA_RELEASE=1`.
- Update README, install, operations, and CLI/Web wording so beta/buildable/testable is not overstated as production-ready.

## Impact

- Maintainers get an early failure before producing artifacts that look GA-ready.
- Beta packages remain possible when the maintainer deliberately opts in.
- Users see clearer language around "ready" and "production" output.
