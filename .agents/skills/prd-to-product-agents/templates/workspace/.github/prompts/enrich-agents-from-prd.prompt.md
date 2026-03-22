---
description: Inject Project Context into all agents after PRD processing.
agent: product-owner
tools:
  - search
  - read
  - edit/editFiles
  - execute
---


# enrich-agents-from-prd

## Purpose

After processing the PRD and defining vision, scope, backlog and acceptance criteria, inject the `## Project Context` section into the **shared context file** so all agents understand the domain.

## Layer

This is **Layer 1** injection. Only `product-owner` executes this prompt.

## Context scope

- `docs/project/vision.md`
- `docs/project/scope.md`
- `docs/project/backlog.yaml`
- `docs/project/releases.md`
- `docs/project/stakeholders.md`
- `docs/project/glossary.md`

## Write

Update `.github/agents/context/shared-context.md`:

- Fill or replace the content under `## Project Context` with a unified domain summary.
- Do NOT edit `.github/agents/identity/{name}.md`, `.github/agents/{name}.agent.md`, or per-agent context overlays.

Optionally, add brief agent-specific focus notes in per-agent overlay files (`context/{name}.md`) when a role needs a particular emphasis that doesn't belong in the shared block.

## Rules

- Primary target is `context/shared-context.md`. Per-agent overlays are secondary and optional.
- The shared Project Context must cover all domain dimensions in one block:
  - Project milestones, stakeholders, release plan (coordinator perspective).
  - User problems, success metrics, scope boundaries (product perspective).
  - Target users, key journeys, accessibility requirements (UX perspective).
  - Domain model, integrations, NFRs, constraints (architecture perspective).
  - Module boundaries, story dependencies, delivery priorities (implementation perspective).
  - Acceptance scope, risk areas, test priorities (quality perspective).
  - Environments, deployment targets, compliance needs (operations perspective).
- Keep the shared block concise: 20-40 lines of structured Markdown.
- **First run**: fill the empty section below the HTML comment.
- **Subsequent runs**: use versioned replace -- overwrite the full section, add `<!-- injected: YYYY-MM-DD by product-owner -->` at the top.
- After updating all context files, run the assemble-agents command to regenerate `.agent.md` files:
  ```
  prdtp-agents-functions-cli agents assemble
  ```

## Success criteria

- The shared context file (`context/shared-context.md`) has a non-empty `## Project Context` section.
- No identity file (`identity/{name}.md`) was modified.
- The `.agent.md` files were regenerated via the assemble-agents script.
- The injected content is traceable to canonical docs.

## Exit

Report back to `pm-orchestrator` with:

- **Task**: project context injection
- **Status**: done | blocked | partial
- **Summary**: Up to 3 sentences of outcome
- **Artifacts changed**: files created or modified
- **Findings**: issues discovered, if any
- **Next recommendation**: suggested next delegation or action
