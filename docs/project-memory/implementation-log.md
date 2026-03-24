# Project Memory CLI Implementation Log

## Purpose

This log records implementation decisions made while moving from backlog to code so the reasoning does not stay trapped in chat history.

## Decisions

### 2026-03-24 - Keep the CLI under `cli-tools/project-memory-cli/`

Reason:

- it matches the existing repository pattern for Rust CLIs
- it avoids mixing this project-side tooling with packaged runtime surfaces
- it makes repository validation ownership explicit

### 2026-03-24 - Use `.project-memory/snapshot.json` as the persisted store

Reason:

- a single deterministic JSON snapshot is enough for the MVP
- it keeps debugging simple during the first iteration
- it supports incremental reingest without introducing an external database

### 2026-03-24 - Make fingerprinted reingest the MVP incremental path

Reason:

- it solves the core repeated-reread problem immediately
- it is easier to validate than an always-on watcher
- it gives a stable base for later watch mode work

### 2026-03-24 - Keep traceability conservative

Reason:

- the repository needs defensible links, not guessed links
- explicit requirement identifiers and file-path references are auditable
- this keeps validation findings explainable to humans and agents

### 2026-03-24 - Implement `watch` as a bounded polling command

Reason:

- it gives cross-platform incremental refresh behavior without introducing a daemon
- it reuses the proven `ingest` path instead of duplicating invalidation logic
- it keeps failure modes explicit through `max-events` and `timeout-ms`

### 2026-03-24 - Limit structural enrichment to Rust metadata for now

Reason:

- it closes the remaining backlog item without introducing tree-sitter or a multi-language parser surface
- symbols and `use` imports are enough to prove that structural metadata improves retrieval
- unsupported languages still degrade cleanly because their indexed content remains queryable as plain text