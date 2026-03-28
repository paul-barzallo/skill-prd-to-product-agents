# Runtime Error Recovery

Error recovery procedures for the workspace runtime CLI (`prdtp-agents-functions-cli`).

## Context system confusion or stale state

If an agent sees conflicting information across docs, assembled agent files,
reporting output, or the SQLite ledger:

1. Treat `docs/project/*` as authoritative.
2. If assembled agent files are stale, refresh the correct context source and run `prdtp-agents-functions-cli agents assemble`.
3. If reporting is stale, run `prdtp-agents-functions-cli report snapshot` and then `prdtp-agents-functions-cli report dashboard` when reporting is enabled.
4. If the ledger is stale, leave canonical files unchanged and let infrastructure run `prdtp-agents-functions-cli audit sync`.
5. Do not edit `.state/project_memory.db` directly and do not use it to override canonical files.

## Audit sync failure

If `prdtp-agents-functions-cli audit sync` fails:

1. Check whether `capabilities.sqlite.policy.enabled=true` in `.github/workspace-capabilities.yaml`.
2. Check whether `.state/project_memory.db` exists.
3. If SQLite policy is disabled, `audit sync` is out of contract by design.
4. To recover: install `sqlite3`, enable SQLite policy intentionally, run `prdtp-agents-functions-cli database init`, then re-run `prdtp-agents-functions-cli audit sync`.

## Database initialization failure

If `prdtp-agents-functions-cli database init` fails:

- Check whether `capabilities.sqlite.policy.enabled=true` in `.github/workspace-capabilities.yaml`.
- Verify `sqlite3` is reachable.
- Check the schema file at `.state/memory-schema.sql`.
- If the schema cannot be applied, report which table or statement failed.
- Do not leave the DB in a fake ready state.

## Governance configuration failure

If `prdtp-agents-functions-cli governance configure` fails:

- Check that all required flags were provided.
- Check `.github/github-governance.yaml` and `.github/CODEOWNERS` still exist.
- Re-run `prdtp-agents-functions-cli validate governance` after fixing inputs.
- Do not set `readiness.status=configured` manually; the command owns that transition.

## Git finalize failure

If `prdtp-agents-functions-cli git finalize` fails:

- It runs runtime validation inline before commit creation. Read the terminal error output first.
- If workspace validation fails, run `prdtp-agents-functions-cli validate workspace`.
- If governance checks fail, run `prdtp-agents-functions-cli validate governance` or `validate readiness` depending on workspace state.
- If validation passes but the commit fails, inspect Git state with `git status`.
- In local-only mode, evidence is written to `.state/local-history/` without Git.

## Pre-commit validation failure

If `prdtp-agents-functions-cli git pre-commit-validate` rejects a commit:

- Check immutable-file edits listed in `.github/immutable-files.txt`.
- Check whether the local bypass token covers exactly the staged governance files.
- Check for invalid operational YAML in staged files.
- For intentional governance maintenance, use `prdtp-agents-functions-cli governance immutable-token --reason "..." --files <path...>` to create a local bypass token for that exact edit set.
- Treat the token as a local operational guardrail, not as an external approval artifact or strong authorization control.

## State operations failure

If `prdtp-agents-functions-cli state *` commands fail when SQLite is unavailable:

- State operations spool locally when `capabilities.sqlite.policy.enabled=false`.
- Re-enable by installing SQLite, setting `capabilities.sqlite.policy.enabled=true`, running `prdtp-agents-functions-cli database init`, then `prdtp-agents-functions-cli audit replay-spool`.

## Report generation failure

If `prdtp-agents-functions-cli report dashboard` fails:

- Check whether `capabilities.reporting.policy.enabled=true` in `.github/workspace-capabilities.yaml`.
- Check that `.state/reporting/report-snapshot.json` exists; if not, run `report snapshot`.
- The dashboard is generated from canonical YAML plus the current snapshot.
- The reporting UI is local, optional, and read-only.
