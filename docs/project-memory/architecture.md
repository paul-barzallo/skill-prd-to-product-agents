# Project Memory CLI Architecture Review

## Goal

Build a practical Rust CLI that reduces repeated repository rereads, produces stable JSON outputs for agents, and maintains a local memory/index layer that can evolve incrementally as project files change.

## Fit With This Repository

This repository already organizes Rust CLIs under `cli-tools/`, each with a clear scope. The most coherent placement for this initiative is a new crate at `cli-tools/project-memory-cli/`.

That choice keeps the scope explicit:

- `skill-dev-cli`: repository maintenance and validation
- `prd-to-product-agents-cli`: package/bootstrap validation
- `prdtp-agents-functions-cli`: runtime workspace operations
- `project-memory-cli`: local project memory, indexing, traceability, and retrieval

## MVP Objectives

The v1 should do the following well:

1. scan a repository while respecting ignore rules
2. classify relevant documents and source files
3. generate stable fingerprints for incremental refresh
4. persist a local index/memory store on disk
5. answer JSON-based queries over indexed content
6. emit minimal traceability links from PRD-style requirements to downstream artifacts
7. validate missing coverage and obvious consistency gaps
8. expose a deterministic CLI contract that agents can call safely

## Explicitly Out Of Scope For P0

- vector databases or external services
- full graph database storage
- tree-sitter-based language parsing across many ecosystems
- long-running background daemons as the primary execution model
- autonomous summarization pipelines that cannot be reproduced deterministically
- invasive repository mutation flows

## Internal Modules

Proposed module split for the future crate:

- `cli`: command parsing and output modes
- `scan`: repository walking, ignore handling, file typing
- `index`: structural index and fingerprints
- `store`: on-disk persistence for index and derived memory artifacts
- `query`: textual and structural retrieval
- `trace`: requirements, artifacts, and code-link relationships
- `validate`: coverage and consistency checks
- `impact`: reverse reachability over known trace links
- `watch`: follow-up incremental refresh entrypoint
- `json`: stable response envelopes and schema versions

## Data Model Layers

The plan calls for several kinds of memory. The MVP should formalize five layers:

1. `document_memory`: extracted source documents and normalized metadata
2. `structural_memory`: file tree, types, hashes, and locations
3. `trace_memory`: requirement-to-artifact relationships
4. `operational_memory`: command runs, refresh metadata, and last-seen fingerprints
5. `derived_memory`: summaries and coverage results derived from deterministic inputs

## JSON Contract Principles

Every command should support JSON output that is:

- versioned
- deterministic
- stable across runs with identical inputs
- explicit about partial coverage and unknowns

Common response fields should include:

- `schema_version`
- `command`
- `workspace_root`
- `generated_at`
- `status`
- `warnings`
- `data`

## Incrementality Strategy

P0 should use content fingerprints and persisted scan metadata to avoid full reprocessing when inputs are unchanged.

The first cut should support:

- scan all relevant files once
- persist fingerprints and metadata
- on next run, recompute file hashes only for candidate files
- refresh only changed documents and invalidate affected derived outputs

Continuous watch mode is useful, but it should follow once the manual incremental flow is correct.

## Traceability Strategy

The first traceability layer should rely on explicit document conventions and conservative heuristics:

- detect PRD-style files and requirement sections
- detect ADRs, specs, prompts, skills, and key implementation artifacts
- create trace edges from requirement identifiers and document references
- keep unknown or ambiguous links visible instead of inventing confidence

## Validation Strategy

Validation should initially focus on objective checks:

- requirement identifiers with no downstream artifact references
- artifacts linked to stale or missing sources
- contradictions in declared references
- missing indexed files expected by trace rules

## Key Trade-Offs

- Favor deterministic text and metadata extraction over advanced parsing.
- Favor local files and JSON over introducing a service boundary.
- Favor explicit heuristics over opaque ranking logic.
- Favor a narrow, useful MVP over a distributed memory platform.