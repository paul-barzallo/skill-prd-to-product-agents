# Open Gaps

This file tracks repository-level gaps that still matter for maintainability,
release quality, and operational discipline.

## How to use this file

- Keep entries concrete and repository-scoped.
- Prefer one gap per bullet.
- Remove or update a gap when it is genuinely closed.
- If a gap becomes a long-lived design choice, capture it in `docs/decisions/`.

## Priority 0

- The empty legacy path `docs/project/` may still exist in some bootstrapped workspaces and should be cleaned up once no workspace tooling depends on it.
- Audit remediation still needs consistent upkeep even when working notes stay outside the repo.
- Repository changelog discipline must now be maintained consistently in `CHANGELOG.md`.

## Priority 1

No active Priority 1 repository gaps right now. Keep release/workflow drift and published-binary mode integrity under Watch items because `test repo-validation` now enforces the current contract.

## Priority 2

- No explicit support policy for maintainers beyond the current contribution and security guidance.
- No repository-level dashboard or summary page linking current status, gaps, decisions, and audits in one place.

## Watch items

- Drift between release documentation and GitHub workflow behavior.
- Drift between packaged binary expectations and checksum maintenance.
- Drift caused by repository docs over-claiming what validators actually enforce.
