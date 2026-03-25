# project-memory-cli

`project-memory-cli` is a repository-side Rust CLI for deterministic local memory over a project tree.

It is designed to reduce repeated full-repository rereads for agent workflows by persisting a local snapshot with:

- indexed text files and metadata
- stable content fingerprints
- basic traceability from requirements to artifacts
- local query, impact, and validation commands

## Scope

This CLI is not a packaged skill binary and it is not part of generated workspace runtime deployment. It lives under `cli-tools/` as project-side tooling.

## Storage Model

The CLI persists its local snapshot under `.project-memory/snapshot.json` inside the indexed project root.

It also maintains a SQLite mirror store at `.project-memory/project-memory.db` as the foundation for chunked and semantic retrieval. The JSON snapshot remains the compatibility contract for the current command set during the migration.

That snapshot stores:

- indexed file content and metadata
- deterministic chunks with provenance metadata
- file hashes for incremental refresh
- derived trace edges

The SQLite mirror additionally stores one local embedding per chunk using the deterministic `local_hashed_v1` provider. This is retrieval infrastructure, not a claim of production-grade semantic embeddings.

## Commands

All commands accept `--project-root <path>`. If omitted, the current directory is used.

### `ingest`

Scan the repository, respect ignore rules, reuse unchanged indexed files from the previous snapshot, and refresh derived trace data.

```bash
cargo run --manifest-path cli-tools/project-memory-cli/Cargo.toml -- --project-root . ingest
```

### `query`

Search indexed content without rescanning the repository. Text queries now rank deterministic chunks and return chunk provenance, while symbol/import-only queries keep the file-level fallback.

```bash
cargo run --manifest-path cli-tools/project-memory-cli/Cargo.toml -- --project-root . query --text "REQ-001"
cargo run --manifest-path cli-tools/project-memory-cli/Cargo.toml -- --project-root . query --file-type prd --limit 5
cargo run --manifest-path cli-tools/project-memory-cli/Cargo.toml -- --project-root . query --symbol calculate_total
cargo run --manifest-path cli-tools/project-memory-cli/Cargo.toml -- --project-root . query --import crate::pricing
```

### `retrieve`

Retrieve ranked chunk matches for hybrid local recall. This is the chunk-first retrieval contract that future external semantic ranking will build on.

```bash
cargo run --manifest-path cli-tools/project-memory-cli/Cargo.toml -- --project-root . retrieve --text "incident triage" --limit 5
```

### `watch`

Watch the project tree, wait for relevant file changes, and refresh the snapshot incrementally when they occur.

```bash
cargo run --manifest-path cli-tools/project-memory-cli/Cargo.toml -- --project-root . watch --timeout-ms 10000 --max-events 1
```

### `trace`

Inspect requirement and artifact edges from the persisted snapshot.

```bash
cargo run --manifest-path cli-tools/project-memory-cli/Cargo.toml -- --project-root . trace --requirement REQ-001
```

### `impact`

Return reverse reachability for a requirement or file node.

```bash
cargo run --manifest-path cli-tools/project-memory-cli/Cargo.toml -- --project-root . impact --node src/main.rs
```

### `validate`

Emit machine-readable findings for missing coverage and broken references.

```bash
cargo run --manifest-path cli-tools/project-memory-cli/Cargo.toml -- --project-root . validate
```

## Design Choices

- The MVP uses fingerprint-based refresh, not a long-running watcher.
- Traceability is conservative and deterministic; it relies on explicit requirement identifiers such as `REQ-001` or `PMEM-001` plus file-path references.
- JSON is the primary output contract for agent consumption.
- `watch` is a single-invocation polling workflow, not a daemon. It exits after `max-events` or `timeout-ms`.
- Fenced code samples and Rust string literals are intentionally ignored for trace extraction so examples do not become false validation failures.
- structural enrichment is currently limited to Rust symbols and `use` imports extracted with conservative regex-based parsing.
- chunking is deterministic today: markdown-like files split by sections and large sections, while code and other text split into fixed line windows.
- query text retrieval is now chunk-aware: results include `chunk_id`, chunk type, and line-span provenance for the ranked match.
- `retrieve` now combines exact lexical evidence with a deterministic local embedding score persisted in SQLite.
- the current embedding provider is `local_hashed_v1`, intended as infrastructure and fallback behavior rather than a production semantic model.
