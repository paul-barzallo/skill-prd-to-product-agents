---
description: Run the deep reasoning path for incidents, rollback analysis, and complex release debugging.
agent: devops-release-engineer
tools:
  - search
  - read
  - execute
  - edit/editFiles
model:
  - Claude Opus 4.5
  - GPT-4.1
---


# release-incident-analysis

Use this prompt when a release or environment incident requires deeper reasoning than the default release workflow.

## Context scope

- `docs/project/releases.yaml`
- `docs/project/releases.md`
- `docs/project/findings.yaml`
- environment/monitoring evidence (external)

## Typical fit

- rollback decision analysis
- multi-signal production incidents
- environment drift with unclear root cause
- release gates blocked by interacting failures

## Guardrails

- Treat safety and rollback clarity as first-class outputs.
- If code changes are required, report back to `pm-orchestrator`; route a lateral finding to `tech-lead` for environment issues.

## Exit

Report back to `pm-orchestrator` with:

- **Task**: incident analysis requested
- **Status**: done | blocked | partial
- **Summary**: Up to 3 sentences of outcome
- **Artifacts changed**: files created or modified
- **Findings**: issues discovered, if any
- **Next recommendation**: suggested next delegation or action

## Write

- Record progress or new findings using permitted calls in your boundary to `prdtp-agents-functions-cli state *`
- Always use `prdtp-agents-functions-cli git finalize` to close the operational branch and commit the new state.
