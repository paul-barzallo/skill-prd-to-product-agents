# PMEM-001 - Confirm architecture and repository integration for project-memory-cli

## Type
Spike

## Priority
P0

## Objective
Lock the v1 architecture, CLI placement, storage location, and command surface before implementation starts.

## Context
The initial plan is ambitious and spans several capabilities. This repository already contains multiple Rust CLIs with distinct scopes, so the first step is to define how `project-memory-cli` fits the existing structure without overlapping other tools.

## Scope
- confirm `cli-tools/project-memory-cli/` as the home for the new crate
- document subcommands, output modes, and storage conventions
- document MVP scope and explicitly deferred capabilities

## Out Of Scope
- implementation of ingestion or query logic
- full ADR process outside the initiative scope

## Dependencies
- PMEM-000

## Technical Tasks
- document v1 architecture under `docs/project-memory/architecture.md`
- define command surface for `ingest`, `query`, `trace`, `validate`, and `impact`
- define the on-disk store location and JSON schema envelope
- record the P0, P1, and P2 boundary

## Acceptance Criteria
- architecture doc exists and is repository-aligned
- command and JSON contracts are defined at a high level
- deferred scope is explicit and defensible

## Risks
- unresolved ownership boundaries could cause overlap with existing CLIs

## Definition Of Done
- the future crate location is decided
- architecture and MVP boundaries are documented

## Expected Artifacts
- `docs/project-memory/architecture.md`
- `docs/project-memory/mvp.md`