# Project Memory CLI Backlog

This directory contains the planning baseline and implementation backlog for `project-memory-cli`, the repository-side Rust CLI that now provides local project memory, repository ingestion, traceability, and incremental refresh for agent-driven product development workflows.

## Why this exists

The attached plan was directionally strong, but too broad to execute safely in one pass. The backlog in this directory narrowed that first iteration to a maintainable v1 that fits this repository, and the implementation now covers the planned slices in this area.

## Review Summary

- Keep the implementation under `cli-tools/project-memory-cli/` to match existing Rust CLI conventions in this repository.
- Treat repository ingestion, hashing, JSON contracts, query, traceability, and validation as MVP scope.
- Keep file-system watch as a follow-up after the fingerprint-based incremental path is stable.
- Defer symbol-level enrichment and advanced dependency analysis until the structural and documentary memory layers are proven useful.

## Directory Layout

- `architecture.md`: v1 architecture review and implementation framing.
- `roadmap.md`: phased delivery plan and priority breakdown.
- `epics.md`: epic-level slices and dependency chain.
- `mvp.md`: concise definition of the P0 cut.
- `issues/`: one Markdown specification per GitHub issue.

## Current Backlog Waves

- PMEM-000 through PMEM-009 captured the v1 foundation that is now implemented in this repository.
- PMEM-010 through PMEM-019 define the current provider-focused wave for configurable local and remote-safe semantic retrieval backends.

## Backlog Rules

- Each issue must have a concrete deliverable.
- Each issue must have explicit dependencies.
- P0 should be shippable without advanced parsing or external services.
- Documentation and tests are mandatory parts of the development plan, not cleanup tasks.

## GitHub Mapping

The Markdown issue files in this directory are the source material for the matching GitHub issues created for repository planning.

At this point, those planned issues have corresponding implementation in the repository. This directory remains useful as the design and delivery record for that work.

The provider-focused second wave continues to use this directory as the canonical source for issue text before or alongside GitHub issue creation.