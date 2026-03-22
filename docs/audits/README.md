# Audits

This directory stores repository-level assessments, audits, and review outputs
that matter for maintenance decisions.

## Purpose

Use this area to preserve technical review history without scattering findings
across ad hoc root files or chat transcripts.

## What belongs here

- enterprise readiness assessments
- security and governance reviews
- packaging and release audits
- remediation summaries linked to specific review findings

## What does not belong here

- runtime-generated audit ledgers from deployed workspaces
- temporary scratch notes
- duplicate copies of the same review in multiple formats

## Naming convention

Use stable names with date and topic when the audit is finalized:

- `YYYY-MM-topic.md`
- `YYYY-MM-DD-topic.md` when multiple reviews happen in the same month

Examples:

- `2026-03-big-four-readiness.md`
- `2026-03-release-process-review.md`

## Minimum structure for each audit file

Each audit should state:

- scope reviewed
- evidence reviewed
- confirmed facts
- refuted claims or contradictions
- findings by severity
- remediation recommendations
- final decision or suitability statement

## Follow-up expectation

If an audit changes current priorities or repository risk posture, also update:

- `docs/current-status.md`
- `docs/audits/index.md`
- later, the ADR or gap document if the audit closes or opens a structural decision
