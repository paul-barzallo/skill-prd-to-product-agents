
# prdtp-agents-functions-cli Reference

**Purpose**: Runtime workspace CLI - handles all daily operational tasks: state management, Git governance, validation, reporting, audit, agent assembly, database management, and board snapshot synchronization.

**Scope**: Deployed workspace operation only.

**Binary**: workspace-local binaries under `.agents/bin/prd-to-product-agents/`. CI may install the platform binary into `PATH`, but the workspace-local copy is the canonical runtime surface.

**Global flag**: `--workspace <path>` is required.

Every command in this reference must be invoked with an explicit workspace path,
for example `prdtp-agents-functions-cli --workspace . ...`.

---

## Commands

### validate

Validate workspace structure, agents, prompts, governance, models, and encoding.

```text
prdtp-agents-functions-cli --workspace . validate workspace
prdtp-agents-functions-cli --workspace . validate prompts
prdtp-agents-functions-cli --workspace . validate agents
prdtp-agents-functions-cli --workspace . validate governance
prdtp-agents-functions-cli --workspace . validate readiness
prdtp-agents-functions-cli --workspace . validate pr-governance --event-path "$GITHUB_EVENT_PATH"
prdtp-agents-functions-cli --workspace . validate release-gate --event-path "$GITHUB_EVENT_PATH"
prdtp-agents-functions-cli --workspace . validate models
prdtp-agents-functions-cli --workspace . validate encoding
prdtp-agents-functions-cli --workspace . validate ci reporting
prdtp-agents-functions-cli --workspace . validate ci copilot-runtime-contract
```

| Subcommand | Purpose |
| ---------- | ------- |
| `workspace` | Validate full workspace structure and YAML files |
| `prompts` | Validate prompts have required sections |
| `agents` | Validate agent hierarchy and contracts |
| `governance` | Validate a configured workspace has real repository identifiers, reviewers, CODEOWNERS, and no placeholders |
| `readiness` | Validate the strong `production-ready` gate: structure, governance, assembly, encoding, capability contract, and remote GitHub controls |
| `pr-governance` | Validate PR metadata, labels, required sections, commit subjects, and release gate preconditions from a GitHub event payload |
| `release-gate` | Validate only the final release-gate approval path for PRs targeting `main` |
| `models` | Validate model frontmatter against `agent-model-policy.yaml` |
| `encoding` | Validate file encoding (BOM, CRLF, mojibake) |

#### validate ci

CI-focused validation helpers used by workflow automation and release gates.

```text
prdtp-agents-functions-cli --workspace . validate ci pre-commit-fixtures
prdtp-agents-functions-cli --workspace . validate ci yaml-tabs
prdtp-agents-functions-cli --workspace . validate ci yaml-schemas
prdtp-agents-functions-cli --workspace . validate ci raw-sql-prompts
prdtp-agents-functions-cli --workspace . validate ci template-state
prdtp-agents-functions-cli --workspace . validate ci prompt-tool-contracts
prdtp-agents-functions-cli --workspace . validate ci prompt-label-contracts
prdtp-agents-functions-cli --workspace . validate ci operational-state
prdtp-agents-functions-cli --workspace . validate ci degraded-runtime
prdtp-agents-functions-cli --workspace . validate ci reporting
prdtp-agents-functions-cli --workspace . validate ci copilot-runtime-contract
```

| Subcommand | Purpose |
| ---------- | ------- |
| `pre-commit-fixtures` | Verify malformed YAML and immutable-governance fixtures are rejected. |
| `yaml-tabs` | Reject tab characters in `docs/project/*.yaml`. |
| `yaml-schemas` | Parse schema-covered YAML objects under `docs/project/`. |
| `raw-sql-prompts` | Reject raw SQL snippets in prompt Markdown. |
| `template-state` | Ensure runtime-generated state artifacts are not committed into the template. |
| `prompt-tool-contracts` | Ensure prompts and assembled agents declare coherent tool contracts. |
| `prompt-label-contracts` | Ensure prompts only reference labels defined in `github-governance.yaml`. |
| `operational-state` | Run lifecycle and negative checks for handoffs, findings, and releases. |
| `degraded-runtime` | Verify degraded runtime behavior when SQLite is deferred or unavailable. |
| `reporting` | Verify reporting snapshot and dashboard generation. |
| `copilot-runtime-contract` | Reject stale GitHub.com parity claims, obsolete readiness terms, and runtime-doc contract drift. |

### state

Manage operational state (handoffs, findings, releases, events).

#### state handoff

```text
prdtp-agents-functions-cli --workspace . state handoff create \
  --from-role pm-orchestrator --to-role tech-lead \
  --handoff-type normal --entity US-001 --reason new_work --id ho-001

prdtp-agents-functions-cli --workspace . state handoff update \
  --handoff-id ho-001 --new-status claimed --agent-role tech-lead
```

#### state finding

```text
prdtp-agents-functions-cli --workspace . state finding create \
  --source-role qa-lead --target-role tech-lead \
  --finding-type bug --severity high --entity US-001 \
  --title "Description" --id fi-001

prdtp-agents-functions-cli --workspace . state finding update \
  --finding-id fi-001 --new-status triaged --agent-role tech-lead
```

#### state release

```text
prdtp-agents-functions-cli --workspace . state release create \
  --name "Release 1.0" --target-date 2025-06-01 \
  --agent-role devops-release-engineer --stories "US-001" --id R1

prdtp-agents-functions-cli --workspace . state release update \
  --release-ref R1 --new-status ready --agent-role devops-release-engineer
```

#### state event

```text
prdtp-agents-functions-cli --workspace . state event record \
  --env-name prod --event-type incident-detected \
  --reported-by devops-release-engineer --severity high --notes "Description"
```

### git

Git operations (task branches, finalize, pre-commit validation, hooks).

```text
prdtp-agents-functions-cli --workspace . git checkout-task-branch --role backend-developer --issue-id PROJ-42 --slug fix-auth
prdtp-agents-functions-cli --workspace . git finalize --agent-role backend-developer --summary "description"
prdtp-agents-functions-cli --workspace . git pre-commit-validate --staged-file path/to/file
prdtp-agents-functions-cli --workspace . git install-hooks
```

| Subcommand | Purpose |
| ---------- | ------- |
| `checkout-task-branch` | Create or switch to a task branch with naming validation; refuses dirty worktrees and does not rebase or fast-forward implicitly |
| `finalize` | Pre-commit validation + atomic commit; blocks commit creation if workspace validation fails |
| `pre-commit-validate` | Governance, branch protection, immutable file validation |
| `install-hooks` | Install git hooks into `.git/hooks/` |

> **Security note:** The env vars `BOOTSTRAP_ALLOW_MAIN_COMMIT` and `FINALIZE_WORK_UNIT_ALLOW_COMMIT` are internal bypass flags used exclusively by `bootstrap commit` and `git finalize` respectively. They must never be exported manually or set in production CI pipelines.

### audit

Audit ledger operations.

```text
prdtp-agents-functions-cli --workspace . audit sync
prdtp-agents-functions-cli --workspace . audit replay-spool
prdtp-agents-functions-cli --workspace . audit export
```

| Subcommand | Purpose |
| ---------- | ------- |
| `sync` | Sync canonical docs into the SQLite audit ledger |
| `replay-spool` | Replay JSON spool entries into the ledger |
| `export` | Export structured audit evidence as JSONL from canonical state, spool, and work-unit records |

### report

Reporting operations.

```text
prdtp-agents-functions-cli --workspace . report snapshot
prdtp-agents-functions-cli --workspace . report dashboard
prdtp-agents-functions-cli --workspace . report export --format csv
prdtp-agents-functions-cli --workspace . report serve
prdtp-agents-functions-cli --workspace . report pack
```

| Subcommand | Purpose |
| ---------- | ------- |
| `snapshot` | Build `report-snapshot.json` from canonical docs |
| `dashboard` | Refresh `management-dashboard.md` from snapshot |
| `export` | Export reports (CSV, XLSX) |
| `serve` | Start local HTTP server for reporting dashboard |
| `pack` | Run snapshot + dashboard + export (CSV & XLSX) in one step |

### capabilities

Capability detection and checks.

```text
prdtp-agents-functions-cli --workspace . capabilities detect
prdtp-agents-functions-cli --workspace . capabilities authorize --capability git --enabled true --source devops-maintainer --mode full
prdtp-agents-functions-cli --workspace . capabilities check
```

### agents

Agent assembly.

```text
prdtp-agents-functions-cli --workspace . agents assemble
prdtp-agents-functions-cli --workspace . agents assemble --verify
```

Assembles `.agent.md` files from `identity/` + `context/` sources. `--verify`
compares expected output without rewriting files.

### database

SQLite database initialization and migration.

```text
prdtp-agents-functions-cli --workspace . database init
prdtp-agents-functions-cli --workspace . database init --force
prdtp-agents-functions-cli --workspace . database migrate
```

| Subcommand | Purpose |
| ---------- | ------- |
| `init` | Initialize or verify the SQLite audit ledger |
| `migrate` | Apply incremental schema migrations |

Notes:

- If SQLite initialization is deferred, `database init` writes
  `.state/sqlite-bootstrap.pending.md` instead of faking a ready database.
- Successful initialization also writes `.state/sqlite-bootstrap.report.md`.

### governance

Governance operations.

```text
prdtp-agents-functions-cli --workspace . governance immutable-token --reason "Fix typo in copilot-instructions.md" --files .github/copilot-instructions.md
prdtp-agents-functions-cli --workspace . governance configure --owner acme-org --repo product-workspace --release-gate-login acme-devops --reviewer-product @acme-product --reviewer-architecture @acme-arch --reviewer-tech-lead @acme-techlead --reviewer-qa @acme-qa --reviewer-devops @acme-devops --reviewer-infra @acme-infra
```

| Subcommand | Purpose |
| ---------- | ------- |
| `immutable-token` | Generate a time-limited local bypass token for intentional maintenance of immutable governance files. |
| `configure` | Fill local GitHub repository identifiers, reviewer handles, release-gate login, regenerate `CODEOWNERS`, and move readiness to `configured`. `production-ready` is a separately reviewed state checked by `validate readiness` and `validate release-gate`. |

### dependencies

Dependency detection.

```text
prdtp-agents-functions-cli --workspace . dependencies check
```

### board

GitHub issues/PR snapshot synchronization.

```text
prdtp-agents-functions-cli --workspace . board sync
```

Syncs GitHub issues and pull requests to `docs/project/board.md`. It is an execution snapshot, not a GitHub Project field synchronizer.

### github issue

GitHub Issue mutation wrappers.

```text
prdtp-agents-functions-cli --workspace . github issue create --title "Story: checkout flow" --body "..." --label role:backend
prdtp-agents-functions-cli --workspace . github issue update --issue-ref GH-42 --title "Updated title"
prdtp-agents-functions-cli --workspace . github issue comment --issue-ref GH-42 --body "Blocked by API contract"
prdtp-agents-functions-cli --workspace . github issue label --issue-ref GH-42 --add status:blocked --remove status:ready
```

These commands are gated by `capabilities.gh.authorized.enabled=true`.
