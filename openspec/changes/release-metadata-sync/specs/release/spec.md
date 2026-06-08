# Release Spec

## ADDED Requirements

### Requirement: Current release metadata

The repository SHALL keep the current upstream package version as the `macfriends` CLI crate version.

#### Scenario: Cargo package version

- **WHEN** package metadata is inspected
- **THEN** `crates/cli/Cargo.toml` reports `version = "0.1.2"`
- **AND** the corresponding `Cargo.lock` package entry for `macfriends` reports `version = "0.1.2"`

### Requirement: Current release artifacts

User-facing package and install documentation SHALL refer to the release archive names generated from the current package version.

#### Scenario: Package output names

- **WHEN** a maintainer builds an intentional beta package with `MACFRIENDS_ALLOW_BETA_RELEASE=1`
- **THEN** the documented archive is `dist/macfriends-0.1.2-macos-arm64.tar.gz`
- **AND** the documented checksum file is `dist/macfriends-0.1.2-macos-arm64.tar.gz.sha256`

### Requirement: Beta release boundaries

The release documentation SHALL preserve the beta compatibility and readiness caveats.

#### Scenario: Current beta status

- **WHEN** users read release-facing documentation
- **THEN** MacFriends is still described as a `0.1.x` beta project
- **AND** existing compatibility, readiness, and unresolved adapter limitations remain visible.
