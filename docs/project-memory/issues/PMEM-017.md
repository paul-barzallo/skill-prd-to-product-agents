# PMEM-017 - Add embedding cache invalidation and provider fallback behavior

## Type
Feature

## Priority
P1

## Objective
Reuse embeddings when safe, invalidate them when provider semantics change, and define controlled fallback behavior across local and optional remote backends.

## Context
Provider-aware metadata now makes cache behavior possible, but retrieval still needs clearer rules for when embeddings are reusable and what should happen when a preferred provider is unavailable or incompatible with persisted vectors.

## Scope
- cache reuse keyed by content and provider metadata
- invalidation on provider, model, dimension, or content changes
- configurable fallback behavior across local providers

## Out Of Scope
- full distributed cache infrastructure
- hidden automatic fallback to remote providers without explicit policy

## Dependencies
- PMEM-012
- PMEM-013
- PMEM-015

## Technical Tasks
- define cache keys using content hash plus provider metadata
- implement invalidation when provider, model, or dimensions change
- define and implement allowed fallback paths
- add tests for stale-cache and fallback scenarios

## Acceptance Criteria
- persisted embeddings are reused only when they are semantically compatible
- provider changes trigger explicit invalidation instead of silent reuse
- fallback behavior follows documented policy

## Risks
- incorrect reuse will produce misleading retrieval scores that are hard to diagnose

## Definition Of Done
- cache reuse and invalidation are deterministic and tested
- fallback rules are visible in repository documentation

## Expected Artifacts
- `cli-tools/project-memory-cli/src/store.rs`
- `cli-tools/project-memory-cli/src/query.rs`
- `cli-tools/project-memory-cli/src/model.rs`
- `cli-tools/project-memory-cli/tests/project_memory_e2e.rs`
- `docs/project-memory-cli-reference.md`