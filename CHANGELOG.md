# Changelog

This changelog records repository-level changes that matter for maintenance,
validation, packaging, and release behavior.

## Unreleased

### Added

- Root repository hygiene files: `.gitignore`, `.editorconfig`, PR template, and repository validation workflow.
- Maintainer-facing guidance: `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, and `.github/copilot-instructions.md`.
- Repository documentation structure under `docs/`, including architecture map, current status, audits, decisions, gap tracking, known limitations, runbook, and test matrix.
- Repository issue templates for bugs, feature requests, audit findings, and release regressions.
- `project-memory-cli` under `cli-tools/project-memory-cli/` with deterministic ingest, query, trace, impact, and validate commands over a local `.project-memory/snapshot.json` store.
- Planning and implementation notes for `project-memory-cli` under `docs/project-memory/` and `docs/project-memory-cli-reference.md`.
- bounded `watch` mode for `project-memory-cli`, built on polling and the existing incremental ingest path.
- ADR-0003 to formalize scope boundaries between the project repo, the skill package, and the deployed workspace.

### Changed

- The packaged skill now exposes `prd-to-product-agents-cli validate package` as the portable consumer validation surface, while `validate all` is explicitly maintainer-only and includes source-checkout runtime smoke.
- The skill bootstrap CLI now writes diagnostic logs to the system temp area instead of mutating the distributed skill package during validation or bootstrap.
- Packaged bundle validation now stays local-first by verifying checksums, SBOM manifests, and provenance-policy structure without requiring authenticated `gh attestation verify` during the default package validation path.
- Skill package and template hygiene checks now reject unexpected runtime residue under `.state/`, including template log files, and the shipped workspace template no longer includes a checked-in CLI diagnostic log.
- The runtime CLI now also routes diagnostics to temp when maintainers run it against the packaged workspace template, so agent assembly and validation no longer re-contaminate the distributed skill with `.state/logs/cli-diagnostic.log`.
- Prompt and agent contract docs now narrow `execute` more aggressively: document-only prompts such as `clarify-prd` and `small-doc-update` no longer declare `execute`, and architecture/product/UX/PM roles are documented as runtime-CLI-only instead of implicitly sharing engineering shell scope.
- Runtime and package docs now describe `enterprise` strictly as an optional remote overlay on top of the validated `core-local` path, with `token-api` documented as the only supported remote auth mode.
- Runtime docs now publish a claim-to-evidence matrix, execute tables are framed as intended call sets instead of hard enforcement, and contract drift checks fail if unsupported `github-app` claims or stronger execute wording reappear.
- `.github/workflows/enterprise-readiness-sandbox.yml` now stages an isolated packaged skill copy, records `validate package` plus bootstrap evidence, uses the published runtime surface for the distributed contract steps, and uploads a reviewable enterprise evidence artifact after provisioning remote controls, re-validating readiness, and probing the remote audit sink.
- Portable package validation now enforces the source-of-truth split between `SKILL.md` and `templates/workspace/docs/runtime/README.md`, including drift checks that keep repository-maintenance and package-validation guidance out of the deployed runtime README.
- Repository maintainer docs moved from `docs/project/` to `docs/`.
- Repository guidance was narrowed to the project root scope instead of mixing packaged or generated surfaces by default.
- `skill-dev-cli test release-gate` is now explicitly documented as the blocking repository release command.
- Repository local validation now has a GitHub-aligned `skill-dev-cli test repo-validation` entrypoint, and Unix published binaries are explicitly guarded for executable permissions.
- `skill-dev-cli` now includes `test workflow-release-gate` to simulate the build workflow gate on the current platform using staged `collected` binaries.
- `skill-dev-cli test repo-validation` now includes `cargo test --manifest-path cli-tools/project-memory-cli/Cargo.toml` so the new repository-side CLI is covered by the canonical local validation chain.
- Skill-package and deployed-workspace docs now state post-bootstrap independence more explicitly, and repository tests now check workspace portability plus less rigid skill-root resolution.
- Workspace capability snapshots are now rendered from one shared schema across bootstrap, preflight, and runtime detection, and sensitive capability-gated commands now fail closed if the capability file or required policy entry is missing.
- Bootstrap/preflight and runtime capability discovery now share one path-based command-detection helper, bootstrap-report capability lines are regression-tested against `workspace-capabilities.yaml`, the packaged bootstrap recovery reference now matches the Rust CLI contract, and maintainer docs now map audit and release issues to durable follow-up paths.
- PRD/bootstrap prompts and workspace governance docs now distinguish Git-enabled and local-only closure paths explicitly, and prompt validation now rejects invalid `state handoff create --reason` examples semantically instead of relying on one exact forbidden string.
- `.github/workflows/build-skill-binaries.yml` now proposes tracked binary refreshes through a reviewed pull request instead of committing and pushing binaries directly to `main`, with workflow-wide permissions reduced to read-only outside that scoped publish step.
- `project-memory-cli` now resolves embedding provider settings through flags, env vars, and `.project-memory/config.toml`, and it can persist vectors from a loopback-only `local_microservice` backend while keeping `local_hashed_v1` as the safe default.
- `project-memory-cli` now supports an explicit opt-in `openai_compatible` embedding bridge, Azure-compatible deployment shaping, remote safety gates, and model-aware embedding metadata persistence.
- `project-memory-cli` now supports explicit local-only fallback providers, provider/model-aware cache invalidation during retrieve, and richer retrieval diagnostics for effective backend, cache status, and fallback provenance.
- `project-memory-cli` ingest now includes hidden repository metadata such as `.github/workflows/` unless ignore rules explicitly exclude it.
- `project-memory-cli` now chunks YAML and GitHub workflow files by structural anchors such as top-level keys, jobs, and steps to improve retrieval quality on repository automation.
- `project-memory-cli` now persists recomputed retrieve embeddings back into SQLite so cache invalidation converges to reusable provider/model-aligned cache state.
- Packaged-skill docs and deployed-workspace docs now enforce cleaner scope boundaries, and the workspace runtime docs now include a dedicated files-first context-system guide for agents.
- Release version ownership now lives only in the repository root `VERSION` file, while packaged-skill detection and standalone bootstrap no longer require skill-scoped `VERSION` metadata.
- Workspace capability governance now separates `detected.*` from `authorized.*`, keeps sensitive capability commands fail-closed on missing authorization, and adds an explicit runtime authorization command instead of treating auto-detection as permission.
- The workspace runtime CLI now provides typed `validate pr-governance` and `validate release-gate` commands, GitHub Issue mutation wrappers, and structured `audit export` evidence for sensitive operations and handoff/state records.
- Repository supply-chain controls now include dependency review, `cargo deny`, and CI build provenance attestation for published binary runs, while release guidance has been tightened around those gates.
- Immutable governance now relies on remote PR approval checked through `github.immutable_governance` and `validate pr-governance`, the runtime CLI now wraps GitHub PR mutation paths in addition to issue mutation, and published bundles now ship SPDX SBOM plus provenance-policy metadata with consumer-side verification.
- The runtime now splits `core-local` and `enterprise` operating profiles explicitly, records sensitive actions in a local hash-chained ledger with optional remote acknowledgement, degrades `audit sync` safely when SQLite is unavailable, and mutates operational YAML state through typed machine-managed serializers instead of regex/string replacement.

### Notes

- Add a dated release section when the next repository release is cut.
- Keep this changelog focused on repository contract and maintenance changes, not generic code churn.
