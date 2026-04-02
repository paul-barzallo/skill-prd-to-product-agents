---
description: Bootstrap canonical product memory from a clear PRD.
agent: pm-orchestrator
tools:
  - search
  - read
  - edit/editFiles
  - execute
---


# bootstrap-from-prd

## Purpose

Use a PRD as the single entrypoint for the first real planning cycle.

This prompt is allowed to initialize or update canonical docs only after the
PRD passes a clarity gate. If the PRD is incomplete, ambiguous, or internally
contradictory, stop and ask questions first.

## Context scope

- the PRD provided by the user
- `AGENTS.md`
- `.github/workspace-capabilities.yaml`
- `.github/github-governance.yaml`
- `docs/project/*` when relevant

## Read

- the PRD provided by the user
- `AGENTS.md`
- `.github/workspace-capabilities.yaml`
- `.github/github-governance.yaml`
- `docs/project/*` when relevant

## Write

- canonical files under `docs/project/`
- handoffs in `docs/project/handoffs.yaml` through `prdtp-agents-functions-cli --workspace . state handoff create`
- findings in `docs/project/findings.yaml` through `prdtp-agents-functions-cli --workspace . state finding create`
- `docs/project/open-questions.md` for unresolved product or technical doubts

Do not use free-form chat as the system of record.

## PRD clarity gate

Before writing or updating canonical docs, validate that the PRD is clear on:

- product goal
- target users
- in-scope capabilities
- out-of-scope capabilities
- constraints
- acceptance criteria

If any of those are missing or unclear:

1. stop immediately,
2. ask structured questions,
3. do not start implementation,
4. do not invent scope,
5. use `clarify-prd` if the clarification step needs its own dedicated pass.

## Blocked response format

If the PRD does not pass the clarity gate, reply using this exact structure:

- `What happened`: the PRD is missing, ambiguous, or contradictory on specific points.
- `Why blocked`: the system cannot safely initialize canonical docs or execution without those answers.
- `What input is needed`: the exact missing answers.
- `Who should answer`: usually the user, product-owner, or client representative.
- `Next safe step`: answer the questions first, then rerun `bootstrap-from-prd`.

If `gh` is enabled and an issue already exists for the work, the operational
state should also reflect the block with `status:blocked`.

## Autonomy directive

Complete all steps in this workflow autonomously. Do **not** stop and ask the
user for routine decisions (which files to update, whether to create handoffs,
etc.). Only stop and ask when you encounter a genuine blocker - such as the PRD
failing the clarity gate or a script producing an unexpected error.

## Branching

Before making Git changes, read `.github/workspace-capabilities.yaml`.

If `capabilities.git.authorized.enabled=true`, create a working branch from
`develop` using the controlled branch-checkout script:

```shell
prdtp-agents-functions-cli --workspace . git checkout-task-branch --role pm-orchestrator --issue-id "<issue-id>" --slug bootstrap-from-prd
```

If no issue ID exists yet, stop and create or identify the tracking issue first.
Create that issue through the workspace's approved GitHub operating process.
This Git-authorized path requires `--issue-id` and uses the `product/` branch
prefix from the runtime contract. Never commit to `main` or `develop` directly.

If `capabilities.git.authorized.enabled=false`, do not call
`git checkout-task-branch` and do not treat an issue ID as a hard prerequisite.
Stay in local-only mode and close the workflow through
`prdtp-agents-functions-cli --workspace . git finalize`, which writes auditable evidence under
`.state/local-history/` instead of creating a Git commit.

## Process

This workflow uses **sequential delegation** following the L0->L1 hierarchy.
PM (L0) validates the PRD, then delegates to L1 agents one at a time, reviewing
each report-back before proceeding. This keeps each agent's context window
focused on its own domain.

### 1. Validate the workspace contract

- Read `.github/workspace-capabilities.yaml` before using Git, `gh`, or any
  optional tooling.
- Read `.github/github-governance.yaml` to understand current readiness and
  GitHub execution expectations.

### 2. Run the PRD clarity gate

- Extract explicit answers from the PRD.
- List any unclear or missing points.
- If the PRD fails the gate, stop and ask.

### 3. Create a skeleton of canonical memory

Initialize the minimal canonical product layer yourself (PM scope):

- `docs/project/context-summary.md` - brief project summary from the PRD
- `docs/project/open-questions.md` - unresolved doubts from the PRD

Keep the content traceable to the PRD. Do not add invented roadmap, stories,
or architecture details.

### 4. Delegate to product-owner (L1) - product refinement

Use the sub-agent tool to delegate to `product-owner` with a scoped prompt:

> **Task**: From the attached PRD, create or update canonical product docs.
> **Files to read**: the PRD (provided), `docs/project/context-summary.md`,
> `docs/project/open-questions.md`
> **Files to create/update**: `docs/project/vision.md`,
> `docs/project/scope.md`, `docs/project/backlog.yaml`,
> `docs/project/refined-stories.yaml`, `docs/project/acceptance-criteria.md`
> **Report back** with: summary of what was created, list of files changed,
> any scope questions found, and recommendation for next step.

**Review the report-back** before proceeding. If PO reports `blocked`, resolve
the blocker before continuing. If PO recommends UX work, note it for after
architecture.

### 5. Delegate to software-architect (L1) - technical framing

Use the sub-agent tool to delegate to `software-architect` with a scoped prompt:

> **Task**: Given the product scope, create the initial architecture overview
> and relevant ADRs.
> **Files to read**: `docs/project/scope.md`, `docs/project/backlog.yaml`,
> `docs/project/refined-stories.yaml`
> **Files to create/update**: `docs/project/architecture/overview.md`,
> `docs/project/decisions/` (ADRs as needed),
> `docs/project/refined-stories.yaml` (tech_notes, edge_cases only)
> **Report back** with: summary of architecture decisions, list of files
> changed, risks identified, and recommendation for next step.

**Review the report-back** before proceeding. If architect reports risks or
blockers, create a finding and decide whether to continue or replan.

### 6. Delegate to tech-lead (L1) - execution planning

Use the sub-agent tool to delegate to `tech-lead` with a scoped prompt:

> **Task**: Given the stories and architecture, create implementation maps
> and qa_notes for development readiness.
> **Files to read**: `docs/project/refined-stories.yaml`,
> `docs/project/architecture/overview.md`, `docs/project/decisions/*`,
> `docs/project/acceptance-criteria.md`
> **Files to update**: `docs/project/refined-stories.yaml`
> (implementation_map, qa_notes)
> **Report back** with: summary of stories ready for development, list of
> files changed, any gaps found, and recommendation for next step.

**Review the report-back** before proceeding.

### 7. Create coordination handoffs

After all three delegations complete, create handoff records for audit
traceability using `prdtp-agents-functions-cli --workspace . state handoff create`.

**Exact parameter names and valid values:**

```
--from-role      <string>     # Your role: "pm-orchestrator"
--to-role        <string>     # Target role: "product-owner", "software-architect", "tech-lead", etc.
--handoff-type   <string>     # One of: normal, escalation, rework, approval
--entity         <string>     # What is being handed off: e.g. "prd-init", "scope-review"
--reason         <string>     # One of: new_work, needs_refinement, needs_rework, blocked,
                              #         ready_for_review, ready_for_release, scope_change,
                              #         technical_risk, environment_issue, client_rejected
--details        <string>     # (Optional) Additional context
--id             <string>     # (Optional) Custom handoff ID
```

**Example:**

```shell
prdtp-agents-functions-cli --workspace . state handoff create \
    --from-role "pm-orchestrator" \
    --to-role "software-architect" \
    --handoff-type "normal" \
    --entity "technical-framing" \
    --reason "new_work" \
    --details "PRD clarity gate passed. Canonical docs initialized. Technical framing needed."
```

### 8. Commit all changes

After updating canonical docs and creating handoffs, close the workflow through
the supported finalize path before completing it.

If Git is enabled, use your working branch and include the issue reference plus
commit message:

```shell
prdtp-agents-functions-cli --workspace . git finalize \
  --agent-role "pm-orchestrator" \
  --summary "Initialized canonical product memory from PRD." \
  --issue-ref "GH-<id>" \
  --commit-message "docs(ops): GH-<id> initialize canonical memory from PRD" \
  --files-changed "docs/project/vision.md,docs/project/scope.md,docs/project/backlog.yaml,docs/project/acceptance-criteria.md,docs/project/handoffs.yaml" \
  --canonical-docs-changed "docs/project/vision.md,docs/project/scope.md,docs/project/backlog.yaml,docs/project/acceptance-criteria.md" \
  --handoffs "docs/project/handoffs.yaml" \
  --validation-status passed
```

If Git is disabled, run the same closure path without `--issue-ref` and
`--commit-message`; local-only mode records evidence instead of creating a Git
commit:

```shell
prdtp-agents-functions-cli --workspace . git finalize \
  --agent-role "pm-orchestrator" \
  --summary "Initialized canonical product memory from PRD." \
  --files-changed "docs/project/vision.md,docs/project/scope.md,docs/project/backlog.yaml,docs/project/acceptance-criteria.md,docs/project/handoffs.yaml" \
  --canonical-docs-changed "docs/project/vision.md,docs/project/scope.md,docs/project/backlog.yaml,docs/project/acceptance-criteria.md" \
  --handoffs "docs/project/handoffs.yaml" \
  --validation-status passed
```

Do **not** yield control with uncommitted changes. Uncommitted files after a
workflow is a contract violation.

### 9. Keep implementation blocked until planning is ready

Do not delegate to `backend-developer` or `frontend-developer` at any point
during this workflow. L2 agents are delegated by tech-lead, not by PM.

Implementation cannot start until:

- the PRD is clear,
- canonical product docs are initialized (PO delegation complete),
- architecture is framed (Architect delegation complete),
- implementation maps exist (TL delegation complete),
- acceptance criteria are defined.

### 10. Route governance configuration

Do not ask the user to run governance configuration manually from this
workflow. If `.github/github-governance.yaml` still contains `REPLACE_ME`
placeholders or readiness is still `bootstrapped`, note that
bootstrap governance remediation is still required.

This is informational -- do not block the PRD workflow on governance. The
`release-readiness` prompt will enforce governance as a hard gate later.

## Exit

Present results to the user with:

- **Task**: PRD bootstrap
- **Status**: done | blocked | partial
- **Summary**: Up to 5 sentences covering PO, Architect, and TL delegation outcomes
- **Artifacts changed**: list of canonical docs created or modified
- **Findings**: issues or open questions discovered
- **Next recommendation**: suggested next workflow (e.g., `enrich-agents-from-prd`)

## Success criteria

- the PRD passed the clarity gate,
- canonical product docs reflect the PRD without invention,
- any missing answers are written to `open-questions.md`,
- initial handoffs exist where needed,
- no implementation work starts on an ambiguous PRD,
- the user is informed about pending governance configuration if placeholders remain.

<!-- markdownlint-enable MD013 -->
