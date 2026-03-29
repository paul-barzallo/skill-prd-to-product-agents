# Current Status

Use this file as the maintainer handoff snapshot for the repository itself.
Update it when priorities, blockers, or key repository risks change.

## Snapshot

- Repository maturity: v1 presentable / hardened prototype
- Current publication goal: keep repository maintenance coherent while incubating `project-memory-cli` as a repository-side capability
- Validation posture: repository CI, multi-OS release-gate, runtime smoke, and pre-commit/pre-push hooks now share a stricter GitHub-aligned local validation chain; `validate all` now executes a real temporary workspace smoke, and readiness is treated as a `production-ready` gate instead of a configured-state alias

## What is already in place

- root `README.md`, `CONTRIBUTING.md`, and `SECURITY.md`
- repo-level `.gitignore`, `.editorconfig`, PR template, and validation workflow
- explicit `AGENTS.md` rules for repository maintenance, packaging, and minimum validation
- release checklist in `docs/repo-release-checklist.md`
- `project-memory-cli` MVP foundation with ingest, chunk-aware query, chunk-first retrieve, trace, impact, and validate commands backed by `.project-memory/snapshot.json` plus `.project-memory/project-memory.db`
- `project-memory-cli` now also persists deterministic local chunk embeddings in SQLite and uses them in hybrid retrieve scoring
- `project-memory-cli` now resolves embedding provider configuration through flags, env vars, or `.project-memory/config.toml`, supports a loopback-only `local_microservice`, and supports an explicit opt-in `openai_compatible` bridge for generic and Azure-compatible embedding APIs
- `project-memory-cli` now also supports explicit local-only fallback policy, provider/model-aware cache invalidation, and retrieve diagnostics that surface effective backend, cache reuse, fallback use, and remote-risk level
- `project-memory-cli` ingest now includes hidden repository metadata such as `.github/workflows/` unless ignore rules explicitly exclude it
- `project-memory-cli` now chunks YAML and GitHub workflow files by structural anchors so retrieval over repository automation is less dependent on blind fixed windows
- `project-memory-cli` retrieve now persists recomputed embeddings back into SQLite so provider/model cache invalidation converges to reusable cache state on later runs

## Active strengths

- stronger separation between repository docs and other packaged surfaces
- canonical validation commands already exist in Rust CLIs
- packaged binary model is explicit
- claims about bootstrap and readiness are more disciplined than in earlier drafts
- runtime capability detection now emits parseable YAML via typed serialization instead of string-built output, and bootstrap/preflight now share that same capability schema instead of maintaining drift-prone duplicate renderers
- sensitive capability-gated commands now fail closed when `.github/workspace-capabilities.yaml` or the required `authorized.enabled` entry is missing
- task-branch checkout is now non-destructive on dirty worktrees, and `git finalize` blocks commit creation when workspace validation fails
- tracked publishable binaries are now refreshed through a reviewable PR path instead of a bot commit directly to `main`
- generated-workspace freshness now uses `.state/context-checksums.json` as the canonical baseline with staleness warnings on later runs
- the repository now documents repo, skill-package, and deployed-workspace scopes as separate contracts
- the packaged skill now avoids source-repository maintenance guidance, and the deployed workspace now documents its own files-first context system for agents
- template and runtime contract tests now better tolerate explicit skill-root injection instead of assuming only the current repository layout
- project version metadata now lives only at the repository root, and skill-root detection no longer treats `VERSION` as part of the packaged skill contract

## Current gaps to close next

1. structured audit archive and remediation tracking still need consistent upkeep
2. the legacy empty `docs/project/` path in bootstrapped workspaces still needs cleanup if still present in downstream environments
3. repository support and escalation flow is still minimal
4. the runtime contract is materially stricter now, and GitHub Project has been retired from the current supported contract; the remaining follow-through is centered on single-source-of-truth enforcement in CI, explicit capability authorization, and troubleshooting ergonomics
5. release workflow and documentation drift still need periodic review, especially around the reviewed binary-refresh PR path, executable-bit hygiene, and multi-OS gate scope
6. decide whether `project-memory-cli` should stay as repository-only tooling or eventually become part of a broader product workflow story
7. keep the provider diagnostics and validation matrix coherent as fallback behavior evolves, then decide whether any additional operator-facing reporting belongs in `trace` or separate commands

## Current blockers or risks

- the repo can still drift if maintainers bypass hooks or skip the GitHub-aligned local validation chain before pushing
- local build outputs under `cli-tools/*/target/` remain easy to confuse with shipped artifacts if hygiene slips
- security and governance expectations can still be overstated if future docs/prompts drift away from the stronger `production-ready` runtime contract, if prompt/tool frontmatter drifts from the validated execute policy, or if future binary publication changes reintroduce unreviewed mutation paths
- local validation still only simulates the current platform; cross-platform parity remains a GitHub responsibility even after pre-merge multi-OS gating

## Recommended next actions

1. when an audit lands, summarize durable conclusions into `docs/current-status.md`, `docs/open-gaps.md`, `docs/known-limitations.md`, or an ADR instead of versioning temporary audit notes
2. keep `CHANGELOG.md` updated when repository contracts or release behavior change
3. remove the empty legacy `docs/project/` path where still present in bootstrapped workspaces
4. add CI enforcement for the declared source-of-truth split between `SKILL.md` and `templates/workspace/docs/runtime/README.md`
5. operationalize the sandbox readiness workflow and keep its variables/secrets documented
6. decide whether `project-memory-cli` needs release or packaging policy beyond the current repository-only scope

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
- added SQLite-backed snapshot mirroring, deterministic chunk persistence, and chunk-aware query results for `project-memory-cli`
- added an explicit `retrieve` command as the forward-looking chunk retrieval contract for future hybrid semantic ranking
- added deterministic local chunk embeddings plus hybrid retrieve scoring so the retrieval pipeline now exercises stored vectors end to end
- added provider-aware embedding configuration plus a loopback-only `local_microservice` backend for `project-memory-cli`
- added an explicit opt-in `openai_compatible` bridge with Azure-compatible request shaping and remote safety gates for `project-memory-cli`
- added local-only provider fallback, explicit cache-status reporting, and richer retrieve diagnostics for `project-memory-cli`
- fixed `project-memory-cli` ingest so hidden repository metadata like `.github/workflows/` is indexed when not ignored
- improved `project-memory-cli` YAML chunking so workflow jobs and steps become first-class retrieval units instead of generic fixed windows
- fixed `project-memory-cli` retrieve so recomputed embeddings are persisted and later runs converge from `mismatch_recomputed` to `hit`
- cleaned the skill-package and deployed-workspace docs so each surface now explains only its own contract, and added a dedicated workspace runtime guide for the files-first context system
- added an ADR, workspace portability test coverage, and less rigid skill-root resolution in repo and package tests to keep scope boundaries enforceable
- moved release version ownership to the repository root `VERSION` file and removed skill-scoped `VERSION` assumptions from package validation, bootstrap reporting, and repo tests
- hardened the runtime contract so `validate readiness` is a true `production-ready` gate with remote GitHub checks, `capabilities detect` writes typed YAML, `git finalize` blocks invalid commits, `git checkout-task-branch` no longer rewrites dirty worktrees, and `validate all` exercises those behaviors through a real temporary workspace smoke
- retired GitHub Project from the current supported execution contract, aligned board/reporting docs to issues/PR snapshots, and tightened prompt/agent tool-contract validation around `execute`
- unified capability snapshot rendering across bootstrap, preflight, and runtime detection; split `detected.*` from `authorized.*`; added runtime PR-governance validation plus GitHub Issue wrappers; made capability-gated commands fail closed on missing authorization state; and moved binary refresh automation from direct pushes to a reviewed PR flow

## Update rule

When you finish a meaningful maintenance change, update this file if at least one of these changed:

- the top priority
- the main blocker
- the main repository risk
- the expected next maintainer action
