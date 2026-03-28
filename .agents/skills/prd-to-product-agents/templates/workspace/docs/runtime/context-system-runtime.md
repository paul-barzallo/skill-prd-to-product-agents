# Workspace Context System

This workspace uses a files-first context retrieval system for agents and local
runtime tooling.

The system is intentionally not database-first. Agents read canonical Markdown
and YAML, and infrastructure may derive extra views from those files for audit,
assembly, and reporting.

## Canonical truth

The authoritative project state lives under `docs/project/*`.

- Product and execution intent live in Markdown and YAML.
- Operational transitions happen through `prdtp-agents-functions-cli state *` commands.
- Direct edits to `handoffs.yaml`, `findings.yaml`, and `releases.yaml` are out of contract even though those files are canonical.

If a derived surface disagrees with `docs/project/*`, the files win.

## Retrieval layers

The workspace exposes context through these layers:

1. Canonical docs under `docs/project/*`.
2. Assembled agent files under `.github/agents/*.agent.md`.
3. Reporting snapshot and dashboard under `.state/reporting/` and `docs/project/management-dashboard.md`.
4. Passive SQLite audit ledger at `.state/project_memory.db` when SQLite is enabled.

Read the layers in that order. Lower layers are increasingly derivative.

## What agents may use

Agents may:

- read canonical docs under `docs/project/*`
- read assembled `.agent.md` files after assembly
- use `prdtp-agents-functions-cli state *` for allowed operational transitions
- use `prdtp-agents-functions-cli report snapshot` and `report dashboard` when reporting is enabled and the role allows those commands
- use `prdtp-agents-functions-cli agents assemble` after updating agent context sources

Agents must not:

- query, edit, or treat `.state/project_memory.db` as authoritative
- patch derived reporting files to work around stale canonical state
- bypass `state *` commands with direct YAML edits for operational transitions
- treat a stale dashboard, stale assembled agent file, or stale ledger as proof that canonical state changed

## What the SQLite ledger is for

`.state/project_memory.db` is a passive audit and reporting mirror.

- It stores derivative evidence for infrastructure and reporting workflows.
- `audit sync` mirrors canonical file checksums into SQLite.
- A failed sync never changes canonical truth.
- When SQLite is disabled, the workspace may spool degraded audit evidence under `.state/audit-spool/`.

Agents should reason from files, not from the ledger.

## Supported runtime commands

These commands matter most for context retrieval and freshness:

- `prdtp-agents-functions-cli state handoff create/update`
- `prdtp-agents-functions-cli state finding create/update`
- `prdtp-agents-functions-cli state release create/update`
- `prdtp-agents-functions-cli agents assemble`
- `prdtp-agents-functions-cli report snapshot`
- `prdtp-agents-functions-cli report dashboard`
- `prdtp-agents-functions-cli audit sync`
- `prdtp-agents-functions-cli audit replay-spool`

Only use a command when both the capability policy and the acting role allow it.

## Failure handling order

When context looks wrong, recover in this order:

1. Re-read the relevant canonical files under `docs/project/*`.
2. If agent context is stale, refresh the appropriate context source and run `prdtp-agents-functions-cli agents assemble`.
3. If reporting output is stale, run `prdtp-agents-functions-cli report snapshot` and `report dashboard` when reporting is enabled.
4. If SQLite is enabled and infrastructure needs the ledger updated, run `prdtp-agents-functions-cli audit sync`.
5. If SQLite was previously disabled, re-enable it intentionally, run `database init`, then `audit replay-spool`.

Do not reverse this order.

## Degraded modes

The context system continues to function in degraded mode because files remain
canonical even when supporting layers are unavailable.

- If Git is disabled, local evidence moves to `.state/local-history/`.
- If SQLite is disabled, audit falls back to spool-only behavior.
- If reporting is disabled, dashboards and exports are out of contract.
- If markdownlint is disabled, lint checks are skipped by policy.

The canonical docs remain usable in every degraded mode.
