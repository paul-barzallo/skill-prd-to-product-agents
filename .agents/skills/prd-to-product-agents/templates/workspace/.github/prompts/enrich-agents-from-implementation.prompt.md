---
description: Inject Implementation Context into developer agents after implementation planning.
agent: tech-lead
tools:
  - search
  - read
  - edit/editFiles
  - execute
---


# enrich-agents-from-implementation

## Purpose

After preparing implementation maps, resolving dependencies and defining coding conventions, inject the `## Implementation Context` section into developer agents' **context files** so they have everything needed to write code.

## Layer

This is **Layer 3** injection. Only `tech-lead` executes this prompt.

## Context scope

- `docs/project/refined-stories.yaml` (implementation_map for assigned stories)
- `docs/project/architecture/overview.md`
- `docs/project/architecture/` (relevant component docs)
- `docs/project/decisions/` (relevant ADRs)
- `docs/project/acceptance-criteria.md`

## Write

For each target developer, update its **context file** at `.github/agents/context/{name}.md`:

- `backend-developer`
- `frontend-developer`

Fill or update the content under `## Implementation Context` with structured implementation guidance.
Do NOT edit `.github/agents/identity/{name}.md` or `.github/agents/{name}.agent.md` directly.

## Rules

- Edit only `.github/agents/context/{name}.md` files.
- Each developer receives implementation context for their domain:
  - `backend-developer` gets: project directory structure, naming conventions, file organization patterns, testing approach (unit/integration), environment variables, database migration strategy, API versioning rules, error code conventions, and logging standards.
  - `frontend-developer` gets: component file structure, naming conventions, state management patterns, routing conventions, form handling approach, i18n strategy, testing approach (unit/component/e2e), and asset management rules.
- Include concrete examples where possible (file paths, naming patterns, code snippets).
- Keep each injection concise: 20-40 lines of structured Markdown.
- **First run**: fill the empty section below the HTML comment.
- **Subsequent runs**: use versioned replace -- overwrite the full section, add `<!-- injected: YYYY-MM-DD by tech-lead -->` at the top.
- After updating all context files, run the assemble-agents command to regenerate `.agent.md` files:
  ```
  prdtp-agents-functions-cli --workspace . agents assemble
  ```

## Success criteria

- Both developer agents' context files have a non-empty `## Implementation Context` section.
- No identity file (`identity/{name}.md`) was modified.
- The `.agent.md` files were regenerated via the assemble-agents script.
- The injected content is specific enough that a developer can start coding without ambiguity about conventions.

## Exit

Report back to `pm-orchestrator` with:

- **Task**: implementation context injection
- **Status**: done | blocked | partial
- **Summary**: Up to 3 sentences of outcome
- **Artifacts changed**: files created or modified
- **Findings**: issues discovered, if any
- **Next recommendation**: suggested next delegation or action
