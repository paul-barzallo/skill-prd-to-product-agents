---
name: release-checks
description: Run the minimum release checklist and write structured release checks before deployment.
user-invocable: true
disable-model-invocation: false
---


# release-checks

Run the minimum release checklist for a given release and record the outcome. This skill gates deployment readiness.

## When to use

Use this skill when:

- A release is approaching deployment and needs formal sign-off
- The `release-readiness` prompt needs to verify all checks pass
- `devops-release-engineer` prepares a deployment package
- `pm-orchestrator` verifies a release gate before go-live

## Checklist

### Functional checks (qa-lead)

| Check | Pass criteria |
| ---------------------- | ----------------------------------------------------- |
| `all-stories-done` | All stories in release scope are `done` or `released` |
| `acceptance-validated` | All acceptance criteria validated |
| `no-critical-findings` | No open critical/high findings |
| `regression-pass` | QA regression suite passed |

### Technical checks (tech-lead / devops)

| Check | Pass criteria |
| -------------------- | --------------------------------------- |
| `build-success` | Latest build succeeded |
| `security-scan` | No open high/critical security findings |
| `dependency-audit` | No known vulnerable dependencies |
| `migration-verified` | DB migrations tested in staging |

### Operational checks (devops-release-engineer)

| Check | Pass criteria |
| -------------------- | -------------------------------- |
| `env-healthy` | Target environment is `healthy` |
| `rollback-plan` | Rollback documented and tested |
| `monitoring-config` | Alerts and dashboards configured |
| `deployment-runbook` | Step-by-step procedure available |
| `audit-spool-empty` | `.state/audit-spool/` contains no pending files |

### Business checks (product-owner / pm-orchestrator)

| Check | Pass criteria |
| ----------------- | -------------------------------- |
| `scope-approved` | Scope confirmed by product-owner |
| `client-sign-off` | UAT approved (if applicable) |
| `release-notes` | Release notes reviewed |

## Process

1. Identify release by `release_ref`
2. Run each check against canonical YAML/MD files
3. Record check results in `docs/project/release/readiness-checklist.md`
4. Aggregate: all pass = PASS, any fail = FAIL, any blocked = BLOCKED
5. Route: PASS -> devops, FAIL -> tech-lead, BLOCKED -> pm-orchestrator
