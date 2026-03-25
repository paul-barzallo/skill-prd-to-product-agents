# project-memory-cli Reference

**Purpose**: project-side CLI for local repository memory, incremental indexing, traceability, validation, and impact analysis.

**Scope**: repository-side tooling only. This CLI is not part of the packaged skill release flow and it is not deployed into generated workspaces.

**Snapshot location**: `.project-memory/snapshot.json` under the indexed project root.

**SQLite mirror store**: `.project-memory/project-memory.db` under the indexed project root. This currently mirrors persisted ingest state, chunk rows, and deterministic local chunk embeddings while the CLI migrates toward stronger semantic retrieval.

**Chunking model**: ingest now persists deterministic chunks for indexed files. Markdown-like documents split into section-oriented chunks, while code and other text split into fixed line windows.

**Global flag**: `--project-root <path>` points to the repository or project tree being indexed. If omitted, the current directory is used.

---

## Commands

### `ingest`

Scan the project, classify text files, compute content fingerprints, persist the snapshot, and refresh derived trace data.

```bash
project-memory-cli --project-root <project-root> ingest
project-memory-cli --project-root <project-root> ingest --force
```

### `query`

Search the persisted snapshot without rescanning the repository.

Text queries rank deterministic chunks and include chunk provenance in each result: `chunk_id`, `chunk_kind`, `chunk_title`, `start_line`, and `end_line`. Symbol-only and import-only queries continue to return file-level matches when no text query is present.

```bash
project-memory-cli --project-root <project-root> query --text "REQ-001"
project-memory-cli --project-root <project-root> query --symbol calculate_total
project-memory-cli --project-root <project-root> query --import crate::pricing
project-memory-cli --project-root <project-root> query --file-type prd --limit 5
project-memory-cli --project-root <project-root> query --path-contains docs/
```

### `retrieve`

Return ranked chunk matches for hybrid local recall over the persisted snapshot. This command is chunk-first by design and is intended to become the stable retrieval contract for later semantic ranking.

```bash
project-memory-cli --project-root <project-root> retrieve --text "incident triage"
project-memory-cli --project-root <project-root> retrieve --text "checkout totals" --file-type prd --limit 5
```

### `watch`

Watch the project tree, refresh the snapshot after relevant changes, and exit once the requested number of events has been processed or the timeout is reached.

```bash
project-memory-cli --project-root <project-root> watch --interval-ms 250 --timeout-ms 10000 --max-events 1
project-memory-cli --project-root <project-root> watch --force-initial-ingest --max-events 2
```

### `trace`

Show trace edges for requirements or artifact paths.

```bash
project-memory-cli --project-root <project-root> trace
project-memory-cli --project-root <project-root> trace --requirement REQ-001
project-memory-cli --project-root <project-root> trace --path src/checkout.rs
```

### `impact`

Return reverse reachability for a requirement identifier or project-relative path.

```bash
project-memory-cli --project-root <project-root> impact --node REQ-001
project-memory-cli --project-root <project-root> impact --node src/checkout.rs
```

### `validate`

Emit structured findings for missing coverage and broken references based on the persisted snapshot.

```bash
project-memory-cli --project-root <project-root> validate
project-memory-cli --project-root <project-root> validate --fail-on-warnings
```

## Validation Behavior

- `validate` returns exit code `1` when errors are present.
- `validate --fail-on-warnings` also returns exit code `1` when only warnings are present.
- `query`, `trace`, `impact`, and `validate` all require an existing snapshot from `ingest`.

## Current MVP Limits

- requirement detection relies on explicit identifiers such as `REQ-001` or `PMEM-001`
- traceability relies on explicit requirement mentions and file-path references
- watch mode is a bounded single-command workflow backed by polling; it is not a background daemon or a multi-repository watcher
- fenced code examples and Rust string literals are excluded from trace extraction so repository docs and tests do not create false broken-reference findings
- structural enrichment currently supports Rust only, and it relies on conservative regex extraction for symbols and `use` imports
- `retrieve` is the preferred command when the caller wants chunk-level recall rather than the broader legacy `query` surface
- `retrieve` now emits both `lexical_score` and `semantic_score` for each result
- the current semantic component uses the deterministic local provider `local_hashed_v1`; it is useful for infrastructure and fallback ranking but should not be described as equivalent to an external embedding model