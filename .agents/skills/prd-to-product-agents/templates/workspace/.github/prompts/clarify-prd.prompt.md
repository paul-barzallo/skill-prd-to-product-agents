---
description: Clarify an incomplete or ambiguous PRD before planning or implementation.
agent: pm-orchestrator
tools:
  - search
  - read
  - edit/editFiles
---


# clarify-prd

## Purpose

Use this prompt when the user provides a partial, ambiguous, or contradictory
PRD and the workspace cannot safely start planning or implementation yet.

## Context scope

- the PRD or partial requirements provided by the user
- `AGENTS.md`
- `docs/project/open-questions.md`

## Write

- `docs/project/open-questions.md`

Do not rewrite core canonical docs from this prompt unless the PRD becomes
clear during the same interaction.

## Required output format

If clarification is required, respond with:

- `What happened`
- `Why blocked`
- `What input is needed`
- `Who should answer`
- `Next safe step`

Each field must be concrete, short, and actionable.

## Process

### 1. Identify ambiguity

Check whether the requirements are clear on:

- objective
- users
- scope
- out of scope
- constraints
- acceptance criteria

### 2. Write unresolved questions

Append or update unresolved items in `docs/project/open-questions.md`.

Use one row per unresolved item and keep the status explicit.

### 3. Decide whether a handoff is needed

- route to `product-owner` for scope or priority clarification,
- for technical unknowns, note them in `open-questions.md` and recommend
  `pm-orchestrator` delegate to `software-architect` in the next planning step,
- route to `pm-orchestrator` for cross-role coordination blockers.

If the work is already represented by an issue, state explicitly in the
report-back that an execute-capable coordinator must mark it `status:blocked`
through the supported runtime path.

### 4. Stop safely

Do not start implementation.
Do not fabricate backlog or stories.
Do not pretend the PRD is good enough when it is not.

## Exit

Present results to the user with:

- **Task**: PRD clarification
- **Status**: clarified | blocked | partial
- **Summary**: Up to 3 sentences of what was found
- **Artifacts changed**: files created or modified
- **Open questions**: count of unresolved items
- **Next recommendation**: suggested next step (e.g., provide missing answers, run `bootstrap-from-prd`)

## Success criteria

- ambiguity is called out explicitly,
- missing answers are written to `open-questions.md`,
- the next safe step is clear,
- implementation remains blocked until the PRD is clarified.

<!-- markdownlint-enable MD013 -->
