# Current Status

Use this file as the maintainer handoff snapshot for the repository itself.
Update it when priorities, blockers, or key repository risks change.

## Snapshot

- Repository maturity: v1 presentable / hardened prototype
- Current publication goal: prepare the repository for clean GitHub publication and controlled maintenance
- Validation posture: repository CI, multi-OS release-gate, and pre-commit/pre-push hooks now have a single GitHub-aligned local validation command; published Unix binaries still require executable-bit discipline

## What is already in place

- root `README.md`, `CONTRIBUTING.md`, and `SECURITY.md`
- repo-level `.gitignore`, `.editorconfig`, PR template, and validation workflow
- explicit `AGENTS.md` rules for repository maintenance, packaging, and minimum validation
- release checklist in `docs/repo-release-checklist.md`

## Active strengths

- stronger separation between repository docs and other packaged surfaces
- canonical validation commands already exist in Rust CLIs
- packaged binary model is explicit
- claims about bootstrap and readiness are more disciplined than in earlier drafts

## Current gaps to close next

1. structured audit archive and remediation tracking just started
2. the legacy empty `docs/project/` path still needs cleanup if still present in all environments
3. repository support and escalation flow is still minimal
4. changelog discipline now exists but still needs ongoing maintainer use
5. release workflow and documentation drift still need periodic review, especially around path filters, published binary permissions, and multi-OS gate scope

## Current blockers or risks

- the repo can still drift if maintainers bypass hooks or skip the GitHub-aligned local validation chain before pushing
- local build outputs under `cli-tools/*/target/` remain easy to confuse with shipped artifacts if hygiene slips
- security and governance expectations can still be overstated unless limitations are kept visible in docs
- local validation still only simulates the current platform; cross-platform parity remains a GitHub responsibility even after pre-merge multi-OS gating

## Recommended next actions

1. keep `docs/audits/index.md` updated as audits land or close
2. keep `CHANGELOG.md` updated when repository contracts or release behavior change
3. strengthen maintainer escalation/support guidance if needed
4. remove the empty legacy `docs/project/` path where still present
5. review whether release workflows need a documentation sync check and whether published binary permissions and path filters remain stable

## Last repository housekeeping changes

- added repo publication hygiene files and validation automation
- added maintainer-facing contribution and security guidance
- added architecture map, current status, and audits area to reduce context loss
- added open gaps, known limitations, and initial repository ADRs
- added issue templates, maintainer runbook, and repository test matrix
- added audit index and repository changelog

## Update rule

When you finish a meaningful maintenance change, update this file if at least one of these changed:

- the top priority
- the main blocker
- the main repository risk
- the expected next maintainer action
