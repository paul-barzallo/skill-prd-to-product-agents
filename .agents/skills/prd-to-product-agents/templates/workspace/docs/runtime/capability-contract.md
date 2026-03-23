# Workspace Capability Contract

The generated workspace stores detected tooling and local policy decisions in `.github/workspace-capabilities.yaml`.

This file is the authoritative capability snapshot and policy contract for commands that consult it. Agents and runtime code should read it instead of inferring capabilities ad hoc.

## Ownership

- `detected.*` fields are infrastructure-owned and refreshed by `prdtp-agents-functions-cli capabilities detect`.
- `policy.*` fields are local operating decisions that may be edited intentionally by the user or `devops-release-engineer`.
- Agents must not silently flip policy values on their own.

## Capability outcomes

| Capability | `policy.enabled=true` | `policy.enabled=false` |
| --- | --- | --- |
| `git` | task branches and git-backed finalize flow may run | local-only mode; no task branches or git-backed finalize |
| `gh` | GitHub board sync and related automation may run | GitHub automation stays off |
| `sqlite` | database init/migrate plus audit sync/replay may write to `.state/project_memory.db` | SQLite-backed audit commands are out of contract; state commands keep using canonical YAML and degraded spools |
| `markdownlint` | Markdown validation may run | lint is skipped by policy with a warning |
| `reporting` | snapshot, dashboard, export, serve, and pack may run | reporting commands are out of contract |
| `local_history` | `.state/local-history/` may be used for local evidence | local-only evidence is disabled |

## Git-disabled mode

If `capabilities.git.policy.enabled=false`:

- `prdtp-agents-functions-cli git checkout-task-branch` is out of contract.
- `prdtp-agents-functions-cli git finalize` falls back to local-only evidence.
- Branch, PR, and GitHub automation flows are out of contract.

## GitHub governance contract

Repository governance is defined separately in `.github/github-governance.yaml`.

- `readiness.status` expresses whether the workspace is `template`, `bootstrapped`, `configured`, or `production-ready`.
- Placeholder reviewers or repository identifiers are acceptable only before the workspace reaches `configured`.
- `validate governance` is for configured workspaces.
- `validate readiness` is the stronger operational gate and must fail while local governance is still incomplete.

## SQLite-disabled mode

If `capabilities.sqlite.policy.enabled=false`:

- `prdtp-agents-functions-cli database init`, `database migrate`, `audit sync`, and `audit replay-spool` are out of contract.
- `state *` commands still mutate canonical YAML and infrastructure may spool degraded audit evidence locally.
- Re-enable later by setting `capabilities.sqlite.policy.enabled=true`, running `prdtp-agents-functions-cli database init`, and then `prdtp-agents-functions-cli audit replay-spool`.

## Reporting-disabled mode

If `capabilities.reporting.policy.enabled=false`:

- `prdtp-agents-functions-cli report snapshot`, `report dashboard`, `report export`, `report serve`, and `report pack` are out of contract.
- Canonical project state remains in `docs/project/*`; the reporting layer is derivative, not authoritative.

## Markdownlint-disabled mode

If `capabilities.markdownlint.policy.enabled=false`:

- Markdown validation is skipped by policy.
- This is a documented degraded mode, not an unexpected tooling failure.
