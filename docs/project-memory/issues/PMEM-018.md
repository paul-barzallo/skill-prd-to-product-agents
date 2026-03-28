# PMEM-018 - Extend retrieve diagnostics for provider, model, cost-risk, and fallback provenance

## Type
Feature

## Priority
P1

## Objective
Make provider-backed retrieval explainable to operators and agents by exposing which backend responded, whether fallback occurred, and how semantic scoring was produced.

## Context
As retrieval becomes provider-aware, the current output contract is too sparse for operational clarity. Diagnostics should show backend identity, model identity where relevant, fallback provenance, and whether a result involved local or remote semantic work.

## Scope
- richer retrieve diagnostics in the JSON contract
- fallback provenance and backend identity reporting
- explicit cost-risk or remote-use indicators

## Out Of Scope
- verbose transport-level tracing by default
- per-token or provider billing analytics

## Dependencies
- PMEM-015
- PMEM-016
- PMEM-017

## Technical Tasks
- extend retrieve report metadata for provider, model, and fallback provenance
- expose whether remote access occurred and whether the result used local or remote semantic scoring
- keep output deterministic and machine-readable
- add contract tests for new diagnostics fields

## Acceptance Criteria
- retrieve output identifies the active backend clearly
- fallback events are visible in structured output
- remote-use and risk context are surfaced without breaking existing JSON consumers

## Risks
- under-explained retrieval will make debugging provider behavior expensive and slow

## Definition Of Done
- retrieve diagnostics are expanded and documented
- tests cover local, fallback, and remote-flagged output cases

## Expected Artifacts
- `cli-tools/project-memory-cli/src/model.rs`
- `cli-tools/project-memory-cli/src/query.rs`
- `cli-tools/project-memory-cli/tests/project_memory_e2e.rs`
- `docs/project-memory-cli-reference.md`