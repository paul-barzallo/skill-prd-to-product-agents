# Audit Index

This index tracks repository-level audits, their status, and whether follow-up
work is still pending.

## How to use this file

- Add one row per finalized repository audit.
- Update the status when remediation changes materially.
- Link follow-up work through issues, ADRs, or gap documents when available.

## Audit status table

| Audit | Date | Scope | Status | Follow-up |
| --- | --- | --- | --- | --- |
| Big Four readiness review | 2026-03-22 | repository maintainability, governance, release readiness | reviewed | migrate any final write-up into `docs/audits/` and track open remediations |

## Status vocabulary

| Status | Meaning |
| --- | --- |
| `reviewed` | audit completed and recorded |
| `in-remediation` | follow-up work is active |
| `partially-closed` | some recommendations landed, others remain |
| `closed` | the audit no longer drives active repository work |

## Follow-up rule

When an audit changes priorities or release posture, also update:

- `docs/current-status.md`
- `docs/open-gaps.md`
- `docs/decisions/` if a durable repository decision was made
