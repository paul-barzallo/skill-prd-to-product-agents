# ADR-0002: Release Gate Blocks Repository Release

## Status

Accepted

## Context

The repository contains multiple Rust CLIs, packaged binaries, and maintainer
documentation that can drift independently. A release process based only on
manual review would be too fragile and too easy to bypass accidentally.

## Decision

`skill-dev-cli test release-gate` is the blocking repository release check for
structural or release-sensitive changes.

Supporting validations remain useful on their own, but release intent is not
considered sufficiently checked until the aggregated release gate passes.

## Consequences

- Maintainers have a single named command to treat as the final repository gate.
- Supporting checks still matter, but they are not a substitute for the release gate.
- Any workflow or documentation that claims release readiness should stay aligned with this command.
- If the release-gate contents change materially, the repository checklist and docs must change in the same update.

## Related docs

- `docs/repo-release-checklist.md`
- `docs/skill-dev-cli-reference.md`
- `.github/workflows/repo-validation.yml`
