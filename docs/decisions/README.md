# Decisions

This directory stores repository-level architecture and maintenance decisions.

## Purpose

Use ADRs here when a repository choice should not have to be rediscovered,
re-argued, or reconstructed from code and chat history.

## What belongs here

- repository documentation structure
- repository release and validation policy
- repository packaging and binary publication rules
- maintainer process decisions with long-lived consequences

## What does not belong here

- temporary notes or exploratory thoughts
- audit reports themselves
- generated content
- decisions about other scopes unless the current repository task explicitly owns them

## Naming convention

- `ADR-0001-short-title.md`
- `ADR-0002-short-title.md`

Keep titles short and stable.

## Minimum ADR structure

- status
- context
- decision
- consequences
- related docs

## Current ADRs

- `ADR-0001-repository-docs-live-under-docs.md`
- `ADR-0002-release-gate-blocks-repository-release.md`
