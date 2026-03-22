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

`audit sync` exits successfully in degraded mode when either of these is true:

- `sqlite3` is unavailable
- `.state/project_memory.db` does not exist

In degraded mode it writes `.state/state-sync-degraded.log`, reports `Result: degraded`, and skips SQLite writes.

## Recovery

1. Install or expose `sqlite3` in `PATH`.
2. Run `prdtp-agents-functions-cli database init`.
3. Re-run `prdtp-agents-functions-cli audit sync`.

## Operational expectation

- Repeated sync runs with no file changes are valid no-op audits.
- Checksum mismatches after a completed sync indicate a later canonical edit or a failed later sync, not SQLite precedence over files.
