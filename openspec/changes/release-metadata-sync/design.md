# Design

## Version Source

`scripts/package.sh` derives release artifact names from `crates/cli/Cargo.toml`, so the Cargo package version remains the single release version source for generated package names.

## Documentation

User-facing install and package examples must show the exact archive names produced by `make package`.
Because the active adapter is still beta-only, release artifact generation is verified with `MACFRIENDS_ALLOW_BETA_RELEASE=1`; packaging without that opt-in remains blocked by `scripts/release-guard.sh`.

Historical changelog references to `0.1.0` remain intact because they describe prior release history, not current release metadata.

## Beta Status

The current release line intentionally keeps the existing beta compatibility limits: Apple Silicon macOS, WeChat 4.1.8, ad-hoc signed controlled copy, explicit beta package opt-in, explicit readiness gates, and unresolved private primitive resolution.
