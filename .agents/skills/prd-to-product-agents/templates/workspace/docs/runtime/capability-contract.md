# Workspace Capability Contract

The generated workspace stores detected tooling and explicit runtime authorization decisions in `.github/workspace-capabilities.yaml`.

This file is the authoritative capability snapshot for commands that consult it. Agents and runtime code must read it instead of inferring capabilities ad hoc.

## Ownership

- `detected.*` fields are infrastructure-owned and refreshed by `prdtp-agents-functions-cli capabilities detect`.
- `authorized.*` fields are operating decisions and are the hard gate for capability use.
- `policy.*` fields carry non-privilege settings such as `mode` or reporting visibility.
- Sensitive capabilities must not auto-elevate from detection alone.
- Commands that require capability gating fail closed if `.github/workspace-capabilities.yaml` or the relevant `authorized.enabled` entry is missing.
- Use `prdtp-agents-functions-cli capabilities authorize --capability <name> --enabled <true|false>` to change authorization intentionally.

## Capability outcomes

| Capability | `authorized.enabled=true` | `authorized.enabled=false` |
| --- | --- | --- |
| `git` | task branches and git-backed finalize flow may run | local-only mode; no task branches or git-backed finalize |
| `gh` | GitHub issue wrappers, board sync, PR governance validation, and remote governance checks may run | GitHub mutation and production-ready readiness checks stay off |
| `sqlite` | database init/migrate plus audit sync/replay may write to `.state/project_memory.db` | SQLite-backed audit commands are out of contract; state commands keep using canonical YAML and degraded spools |
| `markdownlint` | Markdown validation may run | lint is skipped by authorization with a warning |
| `reporting` | snapshot, dashboard, export, serve, and pack may run | reporting commands are out of contract |
| `local_history` | `.state/local-history/` may be used for local evidence | local-only evidence is disabled |

## Git-disabled mode

If `capabilities.git.authorized.enabled=false`:

- `prdtp-agents-functions-cli git checkout-task-branch` is out of contract.
- `prdtp-agents-functions-cli git finalize` falls back to local-only evidence.
- Branch, PR, and GitHub mutation flows are out of contract.

## GitHub governance contract

Repository governance is defined separately in `.github/github-governance.yaml`.

- `readiness.status` expresses whether the workspace is `template`, `bootstrapped`, `configured`, or `production-ready`.
- Placeholder reviewers or repository identifiers are acceptable only before the workspace reaches `configured`.
- `validate governance` is for configured workspaces.
- `validate readiness` is the stronger production-ready gate and must fail unless local governance is complete and remote GitHub controls are reachable and verified.
- `validate pr-governance` and `validate release-gate` are the supported contract checks for pull requests and final promotion to `main`.

## SQLite-disabled mode

If `capabilities.sqlite.authorized.enabled=false`:

- `prdtp-agents-functions-cli database init`, `database migrate`, `audit sync`, and `audit replay-spool` are out of contract.
- `state *` commands still mutate canonical YAML and infrastructure may spool degraded audit evidence locally.
- Re-enable later by setting `capabilities.sqlite.authorized.enabled=true`, running `prdtp-agents-functions-cli database init`, and then `prdtp-agents-functions-cli audit replay-spool`.

## Reporting-disabled mode

If `capabilities.reporting.authorized.enabled=false`:

- `prdtp-agents-functions-cli report snapshot`, `report dashboard`, `report export`, `report serve`, and `report pack` are out of contract.
- Canonical project state remains in `docs/project/*`; the reporting layer is derivative, not authoritative.

## Markdownlint-disabled mode

If `capabilities.markdownlint.authorized.enabled=false`:

- Markdown validation is skipped by authorization.
- This is a documented degraded mode, not an unexpected tooling failure.
