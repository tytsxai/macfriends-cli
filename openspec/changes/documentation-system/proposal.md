# Documentation System

## Why

MacFriends already has user-facing guides, but maintainers still need one complete, code-aligned documentation system that explains architecture, deployment modes, configuration, module ownership, core logic, operations, and troubleshooting from the current repository state.

The project is beta-only and has strict readiness gates around unresolved native primitives. Documentation must make those limits explicit so future maintainers do not accidentally present buildable or fixture-tested artifacts as production-ready.

## What Changes

- Add maintainer-oriented docs for deployment, configuration, key modules, and core logic.
- Expand the architecture and operations docs with concrete runtime flow, data ownership, safety gates, and maintenance rules.
- Update the documentation index so new users, operators, and developers have an explicit reading path.
- Keep all new content aligned with the current Rust CLI, Objective-C++ agent, scripts, adapter manifest, and local Web console behavior.

## Scope

This change is documentation-only. It does not change CLI behavior, native agent behavior, packaging scripts, compatibility targets, or runtime paths.
