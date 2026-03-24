# PMEM-004 - Add query command for local retrieval over indexed project memory

## Type
Feature

## Priority
P0

## Objective
Allow agents to retrieve relevant local context through deterministic JSON responses instead of rereading the whole repository.

## Context
The value of project memory depends on retrieval. Once indexed data exists, the CLI should support focused lookup of documents and fragments using textual and structural filters.

## Scope
- text-based querying over indexed content and metadata
- filtering by file type, path, and memory layer
- fragment-oriented JSON results with provenance metadata

## Out Of Scope
- semantic/vector retrieval
- opaque ranking models

## Dependencies
- PMEM-003

## Technical Tasks
- define query request parameters
- implement metadata and text filtering over stored index data
- return fragments with path, offsets, and relevance explanation
- add tests covering empty, narrow, and broad query cases

## Acceptance Criteria
- query results include provenance and stable JSON formatting
- users can limit searches to specific file classes or paths
- empty results are explicit rather than silent failures

## Risks
- low-signal query output could encourage fallback to full-repo rereads

## Definition Of Done
- `query` returns deterministic JSON results over persisted state
- tests cover core retrieval modes

## Expected Artifacts
- query module implementation
- query fixtures and JSON contract tests
- `cli-tools/project-memory-cli/src/query.rs`
- `cli-tools/project-memory-cli/tests/project_memory_e2e.rs`
- `docs/project-memory-cli-reference.md`