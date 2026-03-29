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

## Scope boundaries

Use these boundaries before editing or documenting any surface in this
repository.

| Scope | Source location | Purpose | Must not depend on |
| --- | --- | --- | --- |
| Project repo | root `docs/`, `.github/`, `cli-tools/skill-dev-cli/`, `cli-tools/project-memory-cli/`, `bin/` | develop, validate, and release the skill repository | deployed workspace runtime rules as if they were repository policy |
| Skill package | `.agents/skills/prd-to-product-agents/` | ship the bootstrap CLI, packaged references, and workspace template source | repository-maintenance guidance except where packaging is the explicit subject |
| Deployed workspace | `.agents/skills/prd-to-product-agents/templates/workspace/` | define the self-contained runtime surface copied into client projects | repository-maintenance tooling or knowledge of this source repository after bootstrap |

The repository owns the source for all three surfaces, but the packaged skill
and deployed workspace must remain understandable and operable within their own
scope contracts.

## Repository map

| Area | Main location | Purpose |
| --- | --- | --- |
| Maintainer docs | `docs/` | repository process, status, stable review outcomes, and release guidance |
| Decisions | `docs/decisions/` | long-lived repository choices and ADRs |
| Gap tracking | `docs/open-gaps.md`, `docs/known-limitations.md` | explicit maintenance debt and visible limits |
| Operating guidance | `docs/maintainer-runbook.md`, `docs/test-matrix.md` | practical maintenance and validation usage |
| Project maintenance CLI | `cli-tools/skill-dev-cli/` | markdown, smoke, unit, and release-gate checks |
| Project memory CLI | `cli-tools/project-memory-cli/` | local project indexing, traceability, validation, and incremental retrieval |
| Rust sources | `cli-tools/` | implementation of repository and packaged CLIs |
| Published repo binaries | `bin/` | project-scope published artifacts |
| GitHub automation | `.github/` | workflows, PR template, and repo automation |

The skill package and deployed workspace are maintained from this repository,
but their own operational contracts live under
`.agents/skills/prd-to-product-agents/` and must not be described as part of
repository-only behavior unless the current repository task is specifically
about packaging or bootstrap.

## Project maintenance CLI responsibilities

### `skill-dev-cli`

Owns:

- markdown validation
- smoke tests for bootstrap flow
- release gate orchestration
- project-level repository checks

### `project-memory-cli`

Owns:

- local repository ingestion and fingerprinted snapshots
- deterministic JSON retrieval for indexed files
- minimal requirement-to-artifact traceability
- impact and validation reporting over the persisted snapshot

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
- review outcomes and follow-up: `docs/current-status.md`, `docs/open-gaps.md`, `docs/known-limitations.md`
