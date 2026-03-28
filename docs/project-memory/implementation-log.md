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

### 2026-03-28 - Keep provider defaults local-first and remote use explicitly opt-in

Reason:

- repository maintenance tooling must stay safe and no-cost by default
- provider selection needs one deterministic precedence chain across flags, env vars, and `.project-memory/config.toml`
- remote backends are useful, but only when network use, secrets, and request budgets are intentional and observable

### 2026-03-28 - Index hidden workflow metadata explicitly instead of opening all dotfiles

Reason:

- `.github/workflows/` is high-value repository context for maintenance and release work
- globally un-hiding dotfiles broadened ingest too far and broke the intended repository boundary
- explicit inclusion keeps workflow retrieval available without turning every hidden path into indexed surface area

### 2026-03-28 - Chunk YAML and workflow files by structural anchors

Reason:

- fixed-size windows were too weak for retrieval over CI and release automation
- jobs, steps, and top-level keys provide better retrieval units without introducing a full YAML parser dependency
- repository automation queries are more explainable when chunk titles match workflow structure

### 2026-03-28 - Persist recomputed retrieve embeddings so cache invalidation converges

Reason:

- a cache mismatch that only recomputes in memory makes every later `retrieve` pay the same cost again
- once provider or model drift is detected, the refreshed embedding set should become the new persisted baseline
- regression coverage must verify that a second identical `retrieve` becomes `cache_status = hit`

### 2026-03-28 - Keep repo, skill, and workspace docs as separate contracts

Reason:

- maintainers, skill users, and deployed agents each need different instructions and failure models
- scope leakage causes incorrect operational assumptions after bootstrap
- the workspace now documents a files-first context system and should not depend on repository-maintenance guidance