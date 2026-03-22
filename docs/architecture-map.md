# Architecture Map

This document gives maintainers a repository-level map of the current project
root, its maintenance tooling, and the documents that control release and
change management.

## System overview

The active root for these docs is the repository root.

This map is about:

1. repository maintenance
2. repository validation
3. repository release hygiene

It is not a map of packaged or generated content.

## Repository map

| Area | Main location | Purpose |
| --- | --- | --- |
| Maintainer docs | `docs/` | repository process, status, audits, and release guidance |
| Decisions | `docs/decisions/` | long-lived repository choices and ADRs |
| Gap tracking | `docs/open-gaps.md`, `docs/known-limitations.md` | explicit maintenance debt and visible limits |
| Operating guidance | `docs/maintainer-runbook.md`, `docs/test-matrix.md` | practical maintenance and validation usage |
| Project maintenance CLI | `cli-tools/skill-dev-cli/` | markdown, smoke, unit, and release-gate checks |
| Rust sources | `cli-tools/` | implementation of repository and packaged CLIs |
| Published repo binaries | `bin/` | project-scope published artifacts |
| GitHub automation | `.github/` | workflows, PR template, and repo automation |

## Project maintenance CLI responsibilities

### `skill-dev-cli`

Owns:

- markdown validation
- smoke tests for bootstrap flow
- release gate orchestration
- project-level repository checks

## Source-of-truth hierarchy

Use this hierarchy when evaluating a claim:

1. Rust implementation under `cli-tools/`
2. Validator logic and workflow automation
3. Maintainer docs under `docs/`
4. Root summaries and guidance

If a higher-priority source contradicts a lower-priority source, the lower one is stale.

## Core flows

### 1. Repository maintenance flow

1. Edit repo code or docs
2. Run project, package, or crate validations as required
3. Pass `test release-gate` for structural or release-sensitive work
4. Publish binaries and release artifacts through GitHub workflows

## Main repository risks maintainers must keep in mind

- scope drift inside repository docs and release rules
- stale claims about repository validation or packaging behavior
- packaging drift between source, bundled binaries, and checksums
- local build outputs accidentally treated as source artifacts
- repository claims documented beyond what validators actually enforce

## Maintainer navigation

- repo process: `docs/README.md`
- current work and blockers: `docs/current-status.md`
- open gaps: `docs/open-gaps.md`
- known limitations: `docs/known-limitations.md`
- decisions: `docs/decisions/README.md`
- runbook: `docs/maintainer-runbook.md`
- test coverage map: `docs/test-matrix.md`
- release review: `docs/repo-release-checklist.md`
- audits and historical assessments: `docs/audits/README.md`
- audit follow-up index: `docs/audits/index.md`
