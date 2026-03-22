
# GitHub Delivery Governance

This workspace uses a split operating model:

- `docs/project/*` is the canonical product and coordination memory.
- GitHub Issues, Projects, branches, commits, and PRs are the execution layer.

If GitHub metadata and canonical docs disagree on scope, acceptance, or
release intent, `docs/project/*` wins and GitHub must be updated.

The explicit governance contract lives in `.github/github-governance.yaml`.

## Governance phases

### 1. Local governance setup

Local setup means:

- configure `.github/github-governance.yaml` locally,
- render `CODEOWNERS`,
- move readiness from `bootstrapped` to `configured`.

Bootstrap only creates the local skeleton. Use
`prdtp-agents-functions-cli governance configure` to complete local governance
before treating the workspace as configured.

### 2. Remote governance provisioning

Remote provisioning means:

- create labels,
- create or connect the GitHub Project,
- apply branch protection when permissions allow it,
- move from `configured` toward `production-ready`.

Do not claim remote provisioning if any of these are missing:

- `gh`,
- `gh auth login`,
- real repository owner/name,
- real reviewer identities,
- sufficient GitHub permissions.

## Branch model

| Purpose | Branch |
| --- | --- |
| Production-ready history | `main` |
| Integration branch for daily work | `develop` |
| Task branch | `<role>/<issue-id>-slug` |

Accepted task prefixes:

- `backend/`
- `frontend/`
- `qa/`
- `arch/`
- `ux/`
- `product/`
- `ops/`
- `techlead/`

## Commit and PR conventions

Commit format:

- `feat(frontend): GH-123 checkout form`
- `fix(backend): GH-456 validate policy payload`
- `chore(ops): GH-789 update ci env`

PR labels are mandatory:

| Group | Allowed labels |
| --- | --- |
| Role | `role:backend`, `role:frontend`, `role:qa`, `role:arch`, `role:ux`, `role:product`, `role:ops`, `role:techlead` |
| Kind | `kind:feature`, `kind:bug`, `kind:chore`, `kind:docs` |
| Priority | `priority:p0`, `priority:p1`, `priority:p2`, `priority:p3` |

PR description must include:

- functional summary
- linked issue
- base and head branches
- canonical docs touched
- validations run
- risks
- rollback
- handoffs/findings status

## GitHub Project contract

Use one GitHub Project as the main delivery board.

Minimum Project fields:

| Field | Values |
| --- | --- |
| `Status` | `Backlog`, `Ready`, `In Progress`, `In Review`, `Blocked`, `Done` |
| `Priority` | `P0`, `P1`, `P2`, `P3` |
| `Role` | `backend`, `frontend`, `qa`, `arch`, `ux`, `product`, `ops`, `techlead` |
| `Type` | `feature`, `bug`, `chore`, `docs`, `investigation` |
| `Criticality` | `critical`, `normal` |
| `Release` | free text or milestone |

Minimum views:

- Backlog
- Ready / In Progress
- Blocked
- Critical
- By Role
- Done

## Review and approval model

- Domain review follows `CODEOWNERS`.
- `devops-release-engineer` is the final approval gate before merge.
- Merge requires green checks, resolved conversations, and the correct label
  set.

## Branch protection expected

Configure repository protection to require:

- no direct pushes to `main` or `develop`
- PRs for all changes
- required status checks
- required CODEOWNERS review
- final approval by `devops-release-engineer`
- resolved conversations before merge

## Readiness states

`github-governance.yaml` uses these readiness states:

- `template`
- `bootstrapped`
- `configured`
- `production-ready`

Use `prdtp-agents-functions-cli validate governance` and
`prdtp-agents-functions-cli validate readiness` to see what is still missing
for the next state.

## Visibility surfaces

- `docs/project/board.md`: detailed operational snapshot.
- `docs/project/management-dashboard.md`: executive summary for readiness,
  risk, release state, backlog, blockers, and pending decisions.

Refresh those views with:

- `prdtp-agents-functions-cli board sync`
- `prdtp-agents-functions-cli report dashboard`

If `gh` is unavailable or unauthenticated, the board stays on the last valid
snapshot and the management dashboard falls back to local-only visibility.

<!-- markdownlint-enable MD013 -->
