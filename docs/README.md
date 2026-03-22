# Repository Documentation

This directory contains repository-level documentation for maintaining the
`prd-to-product-agents` repository itself.

## Scope

- Repository maintenance, packaging, and release process.
- Project-only tooling such as `skill-dev-cli`.
- Maintainer documentation for the current project root.

This directory does not document other packaged or generated surfaces unless a
repo-level release or validation task explicitly requires that reference.

## Files

| File | Purpose |
| --- | --- |
| `architecture-map.md` | Repository-level map of scopes, CLIs, flows, and sources of truth. |
| `current-status.md` | Maintainer snapshot of active priorities, risks, and next actions. |
| `open-gaps.md` | Repository-level gap tracker for maintenance and release work. |
| `known-limitations.md` | Repository-level limits that should remain explicit. |
| `maintainer-runbook.md` | Practical operating guide for repository maintenance. |
| `test-matrix.md` | Matrix of repository validation commands and their coverage. |
| `audits/README.md` | Entry point for repository-level audits and review history. |
| `audits/index.md` | Status tracker for finalized repository audits and their follow-up. |
| `decisions/README.md` | Entry point for repository ADRs and long-lived choices. |
| `skill-dev-cli-reference.md` | Reference for the project-only development CLI. |
| `repo-release-checklist.md` | Release checklist for publishing and validating the skill repository. |

## Boundaries

- `docs/` is the source of truth for repository maintenance.
- `skill-dev-cli` is the project maintenance CLI documented here.
- Do not use this directory to document unrelated packaged or generated content.
- Historical or future audits should live under `docs/audits/`.

## Build artifacts

- `cli-tools/*/target/` contains local Rust build outputs only.
- Those directories are not part of the shipped skill package or the generated
  workspace runtime.
- Before handoff or packaging review, clean them with `cargo clean
  --manifest-path <crate>/Cargo.toml` or remove the relevant `target/`
  directory explicitly.

If a repository-level process changes, update the corresponding doc in this
folder.
