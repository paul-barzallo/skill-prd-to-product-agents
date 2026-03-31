---
description: Apply a small, bounded documentation or canonical-file update with the lightweight model path.
agent: product-owner
tools:
  - search
  - read
  - edit/editFiles
model:
  - Claude Haiku 4.5
  - GPT-4.1
---


# small-doc-update

Use this prompt when the work is a bounded update to canonical docs and does not require large-scale re-planning.

## Context scope

- the target document(s) specified by the user
- `docs/project/scope.md` for boundary reference

## Typical fit

- acceptance-criteria edits
- glossary or stakeholder cleanup
- backlog wording updates
- UX or product notes that do not change system scope

## Guardrails

- Do not use this prompt for major scope changes or release re-planning.
- Do not mutate operational YAML through runtime commands from this prompt.
- Do not use this prompt for branch management, commit creation, or release closure.
- If the update changes architecture or execution, report back to `pm-orchestrator`.

## Exit

Report back to `pm-orchestrator` with:

- **Task**: doc update requested
- **Status**: done | blocked | partial
- **Summary**: Up to 3 sentences of outcome
- **Artifacts changed**: files created or modified
- **Findings**: issues discovered, if any
- **Next recommendation**: suggested next delegation or action

## Write

- Limit changes to the targeted canonical docs.
- If the update reveals a blocker or follow-up workflow step, describe it in the report-back instead of invoking runtime CLI state or Git commands from this prompt.
