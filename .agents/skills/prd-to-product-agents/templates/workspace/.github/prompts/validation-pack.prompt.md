---
description: Run the shared validation pack before routing work forward.
agent: qa-lead
tools:
  - search
  - read
  - execute
---

# validation-pack

## Purpose

Run the shared validation pass on the current workspace state and route findings or rework when checks fail.

## Context scope

- canonical docs under `docs/project/` such as `backlog.yaml`, `refined-stories.yaml`, and `quality-gates.yaml`
- `docs/project/findings.yaml` for existing findings
- `docs/project/handoffs.yaml` for pending handoffs

## Write

- findings to `docs/project/findings.yaml` via `prdtp-agents-functions-cli state finding create` when validation fails
- handoffs to `docs/project/handoffs.yaml` via `prdtp-agents-functions-cli state handoff create` when rework is needed
- structured validation outcome only

## Exit

Report back to `pm-orchestrator` with:

- **Task**: validation pack execution
- **Status**: done | blocked | partial
- **Summary**: Up to 3 sentences of outcome
- **Artifacts changed**: files created or modified
- **Findings**: issues discovered, if any
- **Next recommendation**: suggested next delegation or action
