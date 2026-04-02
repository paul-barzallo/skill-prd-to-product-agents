
# AGENTS

## PRD-to-Product Workspace Agents

This workspace uses 9 custom agents for product-development automation. Each agent has an immutable personality, defined tool access, and declared memory contracts.

When bootstrap merges this file into an existing AGENTS.md, it drops only the root `# AGENTS` heading so this section integrates as one more `##` block in the host document.

### Start Here

- Read this file first.
- Review `.state/bootstrap-report.md` after bootstrap to confirm readiness, degraded-mode notes, and next actions.
- Treat a freshly generated workspace as `bootstrapped`, not operationally ready. `Readiness: not_ready` in `.state/bootstrap-report.md` means local configuration is still pending.
- If a PRD already exists and is clear, start with `bootstrap-from-prd` using `pm-orchestrator`.
- `bootstrap-from-prd` applies the PRD clarity gate and must ask follow-up questions before planning or implementation continues when the PRD is incomplete or ambiguous.
- A task is not complete until `prdtp-agents-functions-cli --workspace . git finalize` succeeds.

### Delegation Hierarchy

The workspace uses a strict 3-level hierarchy designed to keep each agent's context window small and focused.

```text
L0 - Strategic Orchestration
`-- pm-orchestrator
    |-- delegates to any L1 agent (sub-agent tool)
    |-- NEVER bypasses L1 to reach L2 directly
    `-- expects a structured report-back from every delegation

L1 - Domain Authority
|-- product-owner         lateral -> ux-designer
|-- software-architect    no laterals (reports to PM)
|-- ux-designer           lateral -> product-owner
|-- tech-lead             delegates to L2 (sub-agent tool)
|-- qa-lead               lateral findings -> product-owner, tech-lead
`-- devops-release-eng.   lateral findings -> tech-lead; escalation -> PM

L2 - Implementation (only tech-lead delegates here)
|-- backend-developer     reports to tech-lead only
`-- frontend-developer    reports to tech-lead only
```

#### Delegation rules (sub-agent tool)

| Rule | Description |
| ---- | ----------- |
| L0 -> L1 only | PM delegates to any L1 agent. Never launches L2 agents directly. |
| L1 -> L2 only via TL | Only tech-lead may delegate to backend-developer and frontend-developer. |
| No skip-level | PM never bypasses tech-lead to assign work to developers. |
| No upward delegation | L1 never launches L0. L2 never launches L1. |

#### Lateral communication (handoffs, not delegation)

| From | To | Allowed for |
| ---- | -- | ----------- |
| qa-lead | product-owner | Functional / scope / UX findings |
| qa-lead | tech-lead | Technical / implementation / security findings |
| devops-release-engineer | tech-lead | Environment / deployment technical issues |
| product-owner | ux-designer | Product-UX synergy requests |
| ux-designer | product-owner | UX scope clarification requests |

All other cross-domain requests go through PM.

#### Report-back protocol

Every sub-agent delegation must end with a structured report returned to the delegator:

```markdown
## Report Back
- **Task**: What was assigned
- **Status**: completed | blocked | partial
- **Summary**: 2-5 sentence outcome
- **Artifacts changed**: List of files modified or created
- **Findings**: Issues discovered (if any)
- **Next recommendation**: What the coordinator should do next
```

#### Context window discipline

- **Delegator**: Pass only the task description, relevant file paths, and acceptance criteria. Never dump full project context into a delegation prompt.
- **Sub-agent**: Read only the files listed in the delegation prompt. Do not scan the entire project tree.
- **Report-back**: Return a summary. Do not replay the full execution trace.
- **Accumulation**: Each L1 report-back adds ~200 words to the coordinator's context, not thousands.

#### Input / output contracts per level

| Level | Inputs | Outputs |
| ----- | ------ | ------- |
| L0 - PM | PRD, user requests, L1 report-backs, canonical state (handoffs.yaml, findings.yaml, releases.yaml) | Delegation prompts to L1, coordination artifacts, management dashboard |
| L1 - PO | Delegation from PM + relevant PRD/product context | Report-back, vision.md, scope.md, backlog.yaml, refined-stories.yaml, acceptance-criteria.md |
| L1 - Arch | Delegation from PM + scope, backlog, technical constraints | Report-back, architecture/overview.md, ADRs |
| L1 - UX | Delegation from PM (or PO lateral) + scope, user journeys, AC | Report-back, UX artifacts under docs/project/ux/ |
| L1 - TL | Delegation from PM + scope, architecture, stories | Report-back to PM, delegation prompts to L2, implementation maps, qa_notes |
| L1 - QA | Delegation from PM + acceptance criteria, implementation state | Report-back, findings (routed laterally to PO/TL by type) |
| L1 - DevOps | Delegation from PM + release state, quality gates, governance | Report-back, release status transitions, environment events |
| L2 - BE | Delegation from TL + implementation_map, story, AC, arch docs | Report-back to TL, code changes, validation-pack results |
| L2 - FE | Delegation from TL + implementation_map, story, AC, UX artifacts | Report-back to TL, code changes, validation-pack results |

### Coordinators (L0 + L1-bridge)

| Level | Agent | Role | Delegates to | Tools |
| ----- | ------- | ------ | ------------- | ------- |
| L0 | [pm-orchestrator](.github/agents/pm-orchestrator.agent.md) | Strategic orchestration | L1 agents only | search, read, edit/editFiles, execute (runtime CLI only), agent |
| L1 | [tech-lead](.github/agents/tech-lead.agent.md) | Technical authority, L1-L2 bridge | backend-developer, frontend-developer | search, read, edit/editFiles, execute (runtime CLI + scoped dev tooling), agent |

### Domain Agents (L1)

| Level | Agent | Role | Primary docs | Tools |
| ----- | ------- | ------ | ------------- | ------- |
| L1 | [product-owner](.github/agents/product-owner.agent.md) | Business requirements and scope | vision.md, scope.md, backlog.yaml | search, read, edit/editFiles, execute (runtime CLI only) |
| L1 | [software-architect](.github/agents/software-architect.agent.md) | Technical design and ADRs | docs/project/architecture/, docs/project/decisions/ | search, read, edit/editFiles, execute (runtime CLI only) |
| L1 | [ux-designer](.github/agents/ux-designer.agent.md) | User experience design | wireframes, UX patterns | search, read, edit/editFiles, execute (runtime CLI only) |

### Implementation Agents (L2)

| Level | Agent | Role | Delegated by | Tools |
| ----- | ------- | ------ | ------------ | ------- |
| L2 | [backend-developer](.github/agents/backend-developer.agent.md) | Server-side implementation | tech-lead only | search, read, edit/editFiles, execute (runtime CLI + scoped dev tooling) |
| L2 | [frontend-developer](.github/agents/frontend-developer.agent.md) | Client-side implementation | tech-lead only | search, read, edit/editFiles, execute (runtime CLI + scoped dev tooling) |

### Operations Agents (L1)

| Level | Agent | Role | Tools |
| ----- | ------- | ------ | ------- |
| L1 | [qa-lead](.github/agents/qa-lead.agent.md) | Quality assurance and testing | search, read, edit/editFiles, execute (runtime CLI + scoped dev tooling) |
| L1 | [devops-release-engineer](.github/agents/devops-release-engineer.agent.md) | Deployment and monitoring | search, read, edit/editFiles, execute (runtime CLI + scoped dev tooling) |

### Handoff Rules

- `pm-orchestrator` (L0) is the entry point and delegates to L1 agents only.
- `pm-orchestrator` never delegates directly to L2 agents (`backend-developer`, `frontend-developer`).
- `tech-lead` (L1) is the only agent that delegates to L2 implementation agents.
- L2 agents report back exclusively to `tech-lead`. They do not hand off to other L1 peers.
- `software-architect` does **not** command developers directly - routes through `tech-lead` via PM.
- `qa-lead` routes functional, scope, and UX findings laterally to `product-owner`.
- `qa-lead` routes technical, implementation, and security findings laterally to `tech-lead`.
- `product-owner` may request UX work laterally to `ux-designer` (product-UX synergy).
- All other cross-domain requests must go through `pm-orchestrator`.
- Every sub-agent delegation ends with a structured report-back to the delegator.
- Security is a **workflow**, not an agent.

### Memory Model

- **Canonical truth**: `docs/project/*` (Markdown/YAML files).
- **Context system**: read `docs/runtime/context-system-runtime.md` for the files-first retrieval order, derivative surfaces, and recovery rules.
- **Execution layer**: GitHub Issues, branches, commits and PRs. `docs/project/board.md` is a derived issues/PR snapshot, not a source of truth.
- **Historical context**: Git (commits, PRs, issues, tags) for traceability.
- **Operational capability contract**: `.github/workspace-capabilities.yaml` is the persisted detection + authorization snapshot consulted by Git, GitHub automation, SQLite audit, reporting, and markdownlint commands. It governs runtime behavior but is not, by itself, a compliance-grade security boundary.
- **Operating profiles**: `.github/github-governance.yaml` declares `operating_profile: core-local | enterprise`. `core-local` is the primary validated path. `enterprise` is an optional remote overlay that requires remote GitHub controls, non-interactive API auth, and a remote audit sink before `production-ready` is valid.
- **GitHub governance contract**: `.github/github-governance.yaml` defines readiness, reviewers, labels, reserved future project metadata, and release-gate expectations.
- A passive audit ledger exists at `.state/project_memory.db` but is managed automatically by infrastructure. Agents do not interact with it.
- Governance immutability is driven only by `.github/immutable-files.txt`. Seeded project docs under `docs/project/` remain editable after bootstrap by their owning roles.
- Operational YAML transitions are driven through `prdtp-agents-functions-cli --workspace . state` subcommands. Direct edits to `handoffs.yaml`, `findings.yaml`, or `releases.yaml` are out of contract even when the file is canonical state.
- If Git capability is disabled, the workspace runs in local-only mode and change evidence is written to `.state/local-history/` instead of commits or PRs.
- `prdtp-agents-functions-cli --workspace . git finalize` is the supported closure path for completed work in both Git and local-only modes.
- Agents must not run `git commit` directly for task work. Use `prdtp-agents-functions-cli --workspace . git finalize` so staged files, commit metadata, validation, and work-unit evidence are enforced together.
- In Git-enabled workspaces, `prdtp-agents-functions-cli --workspace . git finalize` executes the shared pre-commit validator and governance checks before commit creation. If validation fails, no commit is created.
- When `prdtp-agents-functions-cli --workspace . git install-hooks` has been run in a Git-enabled workspace, the installed `pre-commit` hook blocks normal direct `git commit` attempts and tells the caller to use `prdtp-agents-functions-cli --workspace . git finalize` instead.
- If SQLite capability is disabled, audit keeps a local hash-chained mirror under `.state/audit/`, may spool degraded operational evidence under `.state/audit-spool/` and `.state/degraded-ops/`, and `audit sync` records degraded success instead of failing closed.
- `docs/project/management-dashboard.md` is the executive summary view generated from canonical docs plus the execution layer.
- `.state/reporting/report-snapshot.json` is the shared reporting source for Markdown, UI, and exports.
- `prdtp-agents-functions-cli --workspace . report serve` is the local read-only reporting dashboard for PM and TL.
- `prdtp-agents-functions-cli --workspace . report pack` is the supported path for CSV and XLSX reporting exports.

### Git and Task Governance

| Role family | Branch prefix | Typical output |
| ------- | ------- | ------- |
| product-owner | `product/` | scope and backlog updates, issue refinement |
| software-architect | `arch/` | architecture docs, ADRs, technical task framing |
| ux-designer | `ux/` | UX journeys and interaction artifacts |
| tech-lead | `techlead/` | issue breakdown, implementation maps, technical coordination |
| backend-developer | `backend/` | backend code and tests |
| frontend-developer | `frontend/` | frontend code and tests |
| qa-lead | `qa/` | validation findings, quality gates, QA docs |
| devops-release-engineer | `ops/` | release, CI, deployment and environment changes |
| pm-orchestrator | `product/` | coordination snapshots, planning and flow updates |

- In Git-enabled workspaces, daily work starts from a GitHub Issue and uses `develop` as the base branch.
- Task branches follow `<role>/<issue-id>-slug`.
- PRs must use `.github/PULL_REQUEST_TEMPLATE.md` and include one `role:*`, one `kind:*`, and one `priority:*` label.
- Authors must review PR comments and commit comments before asking for merge.
- `devops-release-engineer` is the final approval gate before merge.
- If Git capability is disabled, task-branch creation and PR flow are out of contract; work closes through `prdtp-agents-functions-cli --workspace . git finalize`, which records local-only evidence under `.state/local-history/`.

### Model routing

Model selection is part of the agent contract in VS Code / IDE environments. The canonical policy lives in `.github/agent-model-policy.yaml`.

| Agent | Default model order |
| ------- | ------------------- |
| `backend-developer` | `Claude Opus 4.5` -> `GPT-4.1` -> `Claude Haiku 4.5` |
| `frontend-developer` | `Claude Opus 4.5` -> `Gemini 2.5 Pro` -> `GPT-4.1` -> `Claude Haiku 4.5` |
| `software-architect` | `GPT-4.1` -> `Claude Opus 4.5` -> `Claude Haiku 4.5` |
| `tech-lead` | `GPT-4.1` -> `Claude Opus 4.5` -> `Claude Haiku 4.5` |
| `devops-release-engineer` | `GPT-4.1` -> `Claude Haiku 4.5` -> `Claude Opus 4.5` |
| `qa-lead` | `GPT-4.1` -> `Claude Haiku 4.5` |
| `pm-orchestrator` | `Claude Opus 4.5` -> `GPT-4.1` -> `Claude Haiku 4.5` |
| `product-owner` | `Claude Haiku 4.5` -> `GPT-4.1` -> `Gemini 2.5 Pro` |
| `ux-designer` | `Claude Haiku 4.5` -> `Gemini 2.5 Pro` -> `GPT-4.1` |

- Prompt overrides in `.github/prompts/` are the supported path for small-task routing and deep-analysis escalation.
- Only official GA model names listed in `.github/agent-model-policy.yaml` are allowed.
- GitHub.com ignores `model:` frontmatter and runs in degraded mode for model selection.

### Execute Scope

`execute` is a controlled platform permission. Role frontmatter may include it when runtime CLI closure, governance, or technical validation require it. Documentation-centered roles stay on runtime CLI and coordination wrappers only; engineering roles may additionally use scoped build/test/lint tooling. Prompt frontmatter must omit `execute` for bounded workflows that do not need command execution. The canonical runtime binary lives under `.agents/bin/prd-to-product-agents/`; CI may install `prdtp-agents-functions-cli` into `PATH`, but the workspace-local copy is the contract.

> **Full command reference**: [docs/runtime/prdtp-agents-functions-cli-reference.md](docs/runtime/prdtp-agents-functions-cli-reference.md)

#### prdtp-agents-functions-cli subcommands

| Subcommand | Purpose |
| ---------- | ------- |
| `prdtp-agents-functions-cli --workspace . git finalize` | Pre-commit validation + atomic commit |
| `prdtp-agents-functions-cli --workspace . git checkout-task-branch` | Task branch creation with naming validation |
| `prdtp-agents-functions-cli --workspace . state handoff create/update` | Handoff YAML operations |
| `prdtp-agents-functions-cli --workspace . state finding create/update` | Finding YAML operations |
| `prdtp-agents-functions-cli --workspace . state release create/update` | Release YAML operations |
| `prdtp-agents-functions-cli --workspace . state event record` | Environment event recording |
| `prdtp-agents-functions-cli --workspace . report dashboard` | Render management-dashboard.md from snapshot |
| `prdtp-agents-functions-cli --workspace . report snapshot` | Generate report-snapshot.json |
| `prdtp-agents-functions-cli --workspace . audit sync` | SQLite ledger synchronization |
| `prdtp-agents-functions-cli --workspace . audit replay-spool` | Replay degraded-mode spool into SQLite |
| `prdtp-agents-functions-cli --workspace . audit sink health/test` | Verify or probe the configured audit sink |
| `prdtp-agents-functions-cli --workspace . capabilities detect` | Tool detection -> workspace-capabilities.yaml |
| `prdtp-agents-functions-cli --workspace . validate workspace/prompts/agents/models` | Structural validation |
| `prdtp-agents-functions-cli --workspace . validate governance` | Governance validation for configured workspaces |
| `prdtp-agents-functions-cli --workspace . validate readiness` | Strong operational readiness validation for production-ready workspaces |

#### Per-agent intended call set

This table is an intended call set, not a hard runtime permission boundary.
Frontmatter and tool support may narrow behavior, but durable containment still
depends on workspace governance, CODEOWNERS, PR workflows, and review.
No technical role broker is in scope for this P0.

| Agent | Intended `execute` call set |
| ----- | ---------------------------- |
| `pm-orchestrator` | `prdtp-agents-functions-cli --workspace . git finalize`, `prdtp-agents-functions-cli --workspace . git checkout-task-branch`, `prdtp-agents-functions-cli --workspace . report dashboard`, `prdtp-agents-functions-cli --workspace . report snapshot`, `prdtp-agents-functions-cli --workspace . state handoff create/update`, `prdtp-agents-functions-cli --workspace . state finding update`, `prdtp-agents-functions-cli --workspace . agents assemble` |
| `tech-lead` | `prdtp-agents-functions-cli --workspace . git finalize`, `prdtp-agents-functions-cli --workspace . git checkout-task-branch`, `prdtp-agents-functions-cli --workspace . state handoff create/update`, `prdtp-agents-functions-cli --workspace . state finding create/update`, `prdtp-agents-functions-cli --workspace . agents assemble` |
| `product-owner` | `prdtp-agents-functions-cli --workspace . git finalize`, `prdtp-agents-functions-cli --workspace . git checkout-task-branch`, `prdtp-agents-functions-cli --workspace . state handoff create`, `prdtp-agents-functions-cli --workspace . state finding update` |
| `software-architect` | `prdtp-agents-functions-cli --workspace . git finalize`, `prdtp-agents-functions-cli --workspace . git checkout-task-branch`, `prdtp-agents-functions-cli --workspace . state finding create/update`, `prdtp-agents-functions-cli --workspace . state handoff create` |
| `ux-designer` | `prdtp-agents-functions-cli --workspace . git finalize`, `prdtp-agents-functions-cli --workspace . git checkout-task-branch`, `prdtp-agents-functions-cli --workspace . state handoff create` |
| `backend-developer` | `prdtp-agents-functions-cli --workspace . git finalize`, `prdtp-agents-functions-cli --workspace . git checkout-task-branch`, `prdtp-agents-functions-cli --workspace . state handoff create` |
| `frontend-developer` | `prdtp-agents-functions-cli --workspace . git finalize`, `prdtp-agents-functions-cli --workspace . git checkout-task-branch`, `prdtp-agents-functions-cli --workspace . state handoff create` |
| `qa-lead` | `prdtp-agents-functions-cli --workspace . git finalize`, `prdtp-agents-functions-cli --workspace . git checkout-task-branch`, `prdtp-agents-functions-cli --workspace . state finding create/update`, `prdtp-agents-functions-cli --workspace . state handoff create` |
| `devops-release-engineer` | `prdtp-agents-functions-cli --workspace . git finalize`, `prdtp-agents-functions-cli --workspace . git checkout-task-branch`, `prdtp-agents-functions-cli --workspace . state release create/update`, `prdtp-agents-functions-cli --workspace . state event record`, `prdtp-agents-functions-cli --workspace . state finding create`, `prdtp-agents-functions-cli --workspace . state handoff create`, `prdtp-agents-functions-cli --workspace . state finding update` |
