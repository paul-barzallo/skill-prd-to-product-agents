
# Git Context Guide

Git is the **third tier** of the memory model, after canonical files. (The SQLite audit ledger is passive infrastructure that agents do not interact with.) Agents should consult Git when the work requires understanding history, rationale, or evolution.

## When to consult commits

- **After receiving work**: review recent commits to understand what changed since the last handoff.
- **Reviewing peer work**: check commits by other agents before approving or building on their output.
- **Tracing a bug**: use `git log` and `git blame` to understand when and why a change was introduced.

## When to consult PRs

- **Understanding debate**: PR discussions capture design trade-offs and rejected alternatives.
- **Reviewing implementation**: PR diffs show exactly what changed and who approved it.
- **Finding context for decisions**: ADRs may link to PRs, but the PR itself often has richer discussion.

## When to consult issues

- **Tracing requirements to work**: issues link user requests to implementation.
- **Understanding acceptance criteria evolution**: issue threads show how requirements were refined.
- **Finding related work**: issue labels and milestones group related changes.

## When to consult releases and tags

- **Verifying what shipped**: release tags mark exact commit states that were deployed.
- **Comparing versions**: diff between tags shows everything that changed in a release.
- **Post-release triage**: identify which release introduced an issue by checking tag histories.

## When to use blame and history

- **Understanding authorship**: who wrote code being modified and when.
- **Tracking evolution**: how a file changed over time to understand current state.
- **Identifying risk**: frequently changed files or recent major rewrites may be fragile.

## Rule

> If the answer is already in Git, don't invent context.

Agents should prefer verifiable Git history over assumptions or reconstructed reasoning.
