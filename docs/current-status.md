# Current Status

Use this file as the maintainer handoff snapshot for the repository itself.
Update it when priorities, blockers, or key repository risks change.

## Snapshot

- Repository maturity: v1 presentable / hardened prototype
- Current publication goal: prepare the repository for clean GitHub publication and controlled maintenance
- Validation posture: repository CI, release gate, and pre-commit scaffolding exist; full local verification still depends on running the Cargo commands explicitly

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
5. release workflow and documentation drift still need periodic review

## Current blockers or risks

- the repo can still drift if maintainers change docs without checking the code path that enforces the claim
- local build outputs under `cli-tools/*/target/` remain easy to confuse with shipped artifacts if hygiene slips
- security and governance expectations can still be overstated unless limitations are kept visible in docs

## Recommended next actions

1. keep `docs/audits/index.md` updated as audits land or close
2. keep `CHANGELOG.md` updated when repository contracts or release behavior change
3. strengthen maintainer escalation/support guidance if needed
4. remove the empty legacy `docs/project/` path where still present
5. review whether release workflows need a documentation sync check

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
