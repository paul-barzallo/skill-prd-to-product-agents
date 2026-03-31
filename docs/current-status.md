# Current Status

Use this file as the maintainer handoff snapshot for the repository itself.
Update it when priorities, blockers, or key repository risks change.

## Snapshot

- Repository maturity: v1 presentable / hardened prototype
- Current publication goal: keep repository maintenance coherent while incubating `project-memory-cli` as a repository-side capability
- Validation posture: repository CI, multi-OS release-gate, runtime smoke, and pre-commit/pre-push hooks now share a stricter GitHub-aligned local validation chain; the packaged skill now separates portable `validate package` checks from maintainer-only `validate all`, and readiness is treated as a `production-ready` gate instead of a configured-state alias

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
- immutable governance no longer relies on a local bypass token; PR validation now requires remote reviewer approval and labels declared in `.github/github-governance.yaml`
- published skill bundles now ship checksum manifests, SPDX SBOMs, and provenance-policy files, and packaged consumption now validates those local bundle materials without mutating the distributed skill root
- the workspace runtime now splits `core-local` and `enterprise` operating profiles explicitly, uses a local hash-chained sensitive-action ledger, and degrades `audit sync` safely instead of failing closed when SQLite is unavailable or unauthorized
- generated-workspace freshness now uses `.state/context-checksums.json` as the canonical baseline with staleness warnings on later runs
- the repository now documents repo, skill-package, and deployed-workspace scopes as separate contracts
- the packaged skill now avoids source-repository maintenance guidance, and the deployed workspace now documents its own files-first context system for agents
- template and runtime contract tests now better tolerate explicit skill-root injection instead of assuming only the current repository layout
- project version metadata now lives only at the repository root, and skill-root detection no longer treats `VERSION` as part of the packaged skill contract
- the packaged skill no longer writes CLI diagnostics into the distributed bundle during validation, and template-state hygiene now fails closed on unexpected runtime residue under `.state/`
- the runtime CLI now also avoids writing diagnostics into the packaged workspace template during maintainer assembly/validation, so repo maintenance no longer re-dirties the distributed skill
- workspace contract docs now narrow `execute` more explicitly: product, UX, architecture, and PM stay on runtime-CLI coordination calls, while only engineering roles keep scoped build/test/lint access
- runtime and package docs now describe `enterprise` as an optional remote overlay on top of the validated `core-local` path, with `token-api` as the only supported remote auth mode
- the deployed workspace runtime docs now publish a claim-to-evidence matrix, and CI rejects drift back to unsupported `github-app` wording or stronger execute-enforcement language than the runtime actually provides
- the publisher sandbox workflow now provisions remote enterprise controls, reruns readiness, probes the remote audit sink, and uploads a reviewable evidence artifact instead of stopping at local configuration
- local `repo-validation` now re-proves package portability, template hygiene, and the consumer versus maintainer validation split together, so those items should be treated as regression surfaces rather than open backlog

## Current gaps to close next

1. structured audit archive and remediation tracking still need consistent upkeep
2. the legacy empty `docs/project/` path in bootstrapped workspaces still needs cleanup if still present in downstream environments
3. repository support and escalation flow is still minimal
4. the runtime contract is materially stricter now, and GitHub Project has been retired from the current supported contract; the remaining follow-through is centered on keeping the new evidence workflow, claim matrix, SQLite detected-default wording, and external sandbox prerequisites documented and exercised
5. release workflow and documentation drift still need periodic review, especially around provenance policy, SBOM regeneration, executable-bit hygiene, and multi-OS gate scope
6. decide whether `project-memory-cli` should stay as repository-only tooling or eventually become part of a broader product workflow story
7. keep the provider diagnostics and validation matrix coherent as fallback behavior evolves, then decide whether any additional operator-facing reporting belongs in `trace` or separate commands
8. keep `execute` claims narrow: the packaged skill governs execution through prompts, capability policy, and validation, but it still does not provide a universal command sandbox boundary
9. keep immutable-governance wording narrow unless validation is intentionally strengthened beyond the current separate-identities plus declared-reviewer approval model

## Current blockers or risks

- the repo can still drift if maintainers bypass hooks or skip the GitHub-aligned local validation chain before pushing
- local build outputs under `cli-tools/*/target/` remain easy to confuse with shipped artifacts if hygiene slips
- `core-local` remains intentionally below compliance-grade evidence because the authoritative remote audit sink exists only in `enterprise`
- live enterprise evidence is currently blocked by remote publication state: `.github/workflows/enterprise-readiness-sandbox.yml` exists locally, but GitHub does not currently expose it on `origin/main` or `origin/develop`, so the workflow cannot yet be dispatched remotely
- the packaged skill still treats `execute` as a governed capability rather than a hard sandbox, so enterprise claims must keep that boundary explicit
- the current immutable-governance gate proves separate identities plus approval from one declared immutable-governance reviewer; docs must not sell that as stronger dual-control unless validation changes
- remote audit acknowledgement currently proves a non-empty `ack_id` from the configured sink, not immutable retention or a cryptographic receipt
- security and governance expectations can still be overstated if future docs/prompts drift away from the stronger `production-ready` runtime contract, if prompt/tool frontmatter drifts from the validated execute policy, if the enterprise evidence workflow stops being maintained, or if future binary publication changes reintroduce unreviewed mutation paths
- local validation still only simulates the current platform; cross-platform parity remains a GitHub responsibility even after pre-merge multi-OS gating

## Recommended next actions

1. when an audit lands, summarize durable conclusions into `docs/current-status.md`, `docs/open-gaps.md`, `docs/known-limitations.md`, or an ADR instead of versioning temporary audit notes
2. keep `CHANGELOG.md` updated when repository contracts or release behavior change
3. remove the empty legacy `docs/project/` path where still present in bootstrapped workspaces
4. add CI enforcement for the declared source-of-truth split between `SKILL.md` and `templates/workspace/docs/runtime/README.md`
5. publish `.github/workflows/enterprise-readiness-sandbox.yml` to a remote branch visible to GitHub Actions before retrying the live enterprise evidence run, then keep its variables, secrets, and uploaded artifact under active review
6. decide whether `project-memory-cli` needs release or packaging policy beyond the current repository-only scope
7. rebaseline local stabilization status notes when `repo-validation` moves an item from open backlog to regression-protected local proof

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
- split packaged-skill validation into portable `validate package` and maintainer-only `validate all`, moved skill CLI diagnostics out of the distributed bundle, and tightened `.state/` hygiene so checked-in runtime residue is rejected earlier
- removed `execute` from documentation-only prompts that do not need shell access, tightened CI drift checks around that contract, and stopped runtime maintenance commands from reintroducing template log residue
- narrowed the supported enterprise auth contract to `token-api` only, added a runtime claim-to-evidence matrix, extended CI drift checks to reject stale `github-app` and execute-enforcement wording, and upgraded the publisher sandbox workflow to provision remote controls and upload evidence artifacts
- rebaselined the stabilization status so already-green local package and template items are treated as regression surfaces, clarified SQLite detected-default behavior in shipped docs, and narrowed the remote audit sink promise to a non-empty `ack_id`

## Update rule

When you finish a meaningful maintenance change, update this file if at least one of these changed:

- the top priority
- the main blocker
- the main repository risk
- the expected next maintainer action
