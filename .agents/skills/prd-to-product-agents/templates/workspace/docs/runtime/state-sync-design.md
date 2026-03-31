# State Sync Design

`audit sync` mirrors canonical file checksums into SQLite. It does not establish truth. Files under `docs/project/` remain authoritative.

## Checksum algorithm

- Hash function: SHA-256
- Scope: every `*.md`, `*.yaml`, and `*.yml` file under `docs/project/`
- Artifact rows are a derived mirror of canonical files, not an operational identifier scheme

## Concurrency model

- Canonical YAML writers (`prdtp-agents-functions-cli state *`) write through sidecar locks plus atomic replace.
- `prdtp-agents-functions-cli audit sync` acquires the same advisory locks before hashing operational YAML files.
- Markdown files are read without locking because they are not mutated by the state-ops lifecycle commands.
- SQLite remains a passive mirror and reporting store. A failed or delayed sync never changes canonical files.

## Degraded mode

`audit sync` exits successfully in degraded mode when any of these is true:

- SQLite authorization is missing
- SQLite authorization is disabled
- `.state/project_memory.db` does not exist
- the SQLite mirror is otherwise unavailable

In degraded mode it writes `.state/state-sync-degraded.log`, reports a degraded outcome, and skips SQLite writes. The canonical files remain untouched either way.

## Recovery

1. Authorize SQLite intentionally in `.github/workspace-capabilities.yaml` or via `capabilities authorize`.
2. If `capabilities detect` reports the SQLite CLI unavailable on a platform that expects it, install or expose `sqlite3` in `PATH`.
3. Run `prdtp-agents-functions-cli database init`.
4. Re-run `prdtp-agents-functions-cli audit sync`.

## Operational expectation

- Repeated sync runs with no file changes are valid no-op audits.
- Checksum mismatches after a completed sync indicate a later canonical edit or a failed later sync, not SQLite precedence over files.
