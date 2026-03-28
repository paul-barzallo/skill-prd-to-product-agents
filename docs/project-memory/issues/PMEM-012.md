# PMEM-012 - Add provider abstraction and provider-aware embedding metadata

## Type
Feature

## Priority
P0

## Objective
Extract the embedding backend into an explicit provider layer and persist enough metadata to make embeddings reproducible and invalidatable.

## Context
The current implementation already persists chunk embeddings, but long-term provider support requires richer metadata than a bare vector row. Retrieval and cache reuse should depend on provider identity, model identity, dimensions, and content hash rather than hidden assumptions.

## Scope
- provider-aware embedding service abstraction
- richer persisted metadata for embeddings and snapshot state
- compatibility path for the existing local hashed backend

## Out Of Scope
- provider-specific HTTP contracts
- remote safety gates and spend protection

## Dependencies
- PMEM-011

## Technical Tasks
- formalize the embedding provider abstraction used by ingest and retrieve
- persist provider, model, dimensions, and content hash metadata in SQLite
- expose provider metadata through structured retrieval diagnostics where appropriate
- add tests for provider-aware persistence and cache matching

## Acceptance Criteria
- ingest and retrieve do not rely on a hardcoded provider path
- persisted embeddings carry enough metadata for cache invalidation and reproducibility
- provider mismatch is explicit rather than silent

## Risks
- partial metadata will make cache invalidation unreliable once remote providers are added

## Definition Of Done
- provider-aware metadata is persisted and used in retrieval behavior
- tests cover provider mismatch and compatible reuse paths

## Expected Artifacts
- `cli-tools/project-memory-cli/src/embeddings.rs`
- `cli-tools/project-memory-cli/src/store.rs`
- `cli-tools/project-memory-cli/src/model.rs`
- `cli-tools/project-memory-cli/src/query.rs`
- `cli-tools/project-memory-cli/tests/project_memory_e2e.rs`