# Project Memory CLI Epics

## Epic A: Foundation And Contracts

Goal: establish a repo-aligned architecture, CLI surface, storage layout, and JSON contracts that other capabilities can build on safely.

Issues:

- PMEM-001
- PMEM-002

## Epic B: Indexed Project Memory

Goal: ingest repositories, persist fingerprints and metadata, and support incremental reprocessing without rereading the entire workspace.

Issues:

- PMEM-003
- PMEM-007

## Epic C: Retrieval, Traceability, And Validation

Goal: provide useful agent-facing retrieval plus deterministic traceability and validation outputs over project artifacts.

Issues:

- PMEM-004
- PMEM-005
- PMEM-006

## Epic D: Hardening And Repository Integration

Goal: make the MVP verifiable, documented, and maintainable inside this repository's validation discipline.

Issues:

- PMEM-009

## Epic E: Enrichment Beyond MVP

Goal: add structural analysis depth only after the basic memory system proves its value.

Issues:

- PMEM-008

## Epic F: Provider Architecture And Safety

Goal: introduce provider-aware embedding backends, local-first configuration, and explicit remote safety boundaries without undoing the repository-only scope.

Issues:

- PMEM-010
- PMEM-011
- PMEM-012
- PMEM-013
- PMEM-014
- PMEM-015
- PMEM-016

## Epic G: Provider Reliability And Operator Diagnostics

Goal: make provider-backed retrieval reproducible, diagnosable, and testable across local and optional remote modes.

Issues:

- PMEM-017
- PMEM-018
- PMEM-019