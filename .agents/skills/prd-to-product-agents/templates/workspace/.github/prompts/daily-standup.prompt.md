---
description: Generate a daily standup summary from project state files.
agent: pm-orchestrator
tools:
  - search
  - read
  - execute
---


# daily-standup

## Purpose

Generate a structured daily standup summary by reading
operational state files and Git history.

## Context scope

- `docs/project/handoffs.yaml`
- `docs/project/findings.yaml`
- `docs/project/releases.yaml`
- `docs/project/refined-stories.yaml`
- `docs/project/backlog.yaml`
- Git commit history (last 24 hours)

## Process

### 1. Read operational state

Read these canonical files for current project state:

- `docs/project/handoffs.yaml` -- pending handoffs
- `docs/project/findings.yaml` -- open findings by severity
- `docs/project/releases.yaml` -- current release status
- `docs/project/refined-stories.yaml` -- active work items
- `docs/project/backlog.yaml` -- epic and story status

### 2. Check Git for recent changes

Review recent commits to understand what changed:

- Commits in the last 24 hours
- Open PRs and their status
- Recently merged work

### 3. Format standup report

```markdown
## Daily Standup -- {date}

### Handoff Queue ({n} pending)

- {from} -> {to}: {type} -- {reason} ({created})

### Active Work (from canonical docs)

- {item}: {title} ({owner}) -- {status}

### Open Findings

- Critical: {n}, High: {n}, Medium: {n}, Low: {n}

### Recent Changes (from Git)

- {commit_summary}

### Blockers

- {any handoffs with status = pending and high urgency}
- {any findings with severity = critical}
```

### 4. Identify actions needed

For each pending handoff, suggest which agent should act next.
For critical findings, flag them as requiring immediate attention.

## Exit

Present the standup report to the user with:

- **Task**: daily standup summary
- **Status**: done
- **Summary**: the formatted standup report above
- **Blockers**: count of critical findings and urgent handoffs
- **Next recommendation**: suggested delegation or action based on findings

## Write

- Record progress or new findings using permitted calls in your boundary to `prdtp-agents-functions-cli --workspace . state *`
- Always use `prdtp-agents-functions-cli --workspace . git finalize` to close the operational branch and commit the new state.
