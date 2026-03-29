---
applyTo: ".github/agents/**"
---


# Agent File Rules

Global rules (routing, tool restrictions, personality immutability, memory model) are in `copilot-instructions.md` and always active. This file covers **agent-file-specific** conventions only.

## Delegation hierarchy

The workspace enforces a strict 3-level hierarchy:

- **L0 - Strategic Orchestration**: `pm-orchestrator`. Delegates to L1 only.
- **L1 - Domain Authority**: `product-owner`, `software-architect`, `ux-designer`, `tech-lead`, `qa-lead`, `devops-release-engineer`. Report back to L0 or to the delegating coordinator.
- **L2 - Implementation**: `backend-developer`, `frontend-developer`. Delegated only by `tech-lead`. Report back to `tech-lead` exclusively.

**Enforcement rules:**

- Only agents with the `agent` tool (`pm-orchestrator`, `tech-lead`) may launch sub-agents.
- `pm-orchestrator` must not list L2 agents in its `agents:` frontmatter.
- `tech-lead` must not list L1 peers in its `agents:` frontmatter.
- L2 agents must not have handoffs to L1 peers (e.g., no `backend-developer` -> `qa-lead`).
- Every agent must have a report-back handoff or instruction to return results to its delegator.

**Lateral communication** (handoffs, not sub-agent delegation):

- `qa-lead` -> `product-owner` (functional findings) and `qa-lead` -> `tech-lead` (technical findings) are allowed.
- `devops-release-engineer` -> `tech-lead` (environment issues) is allowed.
- `product-owner` <-> `ux-designer` (product-UX synergy) is allowed.
- All other cross-domain requests go through `pm-orchestrator`.

**Context window discipline:**

- When delegating, pass only the task description, relevant file paths, and acceptance criteria. Never dump full project context.
- Sub-agents read only the files listed in the delegation prompt. They do not scan the entire project.
- Report-backs are summaries (2-5 sentences + artifact list). Never replay full execution traces.

## YAML frontmatter

- Agent files use YAML frontmatter: `description`, `tools`, `model`, and optionally `agents` (coordinators only).
- `model` must be an ordered fallback list for VS Code / IDE execution.
- Allowed model names and the exact per-agent order are defined in `.github/agent-model-policy.yaml`.

## Assembly model

- Agent `.agent.md` files are assembled artifacts -- edit `context/` source files (`identity/` is immutable after bootstrap), then run `prdtp-agents-functions-cli agents assemble`.
- Assembly formula: `identity/{name}.md` + CONTEXT ZONE divider + `context/shared-context.md` + `context/{name}.md` -> `{name}.agent.md`.
- The context zone below the divider is the concatenation of `context/shared-context.md` (shared across all agents) followed by `context/{name}.md` (per-agent overlay).
- Validation checks verify the divider is present, source files exist, shared-context is included, and assembly is in sync.

## Immutability

- Agent identity (`identity/{name}.md`) is immutable after bootstrap: `## Personality`, `## Decision heuristics`, `## Anti-patterns` sections must not be modified.
- Context (`context/{name}.md`) is mutable via enrich-agents prompts only.

## Memory interaction scope

- The `## Memory interaction` section must declare: canonical files (read/write) and Git context (read when applicable).
- Operational state lives in `docs/project/*.yaml` files (handoffs, findings, releases).
- Agents read/write operational YAML via `prdtp-agents-functions-cli state` subcommands (`prdtp-agents-functions-cli state handoff create`, `prdtp-agents-functions-cli state finding create`, etc.).
- Direct line edits to operational YAML are out of contract.
- A passive SQLite audit ledger exists but agents do not interact with it. Audit entries are recorded automatically by infrastructure.
- `.github/workspace-capabilities.yaml` is the persisted detection + authorization contract for runtime commands that consult Git, `gh`, SQLite audit, reporting, or markdownlint.
- `.state/reporting/report-snapshot.json` is the shared reporting source. Agents do not write it directly; they refresh it through `prdtp-agents-functions-cli report snapshot` or `prdtp-agents-functions-cli report dashboard`.
- If Git capability is disabled, the agent must not create commits, branches, or PRs and must use `.state/local-history/` as the evidence layer instead.
- If SQLite capability is disabled, the agent must expect spool-only audit behavior and must not treat missing DB writes as a product-state failure.

## Model routing

- Agent `model:` frontmatter is active in VS Code / IDE environments only.
- GitHub.com ignores `model:` and uses its own runtime model selection.
- The ordered list in `model:` is a fallback stack for availability, not an instruction to spend more model budget by default.
- Task-size escalation is handled through dedicated prompts in `.github/prompts/`, not by changing the default agent order ad hoc.
- Only official GA models listed in `.github/agent-model-policy.yaml` are allowed.
- Preview names, aliases, or ambiguous labels such as `GPT-4`, `Gemini 3.1 Pro`, or `Gemini 3 Flash` are out of contract.

## Onboarding and PRD entrypoint

- `AGENTS.md` is the workspace overview for agents and operators.
- `.state/bootstrap-report.md` is the bootstrap summary produced by infrastructure.
- `bootstrap-from-prd` is the default entrypoint when a PRD is already available and clear enough to initialize canonical product memory.
- Bootstrap generates a local governance skeleton. A fresh workspace remains `bootstrapped` and `not_ready` until governance is configured explicitly.
- Do not route work to implementation agents until the PRD is clear on goal, users, scope, out-of-scope boundaries, constraints, and acceptance criteria.
- Do not treat a vague PRD as "good enough" just to keep momentum.

## Blocked-state contract

When an agent is blocked, it must explain the state with these fields:

- `What happened`
- `Why blocked`
- `What input is needed`
- `Who should answer`
- `Next safe step`

If `gh` is enabled and the blocked work already has a GitHub Issue, the
execution layer should also reflect `status:blocked`.

## GitHub execution workflow

- GitHub Issues + Pull Requests are the execution layer for daily work. `docs/project/*` remains canonical memory, and `docs/project/board.md` is only a derived issues/PR snapshot.
- `.github/github-governance.yaml` is the explicit GitHub governance contract for reviewer identities, labels, reserved future project metadata, readiness, and release-gate expectations.
- `bootstrap-from-prd` is the standard PRD entrypoint, and `clarify-prd` remains available for a dedicated clarification pass when PRD cleanup needs to be isolated before planning continues.
- In Git-authorized workspaces, any agent changing code, configuration, or owned canonical artifacts starts from an assigned GitHub Issue and uses the controlled branch wrapper:
  - `prdtp-agents-functions-cli git checkout-task-branch --role <role> --issue-id <id> --slug <slug>`
  - Branch naming convention: `<role>/<issue-id>-slug`.
  - Manual `git fetch/checkout/pull` sequences are out of contract when the wrapper exists.
- If `capabilities.git.authorized.enabled=false`, `git checkout-task-branch` is out of contract. Local-only work may proceed without an issue ID, but it still must close through `prdtp-agents-functions-cli git finalize` so evidence lands under `.state/local-history/`.
- Direct work on `main` or `develop` is out of contract in Git-enabled workspaces. All Git-backed changes land through PRs.
- Commits follow Conventional Commits with role scope and issue reference, for example `feat(frontend): GH-123 checkout form`.
- Before asking for merge in a Git-authorized workspace, the author or operator opens or updates a PR to `develop`, completes `.github/PULL_REQUEST_TEMPLATE.md`, applies one `role:*`, one `kind:*`, and one `priority:*` label, and reviews PR comments plus commit comments. `prdtp-agents-functions-cli validate pr-governance` and `validate release-gate` are the supported contract checks.
- A task is not complete until `prdtp-agents-functions-cli git finalize` succeeds. That command is the supported closure path for Git-enabled and local-only workspaces.
- `prdtp-agents-functions-cli git finalize` runs the shared pre-commit validator itself before creating Git evidence. After that validation passes, it may use `git commit --no-verify` to avoid host-specific hook failures without weakening governance checks.
- `tech-lead` converts refined stories into GitHub Issues through `prdtp-agents-functions-cli github issue create`, `github issue update`, and `github issue label`, then assigns execution by role.
- `pm-orchestrator` keeps the task board, blockers, and cross-role coordination visible.
- `devops-release-engineer` is the final approval gate before merge and controls release promotion to `main`.
- `docs/project/board.md` is the detailed execution snapshot.
- `docs/project/management-dashboard.md` is the executive management view generated by `prdtp-agents-functions-cli report dashboard`.
- `prdtp-agents-functions-cli report serve` is the supported read-only visual surface for PM and TL.
- `prdtp-agents-functions-cli report pack` is the supported path for CSV and XLSX exports.

## Execute boundaries

- Role-level `execute` is a controlled exception required by CLI-driven governance and task closure, not a blanket permission to run arbitrary commands.
- Prompt frontmatter should omit `execute` whenever the workflow can complete safely without shell or runtime-command use.
- `product-owner`, `ux-designer`, and `pm-orchestrator` are limited to `prdtp-agents-functions-cli` commands, state inspection, and coordination operations.
- `backend-developer`, `frontend-developer`, `qa-lead`, `devops-release-engineer`, `tech-lead`, and `software-architect` may additionally run build, test, lint, and dependency management commands (e.g., `npm install`, `dotnet build`, `pytest`) when clearly tied to their delivery scope. This does not extend to arbitrary shell commands, system administration, or network operations outside the workspace.
- If the platform cannot enforce per-command restrictions, treat this as a residual risk compensated by `.github/workspace-capabilities.yaml`, `.github/github-governance.yaml`, CODEOWNERS, PR workflows, `prdtp-agents-functions-cli git finalize`, and local/SQLite audit evidence.

### Per-role allowed calls

The execution path is the workspace-local `prdtp-agents-functions-cli` runtime under `.agents/bin/prd-to-product-agents/`. CI may install the same binary into `PATH`, but agents must treat the workspace-local copy as canonical.

| Agent | Allowed execute calls |
|-------|----------------------|
| `pm-orchestrator` | `prdtp-agents-functions-cli git finalize`, `prdtp-agents-functions-cli git checkout-task-branch`, `prdtp-agents-functions-cli report dashboard`, `prdtp-agents-functions-cli report snapshot`, `prdtp-agents-functions-cli state handoff create/update`, `prdtp-agents-functions-cli state finding update`, `prdtp-agents-functions-cli agents assemble`, `prdtp-agents-functions-cli audit export` |
| `tech-lead` | `prdtp-agents-functions-cli git finalize`, `prdtp-agents-functions-cli git checkout-task-branch`, `prdtp-agents-functions-cli state handoff create/update`, `prdtp-agents-functions-cli state finding create/update`, `prdtp-agents-functions-cli agents assemble`, `prdtp-agents-functions-cli github issue create`, `prdtp-agents-functions-cli github issue update`, `prdtp-agents-functions-cli github issue label`, `prdtp-agents-functions-cli github issue comment` |
| `product-owner` | `prdtp-agents-functions-cli git finalize`, `prdtp-agents-functions-cli git checkout-task-branch`, `prdtp-agents-functions-cli state handoff create`, `prdtp-agents-functions-cli state finding update` |
| `software-architect` | `prdtp-agents-functions-cli git finalize`, `prdtp-agents-functions-cli git checkout-task-branch`, `prdtp-agents-functions-cli state finding create/update`, `prdtp-agents-functions-cli state handoff create` |
| `ux-designer` | `prdtp-agents-functions-cli git finalize`, `prdtp-agents-functions-cli git checkout-task-branch`, `prdtp-agents-functions-cli state handoff create` |
| `backend-developer` | `prdtp-agents-functions-cli git finalize`, `prdtp-agents-functions-cli git checkout-task-branch`, `prdtp-agents-functions-cli state handoff create` |
| `frontend-developer` | `prdtp-agents-functions-cli git finalize`, `prdtp-agents-functions-cli git checkout-task-branch`, `prdtp-agents-functions-cli state handoff create` |
| `qa-lead` | `prdtp-agents-functions-cli git finalize`, `prdtp-agents-functions-cli git checkout-task-branch`, `prdtp-agents-functions-cli state finding create/update`, `prdtp-agents-functions-cli state handoff create` |
| `devops-release-engineer` | `prdtp-agents-functions-cli git finalize`, `prdtp-agents-functions-cli git checkout-task-branch`, `prdtp-agents-functions-cli state release create/update`, `prdtp-agents-functions-cli state event record`, `prdtp-agents-functions-cli state finding create`, `prdtp-agents-functions-cli state handoff create`, `prdtp-agents-functions-cli state finding update` |

Any script call outside this table requires explicit justification in the PR description and `devops-release-engineer` sign-off.

## Context injection

- Context sections (`## Project Context`, `## Technical Context`, `## Implementation Context`) are injected by specific roles -- see `copilot-instructions.md` for the layered injection model.
