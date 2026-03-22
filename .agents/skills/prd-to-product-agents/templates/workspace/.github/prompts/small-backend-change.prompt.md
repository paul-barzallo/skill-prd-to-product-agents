---
description: Execute a small, low-risk backend change with the lightweight model path.
agent: backend-developer
tools:
  - search
  - read
  - execute
  - edit/editFiles
model:
  - Claude Haiku 4.5
  - GPT-4.1
  - Claude Opus 4.5
---


# small-backend-change

Use this prompt only when the backend change is narrow, low-risk, and already bounded by the implementation map.

## Context scope

Read only these files before starting:

- The assigned GitHub Issue
- `docs/project/refined-stories.yaml` (your assigned story and its `implementation_map`)
- `docs/project/acceptance-criteria.md` (target story only)
- Architecture files referenced in the `implementation_map`

Do not scan the full project tree.

## Typical fit

- one endpoint or handler fix
- validation or logging adjustment
- a single-file or small multi-file backend change
- no architecture redesign

## Guardrails

- Stay inside the assigned GitHub Issue and implementation map.
- If the task expands into refactoring, architecture, or ambiguous behavior, stop and report back to tech-lead recommending the default path.
- Run the normal validation pack before reporting back.

## Exit

Report back to tech-lead (not QA) with the standard report-back format:

- **Task**: What was assigned
- **Status**: completed | blocked | partial
- **Summary**: What was done
- **Artifacts changed**: Files modified
- **Findings**: Issues found (if any)
- **Next recommendation**: What tech-lead should do next

## Write

- Record progress or new findings using permitted calls in your boundary to `prdtp-agents-functions-cli state *`
- Always use `prdtp-agents-functions-cli git finalize` to close the operational branch and commit the new state.
