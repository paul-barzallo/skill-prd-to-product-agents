# Changelog

This changelog records repository-level changes that matter for maintenance,
validation, packaging, and release behavior.

## Unreleased

### Added

- Root repository hygiene files: `.gitignore`, `.editorconfig`, PR template, and repository validation workflow.
- Maintainer-facing guidance: `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, and `.github/copilot-instructions.md`.
- Repository documentation structure under `docs/`, including architecture map, current status, audits, decisions, gap tracking, known limitations, runbook, and test matrix.
- Repository issue templates for bugs, feature requests, audit findings, and release regressions.

### Changed

- Repository maintainer docs moved from `docs/project/` to `docs/`.
- Repository guidance was narrowed to the project root scope instead of mixing packaged or generated surfaces by default.
- `skill-dev-cli test release-gate` is now explicitly documented as the blocking repository release command.
- Repository local validation now has a GitHub-aligned `skill-dev-cli test repo-validation` entrypoint, and Unix published binaries are explicitly guarded for executable permissions.
- `skill-dev-cli` now includes `test workflow-release-gate` to simulate the build workflow gate on the current platform using staged `collected` binaries.

### Notes

- Add a dated release section when the next repository release is cut.
- Keep this changelog focused on repository contract and maintenance changes, not generic code churn.
