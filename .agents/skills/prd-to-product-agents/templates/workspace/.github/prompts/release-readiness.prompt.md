---
description: Verify whether the release is ready for deployment.
agent: devops-release-engineer
tools:
  - search
  - read
  - execute
  - edit/editFiles
---

# release-readiness

## Purpose

Check quality, security, environment, rollback, observability, and approvals before cutting a release.

## Context scope

- `docs/project/releases.yaml` for operational release status and target releases
- `docs/project/releases.md` for human-readable release notes and rollback notes
- `docs/project/quality-gates.yaml` for gate definitions
- `docs/project/backlog.yaml` and `docs/project/refined-stories.yaml` for scope completeness
- `docs/project/findings.yaml` for open findings
- `docs/project/release/readiness-checklist.md` for checklist template
- `.github/github-governance.yaml` for local governance readiness

## Process

### 0. Governance precondition

Before evaluating release gates, verify governance readiness:

1. Read `.github/github-governance.yaml` and check `readiness.status`.
2. If status is `template` or `bootstrapped`, stop. Governance is still pending local configuration.
3. Scan `github-governance.yaml` and `.github/CODEOWNERS` for any remaining `REPLACE_ME` or `@team-` placeholders. If found, stop with the same routing.
4. Minimum required readiness: `production-ready`. If the workspace is only `configured`, stop and route the owner to remote governance hardening plus `prdtp-agents-functions-cli validate readiness`.

### 1. Gather release state

Read canonical files for operational state:

- Release status: parse `docs/project/releases.yaml` to identify releases in status `ready`
- Open findings: parse `docs/project/findings.yaml` and filter for `severity: critical` or `severity: high` with `status` not in `[resolved, wont_fix]`
- Scope completeness: parse `docs/project/backlog.yaml` and `docs/project/refined-stories.yaml` to verify all items assigned to the target release are marked done
- Quality gates: read `docs/project/quality-gates.yaml` for gate definitions
- Release notes: read `docs/project/releases.md` only for narrative notes such as rollback instructions or deployment notes

### 2. Run checklist

Evaluate each required gate:

| Check name | Pass condition | Source |
| --- | --- | --- |
| `scope_complete` | All items in canonical backlog for this release are marked done | `backlog.yaml`, `refined-stories.yaml` |
| `no_critical_findings` | Zero open findings with severity `critical` | `findings.yaml` |
| `security_gate` | Security check prompt completed with no `fail` results | `release/readiness-checklist.md` |
| `staging_healthy` | Environment is healthy | deployment notes / environment evidence |
| `client_approved` | Client review has result `approved` | `releases.md` or review notes |
| `rollback_documented` | Release notes include rollback procedure | `releases.md` |

### 3. Decision tree

```text
IF all checks pass -> update release status from 'ready' to 'approved'
IF any check = 'fail'
  IF severity = 'critical' -> escalation handoff to tech-lead
  IF severity = 'high' -> rework handoff to tech-lead
IF any check = 'blocked'
  -> escalation handoff to pm-orchestrator with reason 'blocked'
```

### 4. Record outcome

If all checks pass, transition the release from `ready` to `approved`:

```shell
prdtp-agents-functions-cli state release update \
  --release-ref  "{release_ref}" \
  --new-status   approved \
  --agent-role   devops-release-engineer
```

If the decision creates handoffs, use supported snake_case reasons:

```shell
prdtp-agents-functions-cli state handoff create \
  --from-role     devops-release-engineer \
  --to-role       tech-lead \
  --handoff-type  escalation \
  --entity        "release/{release_ref}" \
  --reason        technical_risk \
  --details       "{reason for failure}"
```

Use only supported handoff types: `normal`, `escalation`, `rework`, `approval`.
Use only supported reasons from the handoff contract, including `blocked`, `ready_for_release`, `technical_risk`, and `environment_issue`.

## Write

- Run `prdtp-agents-functions-cli state handoff create` for escalation or rework when checks fail
- Run `prdtp-agents-functions-cli state finding create` for newly discovered blockers
- Run `prdtp-agents-functions-cli state release update` to transition status from `ready` to `approved` when all checks pass
- Update `docs/project/releases.md` only for human-readable notes, evidence, or rollback instructions
- Do not write YAML directly to `handoffs.yaml`, `findings.yaml`, or `releases.yaml`

## Exit

Report back to `pm-orchestrator` with:

- **Task**: release readiness verification
- **Status**: done | blocked | partial
- **Summary**: Up to 3 sentences of outcome
- **Artifacts changed**: files created or modified
- **Findings**: issues discovered, if any
- **Next recommendation**: suggested next delegation or action
