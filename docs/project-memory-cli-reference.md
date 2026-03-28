# project-memory-cli Reference

**Purpose**: project-side CLI for local repository memory, incremental indexing, traceability, validation, and impact analysis.

**Scope**: repository-side tooling only. This CLI is not part of the packaged skill release flow and it is not deployed into generated workspaces.

**Snapshot location**: `.project-memory/snapshot.json` under the indexed project root.

**SQLite mirror store**: `.project-memory/project-memory.db` under the indexed project root. This currently mirrors persisted ingest state, chunk rows, and deterministic local chunk embeddings while the CLI migrates toward stronger semantic retrieval.

**Chunking model**: ingest now persists deterministic chunks for indexed files. Markdown-like documents split into section-oriented chunks, while code and other text split into fixed line windows.

YAML files use structured chunks based on mapping boundaries, and GitHub Actions-style workflows additionally split around job and step anchors when those can be inferred from indentation and keys.

**Global flags**: `--project-root <path>` points to the repository or project tree being indexed. If omitted, the current directory is used. `--config`, `--embedding-provider`, `--embedding-endpoint`, `--embedding-base-url`, `--embedding-deployment`, `--embedding-api-version`, `--embedding-model`, `--embedding-api-key-env`, `--embedding-remote-enabled`, `--embedding-timeout-ms`, `--embedding-max-requests-per-run`, `--embedding-fallback-provider`, and `--embedding-fallback-endpoint` override embedding-provider configuration.

**Embedding config path**: `.project-memory/config.toml` by default. Resolution order is `flag > env > config file > local_hashed_v1`.

**Supported providers in this revision**:

- `local_hashed_v1`: deterministic zero-cost fallback and default.
- `local_microservice`: loopback-only HTTP provider intended for an internal embedding service.
- `openai_compatible`: explicit opt-in remote provider for generic OpenAI-compatible APIs and Azure-compatible deployment endpoints.

Fallback policy in this revision:

- fallback is optional
- fallback must stay local (`local_hashed_v1` or `local_microservice`)
- fallback to `openai_compatible` is intentionally rejected
- `retrieve` reports whether fallback was used and whether persisted embeddings were reused or recomputed

---

## Commands

### `ingest`

Scan the project, classify text files, compute content fingerprints, persist the snapshot, and refresh derived trace data. Hidden repository metadata such as `.github/workflows/` is included unless ignore rules exclude it.

```bash
project-memory-cli --project-root <project-root> ingest
project-memory-cli --project-root <project-root> ingest --force
project-memory-cli --project-root <project-root> --embedding-provider local_microservice --embedding-endpoint http://127.0.0.1:6338/v1/embeddings ingest
project-memory-cli --project-root <project-root> --embedding-provider openai_compatible --embedding-base-url https://example.openai-compatible.invalid/v1 --embedding-model text-embedding-3-small --embedding-api-key-env PMEM_OPENAI_API_KEY --embedding-remote-enabled true ingest
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
project-memory-cli --project-root <project-root> --config .project-memory/config.toml retrieve --text "incident triage"
```

### Embedding configuration example

```toml
[embedding]
provider = "local_microservice"
endpoint = "http://127.0.0.1:6338/v1/embeddings"
timeout_ms = 3000
```

OpenAI-compatible example:

```toml
[embedding]
provider = "openai_compatible"
base_url = "https://example.openai-compatible.invalid/v1"
model = "text-embedding-3-small"
api_key_env = "PMEM_OPENAI_API_KEY"
remote_enabled = true
max_requests_per_run = 4
fallback_provider = "local_hashed_v1"
```

Azure-compatible example:

```toml
[embedding]
provider = "openai_compatible"
base_url = "https://example-resource.openai.azure.com"
deployment = "team-embed"
api_version = "2024-06-01-preview"
model = "text-embedding-3-large"
api_key_env = "PMEM_AZURE_OPENAI_API_KEY"
remote_enabled = true
max_requests_per_run = 4
fallback_provider = "local_hashed_v1"
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
- `retrieve` now also exposes `configured_embedding_provider`, `embedding_model`, `remote_access`, `cost_risk`, `cache_status`, `fallback_used`, and `fallback_reason`
- the current semantic component can use `local_hashed_v1`, `local_microservice`, or the explicit opt-in `openai_compatible` bridge depending on configuration
- `local_microservice` is constrained to loopback endpoints so repository defaults stay no-cost and local-first
- `openai_compatible` requires explicit remote enablement and rejects direct secrets in `.project-memory/config.toml`