---
description: Run the reusable security gate before release.
agent: qa-lead
tools:
  - search
  - read
  - execute
  - edit/editFiles
---

# security-check

## Purpose

Check secrets, auth/authz, data exposure, dependencies, and release risk notes; record findings and escalation when needed.

## Context scope

- source code for secrets, credentials, and unsafe patterns
- `docs/project/architecture/overview.md` for auth/authz design
- `docs/project/risks.md` for known security risks
- `docs/project/findings.yaml` for existing security findings

## Process

### 1. Run automated scans

Execute available security tooling in the workspace:

```powershell
# Search for hardcoded secrets (common patterns)
Get-ChildItem -Recurse -Include *.ts,*.js,*.py,*.env,*.yaml,*.json |
  Select-String -Pattern '(api[_-]?key|secret|password|token|credential)\s*[:=]' -CaseSensitive:$false

# Check for .env files that should not be committed
Get-ChildItem -Recurse -Filter '.env*' | Where-Object { $_.Name -ne '.env.example' }

# Check dependency audit if package manager is present
if (Test-Path package.json) { npm audit --json 2>$null }
if (Test-Path requirements.txt) { pip audit --format json 2>$null }
```

### 2. Manual review checklist

Evaluate each area and record results:

| Check name | What to verify |
| --- | --- |
| `no_hardcoded_secrets` | No API keys, tokens, or passwords in source code |
| `auth_implemented` | Authentication exists on all protected endpoints |
| `authz_enforced` | Role-based access control matches architecture docs |
| `data_exposure` | No PII or sensitive data in logs, responses, or error messages |
| `dependency_vulnerabilities` | No critical/high CVEs in dependencies |
| `env_files_excluded` | `.env` files are in `.gitignore`, only `.env.example` tracked |
| `input_validation` | User inputs sanitized, SQL injection and XSS mitigated |

### 3. Route findings

| Result | Action |
| --- | --- |
| `fail` | Create finding via state ops (type `security`, severity from check) targeting `tech-lead` |
| `warning` | Create finding via `prdtp-agents-functions-cli --workspace . state finding create` (type `security`, severity `medium`) targeting `tech-lead` |
| `pass` | No finding needed |

For each failing or warning check, run:

```shell
prdtp-agents-functions-cli --workspace . state finding create \
  --source-role  qa-lead \
  --target-role  tech-lead \
  --finding-type security \
  --severity     high \
  --entity       "US-003" \
  --title        "Login flow fails on expired tokens"
```

Critical or high security findings also get an escalation handoff:

```shell
prdtp-agents-functions-cli --workspace . state handoff create \
  --from-role     qa-lead \
  --to-role       tech-lead \
  --handoff-type  escalation \
  --entity        "finding/{finding_id}" \
  --reason        technical_risk \
  --details       "Critical security finding requires immediate remediation"
```

## Write

- Run `prdtp-agents-functions-cli --workspace . state finding create` for each failing or warning check
- Run `prdtp-agents-functions-cli --workspace . state handoff create` for escalation of critical/high issues
- Do not modify source code directly from this prompt; route remediation to `tech-lead`
- Do not write YAML directly to `findings.yaml` or `handoffs.yaml`

## Exit

Report back to `pm-orchestrator` with:

- **Task**: security check
- **Status**: done | blocked | partial
- **Summary**: Up to 3 sentences of outcome
- **Artifacts changed**: files created or modified
- **Findings**: issues discovered, if any
- **Next recommendation**: suggested next delegation or action
