# Project Memory CLI Roadmap

## Priority Model

- `P0`: required for the first useful release
- `P1`: strong follow-up once the MVP contract is proven
- `P2`: enrichment and optimization work

## Planned Phases

## Phase 0: Planning And Contracts

- confirm architecture and repository fit
- define command surface and JSON response rules
- agree on persistence model and storage location

## Phase 1: MVP Foundation

- create the crate and core command structure
- implement repository ingestion and fingerprinted indexing
- implement query, trace, and validate commands over persisted data
- publish documentation and end-to-end tests

## Phase 2: Incremental Operations

- add watch mode on top of stable fingerprint invalidation
- improve refresh granularity and stale-derived-data handling
- add richer impact analysis over existing trace data

## Phase 3: Structural Enrichment

- add language-aware symbol and dependency extraction where justified
- improve ranking for retrieval and impact analysis
- expand validation rules where the signal is trustworthy

## Priority Breakdown

### P0

- PMEM-001
- PMEM-002
- PMEM-003
- PMEM-004
- PMEM-005
- PMEM-006
- PMEM-009

### P1

- PMEM-007

### P2

- PMEM-008

## Dependency Chain

1. PMEM-001 defines the integration contract
2. PMEM-002 establishes the CLI and JSON surface
3. PMEM-003 creates indexed state
4. PMEM-004, PMEM-005, and PMEM-006 consume the indexed state
5. PMEM-009 closes the MVP with tests, examples, and repo integration docs
6. PMEM-007 and PMEM-008 build on the stable MVP contracts