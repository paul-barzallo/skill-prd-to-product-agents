# PMEM-005 - Build minimal traceability and impact analysis from PRD to artifacts

## Type
Feature

## Priority
P0

## Objective
Create a minimal trace graph linking PRD-style requirements to downstream documents and code artifacts, and expose the first `impact` command on top of that graph.

## Context
The original plan prioritizes traceability as a core differentiator. The MVP should implement a conservative first pass that favors explicit identifiers and references over speculative linkage.

## Scope
- detect requirement identifiers and referenced artifacts
- persist trace edges and unresolved references
- expose `trace` and `impact` outputs in JSON

## Out Of Scope
- full dependency graphs across programming languages
- confidence scoring based on AI inference

## Dependencies
- PMEM-003

## Technical Tasks
- define trace edge types and storage format
- implement requirement and artifact reference extraction heuristics
- implement reverse reachability for `impact`
- add fixtures that model PRD, ADR, spec, and code references

## Acceptance Criteria
- `trace` exposes links and unresolved references
- `impact` returns affected artifacts for a known source node
- ambiguous matches are surfaced as warnings instead of silent links

## Risks
- weak heuristics can create false confidence in trace coverage

## Definition Of Done
- trace graph persists to local storage
- `trace` and `impact` are test-covered on realistic fixtures

## Expected Artifacts
- trace and impact modules
- fixture-based tests for explicit reference flows
- `cli-tools/project-memory-cli/src/trace.rs`
- `cli-tools/project-memory-cli/tests/project_memory_e2e.rs`
- `docs/project-memory-cli-reference.md`