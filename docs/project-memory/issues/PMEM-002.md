# PMEM-002 - Bootstrap the crate and stable JSON CLI surface

## Type
Feature

## Priority
P0

## Objective
Create a compilable Rust CLI with the base command structure and stable JSON response envelopes for agent consumption.

## Context
Without a clean CLI contract, the rest of the backlog will either drift or hardcode assumptions into later modules.

## Scope
- create `cli-tools/project-memory-cli/`
- add command parsing for `ingest`, `query`, `trace`, `validate`, and `impact`
- implement common JSON response envelopes and error handling

## Out Of Scope
- fully functional indexing or traceability behavior

## Dependencies
- PMEM-001

## Technical Tasks
- initialize the crate with `clap`, `serde`, and `serde_json`
- implement shared response structs with schema versioning
- add command help text and exit behavior
- add smoke tests for command parsing and JSON format

## Acceptance Criteria
- the crate compiles
- each base subcommand is discoverable through CLI help
- JSON responses share a common schema envelope

## Risks
- unstable schema shape will cascade into every downstream command

## Definition Of Done
- crate exists and builds
- JSON envelope contract is covered by tests

## Expected Artifacts
- `cli-tools/project-memory-cli/Cargo.toml`
- `cli-tools/project-memory-cli/src/main.rs`
- tests for the command surface