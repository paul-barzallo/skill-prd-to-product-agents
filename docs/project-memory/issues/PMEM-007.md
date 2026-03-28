# PMEM-007 - Add watch mode for selective refresh on file changes

## Type
Feature

## Priority
P1

## Objective
Add a watch workflow that refreshes only affected indexed and derived memory artifacts when repository files change.

## Context
The original plan includes watch mode, but it should build on proven fingerprint invalidation from the MVP rather than define refresh semantics prematurely.

## Scope
- watch relevant files for change events
- trigger selective reingest and derived-state refresh
- report changed nodes and invalidated outputs clearly

## Out Of Scope
- background service management outside a single command invocation
- advanced debouncing across multiple repositories

## Dependencies
- PMEM-003
- PMEM-005
- PMEM-006

## Technical Tasks
- implement a watch entrypoint using a file notification crate
- map file changes to refresh scopes
- preserve deterministic output and exit behavior
- add tests or harnesses for basic event-driven refresh behavior

## Acceptance Criteria
- watch mode refreshes only affected paths on simple edits
- derived state invalidation is visible in command output
- failure modes remain explicit when watched paths disappear

## Risks
- event-driven edge cases can make the system appear flaky if added too early

## Definition Of Done
- watch mode works on representative local fixtures
- documentation explains its limits and expectations

## Expected Artifacts
- watch module
- watch mode usage docs and basic verification coverage
- `cli-tools/project-memory-cli/src/watch.rs`
- `cli-tools/project-memory-cli/tests/project_memory_e2e.rs`
- `docs/project-memory-cli-reference.md`