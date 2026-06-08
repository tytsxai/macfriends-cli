## ADDED Requirements

### Requirement: Maintainer documentation map

The repository SHALL provide a documentation map that covers deployment, configuration, module ownership, operations, troubleshooting, architecture, compatibility, installation, and the Web console.

#### Scenario: Maintainer finds the right document

- **WHEN** a maintainer opens `docs/README.md`
- **THEN** it links to deployment, configuration, modules, operations, troubleshooting, architecture, compatibility, install, Web console, and Chinese user guide documentation
- **AND** each linked document describes its audience and responsibility.

### Requirement: Documentation follows implementation facts

Documentation SHALL describe current repository behavior instead of future or aspirational behavior.

#### Scenario: Runtime behavior changes

- **WHEN** CLI commands, JSON fields, runtime paths, Web API behavior, adapter metadata, release guards, packaging, scan/export behavior, logs, or readiness gates change
- **THEN** the corresponding `docs/` pages are updated in the same change
- **AND** the docs reference concrete commands, paths, files, and readiness fields.

### Requirement: Beta readiness remains explicit

Documentation SHALL keep beta-only and unresolved primitive boundaries visible.

#### Scenario: Adapter primitives are unresolved

- **GIVEN** the adapter manifest reports `release_channel=beta`
- **AND** one or more primitives are not `resolved`
- **WHEN** users read README, install, deployment, operations, compatibility, troubleshooting, or module docs
- **THEN** the docs say the artifact is beta/testable
- **AND** they do not describe the current state as real production-ready scanning.
