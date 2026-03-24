# Current Status

Use this file as the maintainer handoff snapshot for the repository itself.
Update it when priorities, blockers, or key repository risks change.

## Snapshot

- Repository maturity: v1 presentable / hardened prototype
- Current publication goal: keep repository maintenance coherent while incubating `project-memory-cli` as a repository-side capability
- Validation posture: repository CI, multi-OS release-gate, and pre-commit/pre-push hooks now have a single GitHub-aligned local validation command; `repo-validation` now also covers `project-memory-cli`, while published Unix binaries still require executable-bit discipline

## What is already in place

- root `README.md`, `CONTRIBUTING.md`, and `SECURITY.md`
- repo-level `.gitignore`, `.editorconfig`, PR template, and validation workflow
- explicit `AGENTS.md` rules for repository maintenance, packaging, and minimum validation
- release checklist in `docs/repo-release-checklist.md`
- `project-memory-cli` MVP foundation with ingest, query, trace, impact, and validate commands backed by `.project-memory/snapshot.json`

## Active strengths

- stronger separation between repository docs and other packaged surfaces
- canonical validation commands already exist in Rust CLIs
- packaged binary model is explicit
- claims about bootstrap and readiness are more disciplined than in earlier drafts
- the repository now documents repo, skill-package, and deployed-workspace scopes as separate contracts
- template and runtime contract tests now better tolerate explicit skill-root injection instead of assuming only the current repository layout

## Current gaps to close next

1. structured audit archive and remediation tracking still need consistent upkeep
2. the legacy empty `docs/project/` path in bootstrapped workspaces still needs cleanup if still present in downstream environments
3. repository support and escalation flow is still minimal
4. release workflow and documentation drift still need periodic review, especially around path filters, published binary permissions, and multi-OS gate scope
5. decide whether `project-memory-cli` should stay as repository-only tooling or eventually become part of a broader product workflow story

## Current blockers or risks

- the repo can still drift if maintainers bypass hooks or skip the GitHub-aligned local validation chain before pushing
- local build outputs under `cli-tools/*/target/` remain easy to confuse with shipped artifacts if hygiene slips
- security and governance expectations can still be overstated unless limitations are kept visible in docs
- local validation still only simulates the current platform; cross-platform parity remains a GitHub responsibility even after pre-merge multi-OS gating

## Recommended next actions

1. keep `docs/audits/index.md` updated as audits land or close
2. keep `CHANGELOG.md` updated when repository contracts or release behavior change
3. remove the empty legacy `docs/project/` path where still present in bootstrapped workspaces
4. review whether release workflows need a documentation sync check and whether published binary permissions and path filters remain stable
5. decide whether `project-memory-cli` needs release or packaging policy beyond the current repository-only scope

## Last repository housekeeping changes

- added repo publication hygiene files and validation automation
- added maintainer-facing contribution and security guidance
- added architecture map, current status, and audits area to reduce context loss
- added open gaps, known limitations, and initial repository ADRs
- added issue templates, maintainer runbook, and repository test matrix
- added audit index and repository changelog
- added `project-memory-cli` foundation, its planning backlog, and repository validation coverage for the new crate
- added bounded polling watch mode for `project-memory-cli` on top of the snapshot-based incremental refresh path
- added Rust structural enrichment for `project-memory-cli` queries through persisted symbols and imports
- added an ADR, workspace portability test coverage, and less rigid skill-root resolution in repo and package tests to keep scope boundaries enforceable

## Update rule

When you finish a meaningful maintenance change, update this file if at least one of these changed:

- the top priority
- the main blocker
- the main repository risk
- the expected next maintainer action
