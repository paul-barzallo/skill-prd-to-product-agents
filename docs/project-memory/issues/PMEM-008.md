# PMEM-008 - Enrich structural indexing with symbols and dependency relationships

## Type
Feature

## Priority
P2

## Objective
Expand the memory model with symbol-level and dependency-aware relationships where the signal justifies the added complexity.

## Context
The plan mentions modules, imports, and dependency relationships. Those are valuable, but they should not complicate the initial release before the documentary and traceability layers prove useful.

## Scope
- language-aware symbol extraction for selected ecosystems
- import and dependency relationship capture
- richer structural retrieval and impact analysis

## Out Of Scope
- broad multi-language parser adoption without clear value
- fully generalized code intelligence platform behavior

## Dependencies
- PMEM-003
- PMEM-004
- PMEM-005

## Technical Tasks
- choose the first supported language(s)
- define structural entity schemas
- integrate optional parsing without destabilizing the MVP
- add targeted fixtures and performance checks

## Acceptance Criteria
- structural entities are persisted with provenance
- dependency-aware queries improve retrieval for supported languages
- unsupported languages degrade cleanly without hard failures

## Risks
- parser complexity can outrun the operational value of the CLI

## Definition Of Done
- one or more supported languages have tested structural enrichment
- fallback behavior is documented

## Expected Artifacts
- structural indexing module extensions
- language-specific fixtures and benchmarks
- `cli-tools/project-memory-cli/src/scan.rs`
- `cli-tools/project-memory-cli/src/query.rs`
- `cli-tools/project-memory-cli/tests/project_memory_e2e.rs`