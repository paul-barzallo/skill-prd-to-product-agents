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

That snapshot stores:

- indexed file content and metadata
- file hashes for incremental refresh
- derived trace edges

## Commands

All commands accept `--project-root <path>`. If omitted, the current directory is used.

### `ingest`

Scan the repository, respect ignore rules, reuse unchanged indexed files from the previous snapshot, and refresh derived trace data.

```bash
cargo run --manifest-path cli-tools/project-memory-cli/Cargo.toml -- --project-root . ingest
```

### `query`

Search indexed content without rescanning the repository.

```bash
cargo run --manifest-path cli-tools/project-memory-cli/Cargo.toml -- --project-root . query --text "REQ-001"
cargo run --manifest-path cli-tools/project-memory-cli/Cargo.toml -- --project-root . query --file-type prd --limit 5
cargo run --manifest-path cli-tools/project-memory-cli/Cargo.toml -- --project-root . query --symbol calculate_total
cargo run --manifest-path cli-tools/project-memory-cli/Cargo.toml -- --project-root . query --import crate::pricing
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
