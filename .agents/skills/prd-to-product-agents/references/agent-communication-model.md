
# Agent Communication Model

## Principle

Agents communicate through shared state, not long conversational
chains. State lives in versioned files. SQLite is a passive audit ledger
managed by infrastructure - agents do not interact with it directly.

## Allowed channels

### Canonical artifacts (versioned files)

Used for durable intent, design and operational state:

- vision, scope, backlog, refined stories
- acceptance criteria, architecture docs, release docs
- **handoffs** (`docs/project/handoffs.yaml`)
- **findings** (`docs/project/findings.yaml`)
- **releases** (`docs/project/releases.yaml`)
- **context summary** (`docs/project/context-summary.md`)

Operational YAML files are canonical state, but their mutations are expected to
go through the generated workspace runtime state commands rather than direct
freehand edits.

### SQLite audit tables (passive, infrastructure-managed)

Used for evidence and traceability, **not** decision state:

- `agent_activity_log` - who did what, when
- `gate_checks` - quality gate evidence
- `release_checks` - release readiness evidence
- `security_checks` - security audit evidence
- `environment_events` - deployment and incident records
- `sync_runs` - sync history and drift detection

### Git history (contextual memory)

Used for understanding evolution and intent:

- commits, diffs - what changed and when
- PRs - discussion and review rationale
- issues - requirements and bug reports
- tags and releases - what shipped

Git history is context, not live operational state. If Git capability is
disabled, the equivalent local evidence lives under `.state/local-history/`.

## Handoffs

Use handoffs for phase changes, escalation, rework and approval.
Tracked in `docs/project/handoffs.yaml`.

See `agent-flow.md` for detailed routing tables and reason codes.

## Findings

Use findings for bugs, risks, ambiguity, security issues, UX
issues and architecture issues. Tracked in
`docs/project/findings.yaml`.

Releases and environment events follow the same rule: canonical state is kept in
`docs/project/releases.yaml` and runtime commands append evidence around it.

## QA triage

- product-owner receives functional/scope/UX findings
- tech-lead receives technical/implementation/architecture/security
  findings

See `agent-flow.md` for the complete triage flow diagram.

## State precedence

When conflicts arise between channels:

1. Versioned files are authoritative (operational truth)
2. Git history provides context and rationale
3. SQLite is a passive audit ledger managed by infrastructure - agents do not query it

If SQLite is unavailable, spool files and local-history evidence can preserve
traceability, but they still do not outrank canonical versioned files.
