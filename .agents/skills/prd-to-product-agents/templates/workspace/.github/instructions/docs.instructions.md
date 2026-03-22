---
applyTo: "docs/**"
---


# Canonical Documentation Rules

Files under `docs/project/` are the **single source of truth** for domain data.

See `docs/project/source-of-truth-map.md` for the full map of every artifact to its location, schema, steward, mutation path, and consumers.

## Principles

- Domain data (stories, epics, decisions, risks, gates, milestones, acceptance criteria) lives here as the **single source of truth**
- GitHub Issues, GitHub Projects, and Pull Requests are the execution layer for task assignment, status, and code review
- If GitHub task metadata disagrees with canonical docs on scope, acceptance, or release intent, `docs/project/*` wins and GitHub must be updated
- YAML files must use spaces (not tabs) for indentation
- Operational YAML state (`handoffs.yaml`, `findings.yaml`, `releases.yaml`) is mutated through `prdtp-agents-functions-cli state *`, not direct line edits
- Every change to canonical docs should be committed to Git for traceability

## Ownership and authority

Primary stewardship does not mean exclusive mutation rights. For coordination files, follow the explicit authority model below.

| File / folder | Primary steward | Allowed mutations |
| ----------------------------- | ------------------------ | ------------------------ |
| `vision.md`, `scope.md` | product-owner | product-owner authors and updates |
| `backlog.yaml` | product-owner | product-owner authors and updates |
| `refined-stories.yaml` | tech-lead | product-owner edits functional fields; software-architect edits tech notes / edge cases; tech-lead edits implementation fields |
| `architecture/overview.md` | software-architect | software-architect authors and updates |
| `decisions/` | software-architect | software-architect authors and updates |
| `quality-gates.yaml` | qa-lead | qa-lead authors and updates |
| `releases.md` | devops-release-engineer | devops-release-engineer authors and updates |
| `releases.yaml` | devops-release-engineer | devops-release-engineer creates / updates release tracker entries via `state release create` / `state release update` |
| `handoffs.yaml` | pm-orchestrator | any agent may create a handoff entry via `state handoff create`; the assignee may claim / complete it via `state handoff update`; pm-orchestrator may reconcile or cancel routing state via `state handoff update` |
| `findings.yaml` | qa-lead | qa-lead, software-architect, tech-lead, and devops-release-engineer may create findings via `state finding create`; findings currently target product-owner, tech-lead, or pm-orchestrator; qa-lead, pm-orchestrator, or the current target role may transition status via `state finding update` |
| `board.md` | pm-orchestrator | pm-orchestrator refreshes the delivery snapshot from GitHub Issues/PRs via `board sync` and keeps operational pointers aligned |
| `context-summary.md` | pm-orchestrator | pm-orchestrator authors and updates |
| `change-log.md` | pm-orchestrator | pm-orchestrator authors and updates |
| `open-questions.md` | product-owner / software-architect | product-owner manages scope questions; software-architect manages technical questions |
| `risks.md` | product-owner | product-owner authors and updates |
| `stakeholders.md` | product-owner | product-owner authors and updates |
| `glossary.md` | product-owner | product-owner authors and updates |
| `acceptance-criteria.md` | product-owner / qa-lead | product-owner authors intent; qa-lead refines testability language |
| `qa/test-strategy.md` | qa-lead | qa-lead authors and updates |
| `ux/journeys.md` | ux-designer | ux-designer authors and updates |
