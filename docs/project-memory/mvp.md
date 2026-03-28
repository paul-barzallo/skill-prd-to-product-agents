# Project Memory CLI MVP

## Definition

The MVP is the smallest release that is genuinely useful for agents working from a PRD without forcing them to reread the whole repository on every step.

## MVP Must Include

1. a Rust crate under `cli-tools/project-memory-cli/`
2. base commands for `ingest`, `query`, `trace`, `validate`, and `impact`
3. repository scanning that respects ignore rules
4. file typing and metadata extraction for key documentation and source files
5. persisted fingerprints and incremental reingest on repeated runs
6. stable JSON output contracts
7. minimal PRD-to-artifact traceability
8. coverage and consistency validation
9. documentation and end-to-end tests

## MVP Deliberately Excludes

- full daemon-style watch service
- language-specific symbol graphs across multiple ecosystems
- external storage engines
- opaque AI-generated summaries with no deterministic source trail

## Why Watch Mode Is Not In P0

The plan correctly values incremental behavior, but a robust watch service depends on a correct invalidation model first. P0 will already support incremental refresh through persisted fingerprints. A continuous watch loop should arrive only after the base refresh semantics are proven.