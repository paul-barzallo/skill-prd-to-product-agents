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
| `decisions/README.md` | Entry point for repository ADRs and long-lived choices. |
| `project-memory/README.md` | Planning baseline, roadmap, and issue specs for the proposed `project-memory-cli` initiative. |
| `project-memory-cli-reference.md` | Command reference for the repository-side `project-memory-cli`. |
| `skill-dev-cli-reference.md` | Reference for the project-only development CLI. |
| `repo-release-checklist.md` | Release checklist for publishing and validating the skill repository. |

## Boundaries

- `docs/` is the source of truth for repository maintenance.
- `skill-dev-cli` is the project maintenance CLI documented here.
- Do not use this directory to document unrelated packaged or generated content.
- Temporary audit notes stay outside the repo; only stable conclusions belong in the maintained docs under `docs/`.

## Directory ownership by scope

| Area | Primary scope | Notes |
| --- | --- | --- |
| `docs/` | project repo | maintainer docs only; may reference other scopes when the repository task explicitly needs that context |
| `.github/` | project repo | repository automation and review scaffolding |
| `cli-tools/skill-dev-cli/` | project repo | repository validation and release tooling |
| `cli-tools/project-memory-cli/` | project repo | local repository indexing and traceability tooling |
| `.agents/skills/prd-to-product-agents/` | skill package | packaged bootstrap surface and package references |
| `.agents/skills/prd-to-product-agents/templates/workspace/` | deployed workspace | runtime template copied into client repositories |
| `bin/` | project repo | published project-scope binaries only |

The repository owns the source tree for all of these areas, but ownership of
source files is not the same as runtime dependency. The deployed workspace must
remain operable without requiring the repository-maintenance surface after
bootstrap.

## Build artifacts

- `cli-tools/*/target/` contains local Rust build outputs only.
- Those directories are not part of the shipped skill package or the generated
  workspace runtime.
- Before handoff or packaging review, clean them with `cargo clean
  --manifest-path <crate>/Cargo.toml` or remove the relevant `target/`
  directory explicitly.

If a repository-level process changes, update the corresponding doc in this
folder.
