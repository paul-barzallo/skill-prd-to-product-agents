# Open Gaps

This file tracks repository-level gaps that still matter for maintainability,
release quality, and operational discipline.

## How to use this file

- Keep entries concrete and repository-scoped.
- Prefer one gap per bullet.
- Remove or update a gap when it is genuinely closed.
- If a gap becomes a long-lived design choice, capture it in `docs/decisions/`.

## Priority 0

- The empty legacy path `docs/project/` may still exist in some environments and should be cleaned up once no tooling depends on it.
- Audit remediation still needs consistent upkeep even though `docs/audits/index.md` now exists.
- Repository changelog discipline must now be maintained consistently in `CHANGELOG.md`.

## Priority 1

- Support and escalation guidance for maintainers is still minimal.
- Release workflow behavior and release documentation still need periodic drift review.
- Published Unix binaries still need periodic verification that git executable bits remain intact across repository updates.
- The relationship between issue templates and audit follow-up process is not yet documented in `docs/audits/`.

## Priority 2

- No repository changelog focused on contract, packaging, and maintenance process changes.
- No explicit support policy for maintainers beyond the current contribution and security guidance.
- No repository-level dashboard or summary page linking current status, gaps, decisions, and audits in one place.

## Watch items

- Drift between release documentation and GitHub workflow behavior.
- Drift between packaged binary expectations and checksum maintenance.
- Drift caused by repository docs over-claiming what validators actually enforce.
