# Workspace Capability Contract

The generated workspace stores detected tooling and explicit runtime authorization decisions in `.github/workspace-capabilities.yaml`.

This file is the authoritative capability snapshot for commands that consult it. Agents and runtime code must read it instead of inferring capabilities ad hoc. It governs runtime behavior, but it is not a standalone compliance boundary.

## Ownership

- `detected.*` fields are infrastructure-owned and refreshed by `prdtp-agents-functions-cli capabilities detect`.
- Tool detection is path-based. Infrastructure checks whether a command is discoverable on `PATH`; it does not require `command --version` to succeed.
- `authorized.*` fields are operating decisions and are the runtime gate for capability use.
- `policy.*` fields carry non-privilege settings such as `mode` or reporting visibility.
- Sensitive mutation capabilities such as `git` and `gh` must not auto-elevate from detection alone.
- Local operational capabilities such as reporting or local-history may start from detected defaults and be tightened later with `prdtp-agents-functions-cli capabilities authorize`.
- Commands that require capability gating fail closed if `.github/workspace-capabilities.yaml` or the relevant `authorized.enabled` entry is missing.
- Use `prdtp-agents-functions-cli capabilities authorize --capability <name> --enabled <true|false>` to change authorization intentionally.

## Capability outcomes

| Capability | `authorized.enabled=true` | `authorized.enabled=false` |
| --- | --- | --- |
| `git` | task branches and git-backed finalize flow may run | local-only mode; no task branches or git-backed finalize |
| `gh` | board sync, PR governance and release-gate validation, remote audit sink checks, and remote governance/readiness verification may run | GitHub-connected and remote runtime operations stay off |
| `sqlite` | database init/migrate plus audit sync/replay may write to `.state/project_memory.db` | `audit sync` degrades successfully to local mirror mode; database init/migrate/replay remain out of contract until SQLite is re-authorized |
| `markdownlint` | Markdown validation may run | lint is skipped by authorization with a warning |
| `reporting` | snapshot, dashboard, export, serve, and pack may run | reporting commands are out of contract |
| `local_history` | `.state/local-history/` may be used for local evidence | local-only evidence is disabled |

## SQLite default mode

`prdtp-agents-functions-cli capabilities detect` updates only the `detected.*`
fields for SQLite. It does not auto-authorize SQLite.

- `capabilities.sqlite.authorized.enabled` stays a deliberate operating decision.
- The default published seed keeps SQLite unauthorized until a maintainer
  explicitly enables it with `prdtp-agents-functions-cli capabilities authorize`.
- `audit sync` may still degrade safely to local-only mirror behavior while
  SQLite remains unauthorized or the DB is absent.

## Bootstrap and preservation behavior

Bootstrap refreshes `.github/workspace-capabilities.yaml` from the current
environment.

- On first bootstrap, sensitive capabilities stay unauthorized by default.
- On rerun, an existing non-placeholder capability file preserves explicit
  prior authorization decisions while refreshing `detected.*` fields.
- Git hooks are installed only when a Git repository exists and `git install-hooks`
  or the bootstrap Git path can write into `.git/hooks/`.

## Git-disabled mode

If `capabilities.git.authorized.enabled=false`:

- `prdtp-agents-functions-cli git checkout-task-branch` is out of contract.
- `prdtp-agents-functions-cli git finalize` falls back to local-only evidence.
- GitHub mutation flows and hidden maintainer-only wrappers are out of contract.

## GitHub governance contract

Repository governance is defined separately in `.github/github-governance.yaml`.

- `readiness.status` expresses whether the workspace is `template`, `bootstrapped`, `configured`, or `production-ready`.
- `operating_profile: core-local | enterprise` defines whether the workspace stays lightweight or opts into the optional remote-control overlay.
- `github.auth.mode: gh-cli | token-api` defines whether GitHub operations use local CLI flows or typed API calls. `token-api` is the supported remote path for `operating_profile=enterprise`.
- `audit.mode: local-hashchain | remote` defines whether sensitive audit evidence remains local-only or must be acknowledged by a remote sink.
- Placeholder reviewers or repository identifiers are acceptable only before the workspace reaches `configured`.
- `validate governance` is for configured workspaces.
- `validate readiness` is the stronger production-ready gate and must fail unless local governance is complete and remote GitHub controls are reachable and verified.
- `validate pr-governance` and `validate release-gate` enforce the configured `github.release_gate.approval_quorum` and `github.immutable_governance.approval_quorum` thresholds against current GitHub review state.
- Immutable governance edits are never authorized by local tokens alone; durable enforcement comes from remote PR approval verified through `github.immutable_governance.*` and its configured approval quorum.

## SQLite-disabled mode

If `capabilities.sqlite.authorized.enabled=false`:

- `prdtp-agents-functions-cli database init`, `database migrate`, and `audit replay-spool` are out of contract.
- `prdtp-agents-functions-cli audit sync` records degraded success locally and skips SQLite writes.
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
