# PMEM-010 - Track RAG provider initiative for project-memory-cli

## Type
Epic

## Priority
P0

## Objective
Track the provider-focused RAG initiative for `project-memory-cli` now that the v1 indexing, chunking, retrieve contract, and local hashed embeddings already exist.

## Context
The repository no longer needs a generic RAG-foundation backlog. SQLite persistence, deterministic chunking, chunk-aware retrieval, and local hashed embeddings are already implemented. The next initiative is narrower: provider architecture, local-first configuration, optional remote bridging, and explicit safety boundaries for networked backends.

## Scope
- create and maintain the provider-focused tracker issue
- group provider, config, safety, cache, and diagnostics work under one initiative
- make dependency order explicit from architecture through validation

## Out Of Scope
- re-planning chunking, SQLite mirroring, or the existing `retrieve` command from scratch
- packaging or runtime workspace integration beyond repository-side tooling

## Dependencies
- none

## Technical Tasks
- create the GitHub tracker for the provider initiative
- create the child issues from PMEM-011 through PMEM-019
- keep the tracker body aligned with issue state and sequencing

## Acceptance Criteria
- the initiative has one tracker issue in GitHub
- all provider child issues are linked from the tracker
- the tracker states that default behavior remains local and no-cost

## Risks
- the second wave can drift back into already-implemented backlog if the tracker does not keep the baseline explicit

## Definition Of Done
- tracker issue exists in GitHub
- child issue list and sequencing are visible from the tracker

## Expected Artifacts
- GitHub tracker issue for the provider initiative
- `docs/project-memory/issues/PMEM-011.md`
- `docs/project-memory/issues/PMEM-012.md`
- `docs/project-memory/issues/PMEM-013.md`
- `docs/project-memory/issues/PMEM-014.md`
- `docs/project-memory/issues/PMEM-015.md`
- `docs/project-memory/issues/PMEM-016.md`
- `docs/project-memory/issues/PMEM-017.md`
- `docs/project-memory/issues/PMEM-018.md`
- `docs/project-memory/issues/PMEM-019.md`