
# Agent Communication Flow Reference

This document describes the complete communication flow between agents
in the workspace, including handoff routes, escalation paths, and
triage rules.

## Architecture principles

The workspace separates three concerns:

1. **Agents** in `.github/agents/`
2. **Prompt workflows** in `.github/prompts/`
3. **Memory** split into canonical docs (operational truth) and
   SQLite audit ledger (passive, infrastructure-managed telemetry)

The model uses exactly **9 base agents**. Keeping the set small
reduces context transfer overhead, contradictory instructions,
handoff sprawl and role ambiguity.

### Critical authority rules

- software-architect designs, but does not command developers
- tech-lead is the only technical authority over backend/frontend
  developers
- qa-lead triages only to product-owner or tech-lead
- security is a workflow, not an agent
- the system does not stop at go-live; post-release monitoring can
  reopen the cycle

## Agent hierarchy

```txt
pm-orchestrator (coordinator)
- product-owner
- tech-lead (coordinator)
  - backend-developer
  - frontend-developer
- qa-lead
- devops-release-engineer

software-architect (advisory -- no direct command over developers)
ux-designer (advisory -- feeds into product-owner and frontend-developer)
```

## Handoff routes

### Normal flow (happy path)

```txt
product-owner -> pm-orchestrator -> tech-lead -> {backend,frontend}-developer -> qa-lead -> devops-release-engineer
```

| Phase | From | To | Handoff type | Reason code |
| ------------------ | -------------------------- | -------------------------- | ------------ | ----------------- |
| PRD complete | product-owner | pm-orchestrator | normal | ready_for_review |
| Architecture ready | software-architect | pm-orchestrator | normal | ready_for_review |
| Stories refined | tech-lead | pm-orchestrator | normal | ready_for_review |
| Story assigned | tech-lead | backend/frontend-developer | normal | new_work |
| Dev complete | backend/frontend-developer | tech-lead | normal | ready_for_review |
| Code reviewed | tech-lead | qa-lead | normal | ready_for_review |
| QA passed | qa-lead | devops-release-engineer | normal | ready_for_release |
| Deployed | devops-release-engineer | pm-orchestrator | normal | ready_for_review |

### Rework flow

| Trigger | From | To | Handoff type | Reason code |
| ---------------- | ------------- | --------------- | ------------ | --------------- |
| QA finds bug | qa-lead | tech-lead | rework | needs_rework |
| Code review fail | tech-lead | developer | rework | needs_rework |
| Client rejects | product-owner | tech-lead | rework | client_rejected |
| Scope change | product-owner | pm-orchestrator | escalation | scope_change |

### Escalation flow

| Trigger | From | To | Handoff type | Reason code |
| ----------------- | --------- | --------------- | ------------ | ----------------- |
| Story blocked | developer | tech-lead | escalation | blocked |
| Technical risk | tech-lead | pm-orchestrator | escalation | technical_risk |
| Environment issue | devops | pm-orchestrator | escalation | environment_issue |
| Security finding | qa-lead | tech-lead | escalation | technical_risk |

## QA triage rules

`qa-lead` never routes directly to developers. All findings go through one of two paths:

| Finding type | Route to | Examples |
| ---------------------------------- | ------------- | ------------------------------------------ |
| Functional / scope / UX | product-owner | Missing feature, wrong behavior, poor UX |
| Technical / impl / arch / security | tech-lead | Code bug, performance issue, vulnerability |

## Finding flow

```text
qa-lead creates finding -> routes to {product-owner, tech-lead}
  - product-owner: evaluates scope impact -> may create rework handoff
  - tech-lead: evaluates technical fix -> assigns to developer
```

## Gate check flow

```text
pm-orchestrator initiates gate check
  -> qa-lead runs functional checks
  -> tech-lead runs technical checks
  -> devops runs operational checks
  -> product-owner confirms business checks
  -> pm-orchestrator aggregates results
    - ALL PASS -> release proceeds
    - ANY FAIL -> handoff to responsible role
    - ANY BLOCKED -> pm-orchestrator resolves
```

## Incident flow (post-release)

```text
monitoring/alert detected
  -> devops-release-engineer runs runtime-incident-triage
    -> creates environment_event
    -> creates finding (routed per triage rules)
    -> creates handoff if critical
    -> if rollback needed: escalation to pm-orchestrator
```

## Context injection flow

```text
Layer 0: bootstrap -> all agents get personality + behavior
Layer 1: product-owner runs /enrich-agents-from-prd -> all agents get Project Context
Layer 2: software-architect runs /enrich-agents-from-architecture -> 6 agents get Technical Context
Layer 3: tech-lead runs /enrich-agents-from-implementation -> 2 developers get Implementation Context
```

## Communication principles

1. **Structured over free-form**: Use handoffs, findings, gate_checks -- not chat messages
2. **Files are the message bus**: Coordination state lives in `handoffs.yaml`, `findings.yaml`, `releases.yaml` -- SQLite is a passive audit ledger managed by infrastructure
3. **Canonical docs are truth**: Files under `docs/project/` are the single source of truth
4. **One authority chain**: developers -> tech-lead -> pm-orchestrator (never software-architect -> developers)
5. **QA is independent**: qa-lead reports findings but does not assign work
6. **Security is a workflow**: No security agent -- security checks are distributed across qa-lead, tech-lead, devops
