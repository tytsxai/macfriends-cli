# Security Hardening

## Why

The local web console can trigger local CLI operations, and the native agent exposes a Unix domain socket. These surfaces need minimal guardrails so normal local use stays simple while cross-site pages, unsafe output paths, stale PIDs, and loose socket directories cannot break or drive production workflows.

## What Changes

- Require a per-server token for mutating web console APIs.
- Remove broad CORS exposure from the local HTTP server.
- Keep web-triggered exports inside the MacFriends result directory.
- Treat stale PID files as live only when the process still matches the recorded MacFriends process identity.
- Force agent socket directories to owner-private permissions.

## Scope

This keeps the existing CLI commands, default paths, and local dashboard behavior. It does not add user accounts, persistent auth, TLS, or a separate web framework.
