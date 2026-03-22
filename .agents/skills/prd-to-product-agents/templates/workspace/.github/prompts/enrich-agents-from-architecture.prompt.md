---
description: Inject Technical Context into agents after architecture design.
agent: software-architect
tools:
  - search
  - read
  - edit/editFiles
  - execute
---


# enrich-agents-from-architecture

## Purpose

After defining the architecture, stack, data model and integrations, inject the `## Technical Context` section into the **shared context file** so technical agents understand the technical environment.

## Layer

This is **Layer 2** injection. Only `software-architect` executes this prompt.

## Context scope

- `docs/project/architecture/overview.md`
- `docs/project/architecture/` (all component docs)
- `docs/project/decisions/` (all ADRs)
- `docs/project/refined-stories.yaml` (tech_notes, edge_cases)

## Write

Update `.github/agents/context/shared-context.md`:

- Fill or replace the content under `## Technical Context` with a unified technical summary.
- Do NOT edit `.github/agents/identity/{name}.md`, `.github/agents/{name}.agent.md`, or per-agent context overlays for technical context.

Optionally, add brief role-specific technical notes in per-agent overlay files (`context/{name}.md`) when an agent needs details that don't belong in the shared block.

## Rules

- Primary target is `context/shared-context.md`. Per-agent overlays are secondary and optional.
- The shared Technical Context must cover the full stack in one block:
  - Full stack definition, module map, integration contracts, dependency graph, key ADRs.
  - Backend stack (language, framework, ORM, DB), coding patterns, API conventions.
  - Frontend stack (framework, state management, CSS approach, component library).
  - Testable boundaries, integration points, security surface, testing tool stack.
  - Infrastructure stack, CI/CD pipeline, environment topology, monitoring tools.
  - Frontend framework constraints, supported devices, performance budgets, design system.
- Keep the shared block concise: 25-50 lines of structured Markdown.
- **First run**: fill the empty section below the HTML comment.
- **Subsequent runs**: use versioned replace -- overwrite the full section, add `<!-- injected: YYYY-MM-DD by software-architect -->` at the top.
- After updating all context files, run the assemble-agents command to regenerate `.agent.md` files:
  ```
  prdtp-agents-functions-cli agents assemble
  ```

## Success criteria

- The shared context file (`context/shared-context.md`) has a non-empty `## Technical Context` section.
- No identity file (`identity/{name}.md`) was modified.
- The `.agent.md` files were regenerated via the assemble-agents script.
- The injected content is traceable to architecture docs and ADRs.

## Exit

Report back to `pm-orchestrator` with:

- **Task**: technical context injection
- **Status**: done | blocked | partial
- **Summary**: Up to 3 sentences of outcome
- **Artifacts changed**: files created or modified
- **Findings**: issues discovered, if any
- **Next recommendation**: suggested next delegation or action
