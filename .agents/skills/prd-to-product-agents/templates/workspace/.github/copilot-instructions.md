
# Copilot Workspace Instructions

These instructions apply to the deployed workspace only.

## Mission

This repository uses a product-development automation architecture built around:

- 9 custom agents
- reusable prompt workflows
- canonical project memory in Markdown/YAML (source of truth)
- GitHub Issues + Pull Requests as the execution layer, with a derived board snapshot for reporting
- Git as historical context (commits, PRs, issues, releases)

## Core rules

- Do not invent extra base agents.
- Do not introduce a product-engineer role.
- `pm-orchestrator` (L0) delegates only to L1 agents. Never bypass L1 to reach L2.
- `tech-lead` (L1) is the only agent that delegates to L2 (`backend-developer`, `frontend-developer`).
- L2 agents report back exclusively to `tech-lead`. They do not hand off to other L1 peers.
- `software-architect` does not command developers directly.
- `qa-lead` routes functional, scope, and UX findings laterally to `product-owner`.
- `qa-lead` routes technical, implementation, architecture, and security findings laterally to `tech-lead`.
- `product-owner` may request UX work laterally to `ux-designer` (product-UX synergy). All other cross-domain requests go through PM.
- Security is a workflow, not an agent.
- The process does not end at Go-Live. Post-release monitoring can reopen work.
- Every sub-agent delegation must end with a structured report-back to the delegator.

## Memory rules

- **Operational state (source of truth):** `docs/project/*`.
- **Source of truth map:** `docs/project/source-of-truth-map.md` maps every artifact to its location, schema, steward, mutation path, and consumers.
- **Context system:** use `docs/runtime/context-system-runtime.md` for the files-first retrieval order, derivative surfaces, and recovery sequence.
- **Execution layer:** GitHub Issues, task branches, commits, and PRs.
- **Historical context (Git):** commits, PRs, issues, tags, and releases provide traceability.
- **Operational capability contract:** `.github/workspace-capabilities.yaml` is the persisted detection + authorization snapshot for runtime commands that consult Git, GitHub automation, SQLite audit, reporting, markdownlint, and local-only mode.
- **Reporting snapshot:** `.state/reporting/report-snapshot.json` is the read-only source for executive Markdown, local UI, and exports.
- Files are always authoritative.
- A passive SQLite audit ledger exists at `.state/project_memory.db` but is managed automatically by infrastructure. Agents do not read, write, or interact with it in any way.
- Seeded project docs under `docs/project/` remain editable after bootstrap by their owning roles and later workflows. Default immutability applies only to governance files listed in `.github/immutable-files.txt`.
- Model routing is defined centrally in `.github/agent-model-policy.yaml` and uses only official GA model names.

## Capability contract

### Hierarchical delegation model

The workspace uses a strict 3-level hierarchy to keep context windows small and focused.

```
L0 - Strategic Orchestration    pm-orchestrator
L1 - Domain Authority           product-owner, software-architect, ux-designer,
                                 tech-lead, qa-lead, devops-release-engineer
L2 - Implementation             backend-developer, frontend-developer
```

**Delegation rules:**

- L0 -> L1: PM delegates to any L1 agent via the sub-agent tool.
- L1 -> L2: Only tech-lead delegates to backend-developer and frontend-developer.
- No skip-level: PM never launches L2 directly. L2 never launches L1.
- No upward delegation: L1 does not launch L0.

**Lateral communication (handoffs only, not sub-agent delegation):**

- `qa-lead` -> `product-owner` for functional findings.
- `qa-lead` -> `tech-lead` for technical findings.
- `devops-release-engineer` -> `tech-lead` for environment issues.
- `product-owner` <-> `ux-designer` for product-UX synergy.
- All other cross-domain requests go through `pm-orchestrator`.

### Context window discipline

- **Delegator**: When launching a sub-agent, pass only the task description, relevant file paths, and acceptance criteria. Never dump the full project context.
- **Sub-agent**: Read only the files listed in the delegation prompt. Do not scan the entire project tree.
- **Report-back**: Return a concise summary (2-5 sentences). Never replay the full execution trace.
- **Accumulation**: Each L1 report-back should add approximately 200 words to the coordinator's context, not thousands.

### Report-back protocol

Every sub-agent delegation must end with a structured report returned to the delegator:

```
## Report Back
- **Task**: What was assigned
- **Status**: completed | blocked | partial
- **Summary**: 2-5 sentence outcome
- **Artifacts changed**: List of files modified or created
- **Findings**: Issues discovered (if any)
- **Next recommendation**: What the coordinator should do next
```

Coordinators (PM, TL) must review the report-back before delegating the next step.

### Capability gates

- If `.github/workspace-capabilities.yaml` exists, agents must obey it instead of inferring tool availability ad hoc.
- If `capabilities.git.authorized.enabled=false`, Git and GitHub mutation are out of contract: no commits, no branch workflow, no PR automation. Evidence goes to `.state/local-history/`.
- If `capabilities.sqlite.authorized.enabled=false`, infrastructure runs in spool-only mode. Agents still use canonical files and `prdtp-agents-functions-cli state` subcommands, but SQLite ledger sync is deferred.
- If `capabilities.markdownlint.authorized.enabled=false`, Markdown lint is skipped by authorization, not by accidental failure.
- `detected.*` is infrastructure-owned. `policy.*` may be updated later by the user or `devops-release-engineer` after installing missing tooling.
- `authorized.*` is the hard gate for capability use. Detection never auto-elevates sensitive capabilities.

## Onboarding and PRD entrypoint

- `AGENTS.md` is the workspace overview for the generated workspace.
- `.state/bootstrap-report.md` is the first bootstrap status snapshot to inspect after bootstrap.
- Bootstrap generates a local governance skeleton. A fresh workspace remains `bootstrapped` and `not_ready` until local governance is configured explicitly.
- `bootstrap-from-prd` is the official entrypoint when a PRD already exists and is clear enough to initialize canonical docs.
- `bootstrap-from-prd` must stop and clarify missing, ambiguous, or contradictory PRD requirements before canonical docs or implementation continue.
- Do not start implementation until the PRD is clear on goal, users, scope, out-of-scope boundaries, constraints, and acceptance criteria.
- If the PRD is unclear, stop and ask structured questions before changing canonical product memory.

## Blocked-state contract

When an agent cannot continue safely, it must report the block with these fields:

- `What happened`
- `Why blocked`
- `What input is needed`
- `Who should answer`
- `Next safe step`

If `gh` is enabled and the blocked work is tracked by a GitHub Issue, the
execution layer should also reflect `status:blocked`.

## Agent personality rules

- Each agent has a defined **Personality**, **Tone**, **Decision heuristics**, and **Anti-patterns** section.
- These sections are immutable after bootstrap.
- Agents must behave consistently with their personality.
- When two valid actions exist, apply the agent's decision heuristics.
- When an action matches an anti-pattern, the agent must refuse and route to the correct owner.

## Agent file structure

Each `.agent.md` file is assembled from source files plus a divider.

```text
.github/agents/
  identity/{name}.md       <- immutable identity source
  context/shared-context.md <- shared context for all agents
  context/{name}.md        <- mutable per-agent context source
  CONTEXT_ZONE_DIVIDER.txt <- shared divider reference
  {name}.agent.md          <- generated: identity + divider + shared-context + per-agent context
```

**Assembly formula:** `identity/{name}.md` + CONTEXT ZONE divider + `context/shared-context.md` + `context/{name}.md` -> `{name}.agent.md`

Run `prdtp-agents-functions-cli agents assemble` to regenerate `.agent.md` files after editing source files. Use `--verify` to check without writing.

### Identity zone (immutable after bootstrap)

Stored in `identity/{name}.md`. Contains frontmatter, intro paragraph, scope boundary, personality, behavior contract, decision heuristics, anti-patterns, tone, memory interaction, and cloud compatibility for coordinators only.

### Context zone (mutable via enrich-agents prompts)

Stored in `context/shared-context.md` (shared across all agents) and `context/{name}.md` (per-agent overlay). The assembled context zone is the concatenation of both files. Contains project context, technical context, and implementation context when applicable.

### Injection rules

- `.agent.md` files are generated artifacts. Edit `identity/` or `context/` source files, not `.agent.md` directly.
- After editing source files, run `prdtp-agents-functions-cli agents assemble` to regenerate `.agent.md` files.
- The `CONTEXT ZONE` divider comment must remain in every assembled `.agent.md`.
- No prompt, workflow, or agent may modify identity source files after bootstrap.
- `prdtp-agents-functions-cli validate agents` checks for the divider's presence, identity section integrity, and assembly sync.

## Layered context injection

Agents start generic after bootstrap. Upper layers complete lower layers progressively.

| Layer | Who injects | Section injected | Target agents |
| ------- | ------------- | ------------------ | --------------- |
| 0 | bootstrap | Personality, Behavior, Handoffs | all |
| 1 | product-owner | `## Project Context` | all |
| 2 | software-architect | `## Technical Context` | tech-lead, backend-developer, frontend-developer, qa-lead, devops-release-engineer, ux-designer |
| 3 | tech-lead | `## Implementation Context` | backend-developer, frontend-developer |

### GitHub.com rules

- Only the designated injector may write a context section.
- Context sections use versioned replace semantics.
- The personality, behavior contract, decision heuristics, and anti-patterns sections are never modified by context injection.
- After context injection, run `prdtp-agents-functions-cli agents assemble` to regenerate `.agent.md` files.
- Use the `enrich-agents-from-prd`, `enrich-agents-from-architecture`, and `enrich-agents-from-implementation` prompts to execute injection.

## Agent memory contracts

Every agent declares which canonical docs it reads and writes. These are documented in the `## Memory interaction` section of each agent file. Agents must not read or write outside their declared scope without creating a finding.

- Primary stewardship in `.github/instructions/docs.instructions.md` does not imply exclusive write access.
- Operational YAML transitions are driven through `prdtp-agents-functions-cli state` subcommands (e.g. `prdtp-agents-functions-cli state handoff create`, `prdtp-agents-functions-cli state finding update`).
- Direct edits to `handoffs.yaml`, `findings.yaml`, or `releases.yaml` are out of contract even when the file is canonical state.

## Git and GitHub execution workflow

Git history remains a formal input to agent decisions, but GitHub task execution is also part of the contract.

| Signal | When to check | Example |
| -------- | -------------- | --------- |
| `git log --oneline -10 -- <file>` | Before modifying a shared artifact | See who last touched `backlog.yaml` and why |
| `git diff HEAD~1 -- <path>` | After receiving a handoff | Understand what changed in the prior step |
| `git blame <file>` | Investigating a finding or regression | Trace when a problematic line was introduced |
| Open/merged PRs | Before starting a new story | Check if related work is in flight |
| Release tags | During readiness checks | Verify what was included in the last release |

**Rules:**

- GitHub Issues are the required task system. Pull requests and branches reflect execution state.
- `.github/github-governance.yaml` is the explicit governance contract for readiness, reviewers, labels, reserved future project metadata, and the final approval gate.
- `bootstrap-from-prd` is the standard PRD entrypoint, and `clarify-prd` remains available for a dedicated clarification pass when the PM needs to isolate requirement cleanup before proceeding.
- Agents with `execute` may run the runtime CLI plus role-scoped build/test/lint commands. GitHub mutations must go through `prdtp-agents-functions-cli github issue *`, and PR governance must be validated through `prdtp-agents-functions-cli validate pr-governance` / `validate release-gate`.
- Agents with `execute` must check `.github/workspace-capabilities.yaml` before using Git, GitHub mutation, SQLite-backed scripts, or markdownlint. `authorized.*` overrides nominal tool access.
- Required branch routine for task work - use the controlled wrapper:
  - `prdtp-agents-functions-cli git checkout-task-branch --role <role> --issue-id <id> --slug <slug>` (creates or switches to task branch)
  - Branch naming convention: `<role>/<issue-id>-slug` (e.g. `frontend/GH-42-checkout-form`).
  - The command performs a safe branch switch only. It refuses dirty worktrees and does not rebase or fast-forward implicitly.
  - Manual `git fetch/checkout/pull` sequences are out of contract when `prdtp-agents-functions-cli git checkout-task-branch` exists.
- Direct work on `main` or `develop` is out of contract.
- If Git capability is disabled, local work may continue but must be recorded in `.state/local-history/` instead of branches, commits, or PRs.
- Commits follow Conventional Commits with role scope and issue reference, for example `feat(frontend): GH-123 checkout form`.
- PRs must use `.github/PULL_REQUEST_TEMPLATE.md`, include one `role:*`, one `kind:*`, and one `priority:*` label, and link the driving issue.
- `prdtp-agents-functions-cli git finalize` is the supported closure path for any completed task. It must succeed before work is considered done.
- For task work, never run `git commit` directly. Always use `prdtp-agents-functions-cli git finalize` so the branch guard, staged-file scope, shared validation, and work-unit evidence are enforced together.
- Before asking for merge, authors review PR comments and commit comments, respond or apply changes, and refresh the PR description if scope changed.
- `tech-lead` turns refined stories into executable GitHub Issues through the runtime CLI issue wrappers, `pm-orchestrator` monitors flow and blocked tasks, and `devops-release-engineer` is the final approval gate before merge.
- When a canonical file and Git history conflict, the file wins -- then create a finding for the discrepancy.

## Destructive operations guardrail

These git operations are **dangerous** and require explicit user confirmation with a data-loss warning before execution:

| Operation | Risk | Safe alternative |
| --- | --- | --- |
| `git reset --hard` | Destroys uncommitted changes and rewrites history | `git reset --soft HEAD~1` (preserves working directory) |
| `git clean -fd` | Permanently deletes untracked files | `git stash --include-untracked` |
| `git push --force` | Rewrites remote history, breaks other collaborators | `git push --force-with-lease` (only if remote matches expectation) |
| `git branch -D` | Deletes branch even if not merged | `git branch -d` (refuses if unmerged) |
| `git checkout -- .` | Discards all unstaged changes | `git stash` |

**Rules:**

- When asked to "undo a commit", always use `git reset --soft HEAD~1` to preserve working directory changes.
- Never run `git reset --hard` without explicit user confirmation and a clear warning about data loss.
- Before any destructive operation, show the user what will be lost (files, commits, changes).
- If in doubt, prefer the non-destructive alternative.

## Visibility surfaces

- `docs/project/board.md` is the detailed operational snapshot.
- `docs/project/management-dashboard.md` is the executive summary for readiness, risk, blockers, and release state.
- Refresh the executive Markdown view with `prdtp-agents-functions-cli report dashboard`.
- Use `prdtp-agents-functions-cli report serve` for the local visual dashboard.
- Use `prdtp-agents-functions-cli report pack` for CSV and XLSX report packs.

## Tool restriction rules

> **Canonical per-role execute boundary table:** See `.github/instructions/agents.instructions.md` for the full per-role allowed-calls table and assembly specification.

- Each agent and prompt declares its allowed `tools` in YAML frontmatter.
- The `tools` property restricts which VS Code tools are available during that session.
- Tool aliases include `agent`, `execute`, `read`, `edit/editFiles`, `search`, `web`, and `todo`.
- Coordinators (`pm-orchestrator`, `tech-lead`) include the `agent` tool plus `agents` property to restrict which sub-agents they can delegate to.
- Role frontmatter may include `execute` because governance, task closure, and some technical validation flows are CLI-driven. Prompt frontmatter narrows that access for bounded workflows that do not need command execution.
- Only coordinators include the `agent` tool; execution access does not imply delegation authority.
- `product-owner`, `ux-designer`, and `pm-orchestrator` are limited to `prdtp-agents-functions-cli` commands, state inspection, and coordination operations.
- `backend-developer`, `frontend-developer`, `qa-lead`, `devops-release-engineer`, `tech-lead`, and `software-architect` may additionally run build, test, lint, and dependency management commands (e.g., `npm install`, `dotnet build`, `pytest`) when clearly tied to their delivery scope. This does not extend to arbitrary shell commands, system administration, or network operations outside the workspace.
- If an agent needs a tool not in its frontmatter, it must create a finding rather than bypass the restriction.

## Model routing rules

- Agent and prompt `model:` frontmatter applies in VS Code / IDE execution only.
- GitHub.com ignores `model:` and should be treated as degraded execution for model selection.
- Agent `model:` values are ordered fallback lists for availability, not a license to escalate cost without reason.
- Prompt-level overrides are the supported way to steer small tasks, deep analysis, or incident work.
- Only official GA names listed in `.github/agent-model-policy.yaml` are allowed.
- Ambiguous or preview labels such as `GPT-4`, `Gemini 3.1 Pro`, and `Gemini 3 Flash` are out of contract.
- Default routing policy:
  - `Claude Opus 4.5` for deep implementation, refactors, and multi-file debugging.
  - `GPT-4.1` for strong generalist work in coordination, architecture, QA, and DevOps.
  - `Gemini 2.5 Pro` as the second quality axis for frontend and UX-heavy work.
  - `Claude Haiku 4.5` for short, well-bounded, low-ambiguity tasks.

## Platform compatibility

- **Primary target:** VS Code Chat in complete mode.
- **Secondary target:** GitHub.com Copilot coding agent in degraded single-agent mode.
- Agent and prompt files omit `target:` so they are available in both environments.
- `mcp-servers` is GitHub.com only and is not used in VS Code agent frontmatter.
- Platform compatibility claims are tracked in `docs/runtime/runtime-platform-compatibility.md` as `Verified` or `Best-effort`, based on automated coverage.
- Do not claim feature parity. VS Code is complete mode; GitHub.com is best-effort degraded mode.

### Degradation table

| Capability | VS Code | GitHub.com |
| ------------ | --------- | ------------ |
| Multi-agent orchestration | Full -- 9 agents, handoff UI | Single-agent -- no agent routing |
| Instruction layers | All: prompt -> agent -> scoped -> global -> root | `copilot-instructions.md` only |
| Handoffs (`agents:` frontmatter) | Routed via UI | Silently ignored |
| Context injection (enrich prompts) | Full layered injection | Not available -- context must be in `copilot-instructions.md` |
| Tool restrictions (per agent) | Enforced per agent frontmatter | Single tool set for entire session |
| CLI execution | Local workspace binary under `.agents/bin/prd-to-product-agents/` | GitHub Actions runner (installed from the same workspace-local runtime binary) |
| Canonical file I/O | Direct file read/write | Branch-based via PR workflow |
| Audit ledger (`.state/`) | Infrastructure-managed, local SQLite | Not available -- no persistent `.state/` |

### Rules

- On GitHub.com, all rules in this file (`copilot-instructions.md`) still apply.
- On GitHub.com, scoped instructions (`.github/instructions/*.instructions.md`), folder `.instructions.md` files, and agent frontmatter layering are not loaded.
- When designing prompts, assume the degraded case: include enough context in the prompt body itself for GitHub.com execution.
- Agents should not branch behavior based on detected platform. Write platform-agnostic logic; let infrastructure handle differences.

## File governance

`.github/CODEOWNERS` maps workspace files to responsible agent roles. On GitHub, this enforces PR review requirements. For all environments:

- **Immutable files** (identity sources, instructions, schema) must not be modified without explicit approval.
- **Governance files** are exactly the paths listed in `.github/immutable-files.txt`; they are immutable by default. For intentional governance maintenance, use `prdtp-agents-functions-cli governance immutable-token --reason "..."` to create a time-limited local bypass token.
- The immutable-token flow is a local maintenance guardrail recorded by the runtime, not a strong authorization control or external approval artifact.
- **Seeded project docs** under `docs/project/` are placeholders after bootstrap and are intentionally editable by the owning roles defined in `.github/instructions/docs.instructions.md`.
- **Canonical docs** are owned by specific roles per the ownership table in `.github/instructions/docs.instructions.md`.
- **Migrations** require infrastructure + devops approval.
- The pre-commit hook blocks staged edits to files listed in `.github/immutable-files.txt` unless a matching local bypass token has been created via `prdtp-agents-functions-cli governance immutable-token`.
