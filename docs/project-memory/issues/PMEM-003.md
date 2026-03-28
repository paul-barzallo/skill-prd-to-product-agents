# PMEM-003 - Implement repository ingestion, file typing, and fingerprinted index storage

## Type
Feature

## Priority
P0

## Objective
Ingest repository inputs, classify relevant files, compute fingerprints, and persist the initial local index used by all later commands.

## Context
Incremental behavior depends on a durable baseline scan. The CLI needs deterministic repository traversal and on-disk state before query, trace, or validation can be useful.

## Scope
- walk the repository respecting ignore rules
- classify key documents and relevant source files
- compute content fingerprints and persist index metadata
- support incremental reingest when files are unchanged or modified

## Out Of Scope
- watch-mode event loops
- language-aware symbol extraction beyond basic metadata

## Dependencies
- PMEM-002

## Technical Tasks
- implement repository walker with ignore support
- define file type classification heuristics
- persist file metadata, fingerprints, and scan timestamps
- invalidate derived entries when upstream files change
- add integration tests with representative fixtures

## Acceptance Criteria
- repeated ingests reuse persisted fingerprints
- unchanged files are not fully reprocessed
- changed files refresh their derived state deterministically

## Risks
- poor storage design will block later commands or cause stale outputs

## Definition Of Done
- a persisted index exists after ingest
- reingest behavior is tested with changed and unchanged fixtures

## Expected Artifacts
- storage schema for index state
- ingest implementation and fixture tests
- `cli-tools/project-memory-cli/src/scan.rs`
- `cli-tools/project-memory-cli/src/store.rs`
- `cli-tools/project-memory-cli/tests/project_memory_e2e.rs`