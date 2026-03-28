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

The embedding backend is now configurable through `.project-memory/config.toml`, environment variables, or explicit CLI flags. Resolution order is `flag > env > config file > safe default`.

That snapshot stores:

- indexed file content and metadata
- deterministic chunks with provenance metadata
- file hashes for incremental refresh
- derived trace edges

The SQLite mirror additionally stores one local embedding per chunk using the deterministic `local_hashed_v1` provider. This is retrieval infrastructure, not a claim of production-grade semantic embeddings.

When configured, ingest can instead call a loopback-only `local_microservice` provider and persist those vectors in the same SQLite store. `openai_compatible` is now available as an explicit opt-in remote backend for generic OpenAI-compatible APIs and Azure-compatible deployment endpoints.

## Embedding Configuration

The default behavior remains zero-cost local retrieval through `local_hashed_v1`.

To switch to an internal microservice, create `.project-memory/config.toml` under the indexed project root:

```toml
[embedding]
provider = "local_microservice"
endpoint = "http://127.0.0.1:6338/v1/embeddings"
timeout_ms = 3000
```

Equivalent overrides are also available through:

- CLI flags: `--config`, `--embedding-provider`, `--embedding-endpoint`, `--embedding-base-url`, `--embedding-deployment`, `--embedding-api-version`, `--embedding-model`, `--embedding-api-key-env`, `--embedding-remote-enabled`, `--embedding-timeout-ms`, `--embedding-max-requests-per-run`, `--embedding-fallback-provider`, `--embedding-fallback-endpoint`
- environment variables: `PMEM_CONFIG`, `PMEM_EMBEDDING_PROVIDER`, `PMEM_EMBEDDING_ENDPOINT`, `PMEM_EMBEDDING_BASE_URL`, `PMEM_EMBEDDING_DEPLOYMENT`, `PMEM_EMBEDDING_API_VERSION`, `PMEM_EMBEDDING_MODEL`, `PMEM_EMBEDDING_API_KEY_ENV`, `PMEM_EMBEDDING_REMOTE_ENABLED`, `PMEM_EMBEDDING_TIMEOUT_MS`, `PMEM_EMBEDDING_MAX_REQUESTS_PER_RUN`, `PMEM_EMBEDDING_FALLBACK_PROVIDER`, `PMEM_EMBEDDING_FALLBACK_ENDPOINT`

`local_microservice` is intentionally limited to loopback endpoints such as `127.0.0.1`, `localhost`, or `::1` so the default repository posture stays local and safe.

Remote providers are disabled by default. `openai_compatible` requires explicit remote enablement plus `api_key_env`; direct secrets inside `.project-memory/config.toml` are rejected.

Fallback is optional and local-only. Allowed fallback targets are `local_hashed_v1` and `local_microservice`; remote fallback is intentionally disallowed. `retrieve` now reports whether fallback was used, the effective backend, and whether cached embeddings were reused or recomputed.

Example OpenAI-compatible configuration:

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

Example Azure-compatible configuration:

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

## Commands

All commands accept `--project-root <path>`. If omitted, the current directory is used.

### `ingest`

Scan the repository, respect ignore rules, reuse unchanged indexed files from the previous snapshot, and refresh derived trace data. Hidden repository metadata such as `.github/workflows/` is indexed unless it is explicitly ignored.

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
- YAML files now use structured chunking around top-level keys plus workflow-style job and step boundaries so retrieval over `.github/workflows/` and similar config files has better semantic units.
- query text retrieval is now chunk-aware: results include `chunk_id`, chunk type, and line-span provenance for the ranked match.
- `retrieve` now combines exact lexical evidence with a deterministic local embedding score persisted in SQLite.
- the current embedding provider is `local_hashed_v1`, intended as infrastructure and fallback behavior rather than a production semantic model.
- local provider selection now supports `local_hashed_v1` and a loopback-only `local_microservice`, resolved through flags, env vars, or `.project-memory/config.toml`.
- `openai_compatible` now supports generic OpenAI-style base URLs and Azure-compatible deployment URLs through a single provider abstraction.
- remote providers require explicit opt-in and an API key environment variable; secrets embedded directly in `.project-memory/config.toml` are rejected.
- fallback behavior is now explicit and local-only; `retrieve` reports cache hits vs recomputation and whether the command degraded to a fallback provider.
