---
name: runtime-incident-triage
description: Review post-release incidents and turn them into structured findings, environment events and hotfix routes.
user-invocable: true
disable-model-invocation: false
---


# runtime-incident-triage

Review post-release incidents and convert them into structured findings, environment events, and hotfix handoff routes.

## When to use

Use this skill when:

- A production incident is detected (alert, monitoring spike, user report)
- The `post-release-monitoring` prompt detects environmental degradation
- `devops-release-engineer` receives an alert and needs structured triage
- A rollback decision must be documented

## Triage process

### Step 1 -- Capture incident

Gather: source, environment, severity, symptoms, timeline.

### Step 2 -- Create environment event

Run `prdtp-agents-functions-cli state event record` with appropriate `event_type`:

- `health_degraded`, `incident_detected`, `deploy_failed`, `rollback`, `health_restored`

### Step 3 -- Create finding

Run `prdtp-agents-functions-cli state finding create` to record the finding in `docs/project/findings.yaml`.
Route by type: `bug`/`security`/`architecture` -> `tech-lead`; UX/scope -> `product-owner`.

### Step 4 -- Determine action

| Severity | Action |
| -------- | --------------------------- |
| Critical | Immediate rollback + hotfix |
| High | Hotfix handoff + monitor |
| Medium | Regular fix via story flow |
| Low | Backlog as bug |

### Step 5 -- Create handoff

For critical/high: run `prdtp-agents-functions-cli state handoff create` with escalation type to `tech-lead` or `pm-orchestrator`.

### Step 6 -- Report

Structured triage report with timeline, finding ID, action taken.

## Post-incident

1. Log `health_restored` event via `prdtp-agents-functions-cli state event record`
2. Resolve finding and handoff via `prdtp-agents-functions-cli state finding update` and `prdtp-agents-functions-cli state handoff update`
3. Consider post-mortem if systemic
