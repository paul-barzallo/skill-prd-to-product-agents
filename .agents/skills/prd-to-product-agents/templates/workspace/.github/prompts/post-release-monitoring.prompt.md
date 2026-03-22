---
description: Run post-release monitoring and reopen the flow if needed.
agent: devops-release-engineer
tools:
  - search
  - read
  - execute
  - edit/editFiles
---


# post-release-monitoring

## Purpose

Detect post-release incidents and route hotfixes or planning impacts back into the workflow.

## Context scope

- `docs/project/releases.yaml` for current operational release status
- `docs/project/releases.md` for current release metadata and narrative notes
- `docs/project/findings.yaml` for existing findings
- monitoring/logging systems output (external)

## Process

### 1. Check current environment state

Read `docs/project/releases.yaml` to identify the current `approved`, `deployed`, or `rolled_back` release.
Read `docs/project/releases.md` only for human-readable notes and deployment context.
Review `docs/project/findings.yaml` for any existing post-release issues.

### 2. Incident decision tree

```
IF event_type = 'incident_detected' OR event_type = 'health_degraded':
  IF severity = 'critical':
    -> Run state finding create (type 'bug', severity 'critical', target 'tech-lead')
    -> Run state handoff create (to tech-lead, reason 'environment_issue', type 'escalation')
    -> Consider immediate rollback
  IF severity = 'high':
    -> Run state finding create (type 'bug', severity 'high', target 'tech-lead')
    -> Run state handoff create (to tech-lead, reason 'environment_issue')
  IF severity = 'medium' or 'low':
    -> Run state finding create (type 'bug', severity as detected, target 'tech-lead')

IF event_type = 'rollback':
  -> Run state handoff create (to pm-orchestrator, reason 'environment_issue')
  -> Run state release update (move release status to 'rolled_back')

IF event_type = 'health_restored':
  -> Run state finding update to transition related findings to 'resolved' if applicable
```

### 3. Route hotfixes back into flow

For critical incidents requiring code changes:

1. Create a finding via prdtp-agents-functions-cli:

```shell
prdtp-agents-functions-cli state finding create \
  --source-role  devops-release-engineer \
  --target-role  tech-lead \
  --finding-type bug \
  --severity     critical \
  --entity       "release/{release_ref}" \
  --title        "HOTFIX: {description}"
```

2. Create an escalation handoff:

```shell
prdtp-agents-functions-cli state handoff create \
  --from-role     devops-release-engineer \
  --to-role       tech-lead \
  --handoff-type  escalation \
  --entity        "finding/{finding_id}" \
  --reason        environment_issue \
  --details       "{incident_details}"
```

3. Document the hotfix requirement in canonical docs (`docs/project/backlog.yaml` or `docs/project/refined-stories.yaml`).

## Write

- Run `prdtp-agents-functions-cli state finding create` for incidents and hotfix needs
- Run `prdtp-agents-functions-cli state handoff create` for escalation routing
- Run `prdtp-agents-functions-cli state release update` if rollback is required
- Run `prdtp-agents-functions-cli state finding update` to transition existing findings
- Update `docs/project/releases.md` only for narrative notes, incident summaries, or rollback details
- Update canonical docs for hotfix stories
- Do NOT write YAML directly to `findings.yaml`, `handoffs.yaml`, or `releases.yaml` - always use `prdtp-agents-functions-cli state *`

## Exit

Report back to `pm-orchestrator` with:

- **Task**: post-release monitoring
- **Status**: done | blocked | partial
- **Summary**: Up to 3 sentences of outcome
- **Artifacts changed**: files created or modified
- **Findings**: issues discovered, if any
- **Next recommendation**: suggested next delegation or action
