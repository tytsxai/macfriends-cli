# Security Spec

## ADDED Requirements

### Requirement: Mutating web APIs require a local token

The local web console SHALL require a per-server token for mutating HTTP APIs.

#### Scenario: Cross-site POST is rejected

- **WHEN** a request calls `POST /api/scan` without the current `X-MacFriends-Token`
- **THEN** the server returns an error response
- **AND** the CLI command is not executed

### Requirement: Web console binds only to loopback

The local web console SHALL reject non-loopback listen addresses.

#### Scenario: Public bind address is rejected

- **WHEN** `macfriends serve` is started with `--addr 0.0.0.0:8765`
- **THEN** the server exits before binding the listener
- **AND** the error explains that only loopback addresses such as `127.0.0.1` or `[::1]` are allowed

### Requirement: Web exports stay in the result directory

The web console SHALL NOT accept caller-controlled export output paths.

#### Scenario: Export from dashboard

- **WHEN** the dashboard calls `POST /api/export`
- **THEN** the export command runs without a user-provided `--output`
- **AND** the CLI writes the file to the default MacFriends result directory

### Requirement: Invalid web requests fail before command execution

The local web console SHALL reject malformed HTTP requests and malformed JSON bodies as client errors before invoking CLI commands.

#### Scenario: Malformed JSON body is rejected

- **WHEN** a request calls `POST /api/scan` with the current `X-MacFriends-Token` but an invalid JSON body
- **THEN** the server returns `400`
- **AND** the response uses `error_code=web_bad_request`
- **AND** the scan command is not executed

#### Scenario: Truncated HTTP body is rejected

- **WHEN** a request declares a `Content-Length` larger than the bytes actually sent
- **THEN** the server returns or records a bad request error instead of executing the API with a partial or empty body

### Requirement: Runtime PID checks verify process identity

The CLI SHALL avoid treating an unrelated reused PID as a live controlled process.

#### Scenario: Stale run-state PID was reused

- **WHEN** `run-state.json` contains a PID that is running but no longer matches the recorded MacFriends executable identity or socket ownership
- **THEN** stale runtime cleanup may remove the run-state files

### Requirement: Agent socket directory is private

The native agent SHALL make the socket parent directory owner-private.

#### Scenario: Agent creates socket directory

- **WHEN** the agent creates or reuses the socket parent directory
- **THEN** it sets the directory permissions to `0700`
