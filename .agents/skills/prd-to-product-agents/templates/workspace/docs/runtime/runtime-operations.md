
# Runtime Operations Reference

Operational reference for `prdtp-agents-functions-cli` commands used in
day-to-day workspace operation after bootstrap.

All commands require an explicit workspace root:
`prdtp-agents-functions-cli --workspace <path> ...`

The primary validated path is `core-local`. `enterprise` commands are an
optional remote overlay and only become meaningful after explicit external
infrastructure is configured.

Before adding or repeating runtime claims, check `runtime-claims-coverage.md`.

## Context system first

Before using reporting, audit, or database commands, read
`context-system-runtime.md`.

- Canonical truth lives in `docs/project/*`.
- Assembled `.agent.md` files and reporting outputs are derived views.
- `.state/project_memory.db` is a passive infrastructure ledger, not an agent-facing query surface.
- If a context surface looks stale, refresh the canonical files first and rebuild derivatives second.

## Runtime CLI commands

| Command | Role |
| -------- | ------ |
| `prdtp-agents-functions-cli agents assemble` | Rebuild assembled `.agent.md` files from identity/context sources. |
| `prdtp-agents-functions-cli database init` | Initialize or verify the passive SQLite audit ledger. |
| `prdtp-agents-functions-cli database migrate` | Apply incremental schema migrations. |
| `prdtp-agents-functions-cli validate encoding` | Packaging and source hygiene gate for BOM, mojibake and LF policy drift. |
| `prdtp-agents-functions-cli validate workspace` | Full workspace structure and YAML validation. |
| `prdtp-agents-functions-cli validate agents` | Validate agent hierarchy and contracts. |
| `prdtp-agents-functions-cli validate prompts` | Validate prompts have required sections. |
| `prdtp-agents-functions-cli validate governance` | Validate a configured workspace has real repository identifiers, reviewers, CODEOWNERS, and no placeholders. |
| `prdtp-agents-functions-cli validate readiness` | Validate the optional `enterprise` overlay for `production-ready`: structure, governance, assembly, encoding, capability-contract prerequisites, and remote GitHub controls. |
| `prdtp-agents-functions-cli validate pr-governance` | Validate PR metadata, required sections, label contract, commit subjects, and release gate preconditions from the GitHub event payload. |
| `prdtp-agents-functions-cli validate release-gate` | Validate only the final release-gate approval path for PRs targeting `main`. |
| `prdtp-agents-functions-cli validate models` | Validate model frontmatter against agent-model-policy.yaml. |
| `prdtp-agents-functions-cli validate ci ...` | Workspace-portable CI-focused validation helpers for fixtures, schemas, degraded runtime, reporting, and Copilot contract drift. |
| `prdtp-agents-functions-cli capabilities detect` | Detect tool availability and render `workspace-capabilities.yaml`. |
| `prdtp-agents-functions-cli capabilities authorize` | Explicitly authorize or de-authorize a capability without editing YAML by hand. |
| `prdtp-agents-functions-cli dependencies check` | Check workspace dependency availability. |
| `prdtp-agents-functions-cli git finalize` | Supported end-of-task closure path for Git-enabled and local-only workspaces; blocks commit creation if workspace validation fails. |
| `prdtp-agents-functions-cli git checkout-task-branch` | Task branch creation with naming validation; refuses dirty worktrees and does not sync branches implicitly. |
| `prdtp-agents-functions-cli git pre-commit-validate` | Governance gate for immutable files, staged YAML sanity. |
| `prdtp-agents-functions-cli git install-hooks` | Install git hooks into `.git/hooks/`. |
| `prdtp-agents-functions-cli report snapshot` | Build `.state/reporting/report-snapshot.json` from canonical docs and execution evidence. |
| `prdtp-agents-functions-cli report dashboard` | Refresh the executive Markdown dashboard from the reporting snapshot. |
| `prdtp-agents-functions-cli report serve` | Open the local reporting UI against the generated snapshot. |
| `prdtp-agents-functions-cli report pack` | Run snapshot + dashboard + export (CSV & XLSX) in one step. |
| `prdtp-agents-functions-cli report export` | Export CSV, XLSX report packs. |
| `prdtp-agents-functions-cli audit sync` | Passive ledger sync from canonical docs into SQLite. |
| `prdtp-agents-functions-cli audit replay-spool` | Replay degraded-mode spool entries into the ledger. |
| `prdtp-agents-functions-cli audit sink health` | Verify local audit hash-chain integrity and remote sink configuration. |
| `prdtp-agents-functions-cli audit sink test` | Emit a probe event through the configured audit sink and local mirror. |
| `prdtp-agents-functions-cli state handoff create/update` | Handoff YAML operations. |
| `prdtp-agents-functions-cli state finding create/update` | Finding YAML operations. |
| `prdtp-agents-functions-cli state release create/update` | Release YAML operations. |
| `prdtp-agents-functions-cli state event record` | Environment event recording. |
| `prdtp-agents-functions-cli governance configure` | Configure local repository owner/name, reviewers, release-gate logins and quorums, and regenerate `CODEOWNERS`. |
| `prdtp-agents-functions-cli governance provision-enterprise` | Provision optional remote branch protection and governance labels for the enterprise profile. |
| `prdtp-agents-functions-cli board sync` | Refresh the operational board snapshot from GitHub issues and pull requests. |

## CI validation helpers

Use `validate ci` for workflow-only checks that go beyond the core workspace
contract:

These helpers are workspace-portable checks against the current tree. They are workflow-oriented rather than day-to-day operator commands, but they do not require publisher-only checkout state.

- `pre-commit-fixtures`: verifies malformed YAML and immutable-governance
  fixtures are handled correctly by the local finalize gate.
- `yaml-tabs`: rejects tab characters in `docs/project/*.yaml`.
- `yaml-schemas`: parses schema-covered YAML objects.
- `raw-sql-prompts`: blocks raw SQL snippets in prompts.
- `template-state`: ensures runtime-generated `.state` artifacts are not
  committed into the template.
- `prompt-tool-contracts` and `prompt-label-contracts`: enforce prompt and
  assembled-agent frontmatter governance.
- `operational-state`: exercises handoff, finding, and release lifecycles.
- `degraded-runtime`: verifies deferred-SQLite behavior.
- `reporting`: verifies snapshot and dashboard generation.
- `copilot-runtime-contract`: rejects stale GitHub.com parity claims, obsolete readiness states, and runtime-doc drift.

## Work-unit closure

A unit of work is not complete until `prdtp-agents-functions-cli git finalize` succeeds.

- If Git authorization is enabled, `git finalize` runs workspace validation as a blocking pre-commit gate and creates the commit only after validation and governance checks pass.
- Immutable governance files may be staged through the controlled finalize path, but manual `git commit` stays blocked and merge authority still comes only from remote PR approval through `validate pr-governance`.
- If Git authorization is disabled, `git finalize` writes Markdown + JSON evidence under `.state/local-history/`.

## Reporting operations

- `report snapshot` generates `.state/reporting/report-snapshot.json`.
- `report dashboard` renders `docs/project/management-dashboard.md` from the snapshot.
- `report pack` runs snapshot + dashboard + export (CSV & XLSX) in one step.
- `report serve` starts the local read-only reporting UI.
- CSV export is mandatory. XLSX may degrade explicitly when runtime prerequisites are missing.

## Audit operations

- `audit sync` mirrors canonical file checksums into SQLite when SQLite is authorized and available. If SQLite is unauthorized, missing, or not initialized yet, it exits successfully in degraded mode and records that outcome locally.
- `audit replay-spool` recovers events from degraded-mode spool files when SQLite authorization is enabled again.
- `audit sink health` verifies the local hash-chain and, in remote mode, checks that the remote sink configuration is present.
- `audit sink test` emits a probe event through the configured audit path. In the optional `enterprise` overlay, the remote sink must acknowledge the event or the command fails.
- The audit ledger is passive. Failed or delayed syncs never change canonical files.

## State and degradation notes

- `.state/bootstrap-manifest.txt` and `.state/bootstrap-report.md` are runtime
  artifacts written by bootstrap and validation flows; they are not template
  source files.
- `.state/sqlite-bootstrap.pending.md` indicates deferred SQLite
  initialization, not a fake success state.
- `.state/audit/sensitive-actions.jsonl` is the local hash-chained mirror for sensitive runtime actions.
- In local-only mode, `git finalize` writes evidence into
  `.state/local-history/` instead of creating commits.
