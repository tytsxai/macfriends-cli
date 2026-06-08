## ADDED Requirements

### Requirement: Release packaging must honor adapter primitive state

`make ready` and package generation MUST fail by default when the active adapter metadata reports unresolved, blocked, or fixture primitives.

#### Scenario: unresolved adapter primitives without beta opt-in

- **GIVEN** the active adapter has `release_channel=beta`
- **AND** at least one primitive resolution is not `resolved`
- **WHEN** a maintainer runs `make ready` or `make package`
- **THEN** the command fails before producing a new package artifact
- **AND** the output explains that beta opt-in is required.

#### Scenario: explicit beta opt-in

- **GIVEN** the active adapter has unresolved primitives
- **AND** `MACFRIENDS_ALLOW_BETA_RELEASE=1` is set
- **WHEN** a maintainer runs `make ready` or `make package`
- **THEN** the command may continue
- **AND** the output identifies the artifact as beta and not production-ready.

### Requirement: User-facing readiness wording must be conservative

CLI, Web, and documentation MUST distinguish build/test/beta readiness from real runtime production readiness.

#### Scenario: runtime not production-ready

- **GIVEN** `primitive_resolution` contains `unresolved`, `blocked`, or `fixture`
- **WHEN** a user reads CLI/Web status or install/operations documentation
- **THEN** the wording says the run is blocked or beta-only
- **AND** it does not describe the tool as ready for real production scanning.
