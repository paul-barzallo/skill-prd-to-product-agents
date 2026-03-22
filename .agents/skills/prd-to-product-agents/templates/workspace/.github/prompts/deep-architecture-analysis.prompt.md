---
description: Run the deep reasoning path for architecture tradeoffs, systemic redesigns, or ADR-heavy analysis.
agent: software-architect
tools:
  - search
  - read
  - execute
  - edit/editFiles
model:
  - Claude Opus 4.5
  - GPT-4.1
---


# deep-architecture-analysis

Use this prompt when the architecture problem is systemic, ambiguous, or spans multiple bounded contexts.

## Context scope

- `docs/project/architecture/overview.md`
- `docs/project/architecture/` (relevant component docs)
- `docs/project/decisions/` (relevant ADRs)

## Typical fit

- ADRs with major tradeoffs
- multi-service or multi-layer impact
- failure-mode and resilience analysis
- refactor planning that changes architecture boundaries

## Guardrails

- Keep the output decision-oriented, not implementation-heavy.
- If the result changes execution, report back to `pm-orchestrator` recommending `tech-lead` delegation.

## Exit

Report back to `pm-orchestrator` with:

- **Task**: architecture analysis requested
- **Status**: done | blocked | partial
- **Summary**: Up to 3 sentences of outcome
- **Artifacts changed**: files created or modified
- **Findings**: issues discovered, if any
- **Next recommendation**: suggested next delegation or action

## Write

- Record progress or new findings using permitted calls in your boundary to `prdtp-agents-functions-cli state *`
- Always use `prdtp-agents-functions-cli git finalize` to close the operational branch and commit the new state.
