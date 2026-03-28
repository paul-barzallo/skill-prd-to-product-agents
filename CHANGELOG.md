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

- Repository maintainer docs moved from `docs/project/` to `docs/`.
- Repository guidance was narrowed to the project root scope instead of mixing packaged or generated surfaces by default.
- `skill-dev-cli test release-gate` is now explicitly documented as the blocking repository release command.
- Repository local validation now has a GitHub-aligned `skill-dev-cli test repo-validation` entrypoint, and Unix published binaries are explicitly guarded for executable permissions.
- `skill-dev-cli` now includes `test workflow-release-gate` to simulate the build workflow gate on the current platform using staged `collected` binaries.
- `skill-dev-cli test repo-validation` now includes `cargo test --manifest-path cli-tools/project-memory-cli/Cargo.toml` so the new repository-side CLI is covered by the canonical local validation chain.
- Skill-package and deployed-workspace docs now state post-bootstrap independence more explicitly, and repository tests now check workspace portability plus less rigid skill-root resolution.
- `project-memory-cli` now resolves embedding provider settings through flags, env vars, and `.project-memory/config.toml`, and it can persist vectors from a loopback-only `local_microservice` backend while keeping `local_hashed_v1` as the safe default.
- `project-memory-cli` now supports an explicit opt-in `openai_compatible` embedding bridge, Azure-compatible deployment shaping, remote safety gates, and model-aware embedding metadata persistence.
- `project-memory-cli` now supports explicit local-only fallback providers, provider/model-aware cache invalidation during retrieve, and richer retrieval diagnostics for effective backend, cache status, and fallback provenance.
- `project-memory-cli` ingest now includes hidden repository metadata such as `.github/workflows/` unless ignore rules explicitly exclude it.
- `project-memory-cli` now chunks YAML and GitHub workflow files by structural anchors such as top-level keys, jobs, and steps to improve retrieval quality on repository automation.
- `project-memory-cli` now persists recomputed retrieve embeddings back into SQLite so cache invalidation converges to reusable provider/model-aligned cache state.
- Packaged-skill docs and deployed-workspace docs now enforce cleaner scope boundaries, and the workspace runtime docs now include a dedicated files-first context-system guide for agents.
- Release version ownership now lives only in the repository root `VERSION` file, while packaged-skill detection and standalone bootstrap no longer require skill-scoped `VERSION` metadata.

### Notes

- Add a dated release section when the next repository release is cut.
- Keep this changelog focused on repository contract and maintenance changes, not generic code churn.
