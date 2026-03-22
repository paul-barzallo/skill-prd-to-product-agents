# Memory Model

The workspace organizes information in three tiers. Each tier has a distinct purpose.

## Tier 1 - Canonical operational state

All live project state resides in versioned files under `docs/project/`.

- Markdown and YAML files are the single source of truth for decisions and operational state.
- If a fact is not in a canonical file, it is not part of the official project state.

### Key operational files

| File | Content |
| --- | --- |
| `backlog.yaml` | Epics and stories |
| `refined-stories.yaml` | Implementation-ready stories |
| `handoffs.yaml` | Phase transitions and routing |
| `findings.yaml` | Bugs, risks, security issues |
| `releases.yaml` | Release state and history |
| `context-summary.md` | Shared project context |
| `scope.md` | What is in or out of scope |
| `decisions/*.md` | Architecture decision records |

## Tier 2 - Audit mirror and reporting store

`.state/project_memory.db` is a passive SQLite mirror managed by runtime infrastructure.

- It records audit and reporting data derived from canonical files and runtime events.
- Agents do not treat SQLite as the source of truth.
- SQLite may lag, be absent, or be rebuilt without changing the canonical state.

Examples of derived tables include audit events, sync runs, release checks, and reporting metrics.

## Tier 3 - Historical context

Git provides historical memory:

- commits
- PRs
- issues
- tags and releases
- blame/history
- diffs

If a decision or change is already reflected in Git, agents should use that information before inventing context.

## Rules

1. If it drives a decision, it lives in Tier 1.
2. Tier 2 mirrors or summarizes operational data; it does not define it.
3. Tier 3 explains how the state evolved over time.
4. Free-form agent chat is never system memory.
5. SQLite absence must not block the operational workflow.
