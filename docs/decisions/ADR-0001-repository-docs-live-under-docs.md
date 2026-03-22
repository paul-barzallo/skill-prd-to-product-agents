# ADR-0001: Repository Docs Live Under docs/

## Status

Accepted

## Context

Repository maintenance documentation had started to drift into a nested
`docs/project/` location that blurred the line between repository-level process
docs and other kinds of content. That made the repository harder to navigate
and increased the chance of mixing unrelated scopes in maintainer guidance.

## Decision

Repository maintenance documentation lives directly under `docs/`.

This includes:

- repository overview and navigation
- architecture map
- current status
- release checklist
- maintainer CLI reference
- audits
- decisions

Repository docs must describe the current project root and its maintenance
process. They should not expand into unrelated packaged or generated surfaces
unless a repository task explicitly requires that reference.

## Consequences

- Maintainers have a single repository doc root to inspect before editing.
- Repository navigation becomes simpler and less ambiguous.
- Legacy repository docs under `docs/project/` should not be revived.
- New repository process documents should be added under `docs/`, not under ad hoc folders.

## Related docs

- `docs/README.md`
- `docs/architecture-map.md`
- `docs/current-status.md`
