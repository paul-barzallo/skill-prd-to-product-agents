# Sync Mapping Reference

This document defines how canonical project files are mirrored into the SQLite audit/reporting store.

## Design principle

SQLite is a passive mirror and drift-detection index managed by runtime infrastructure, not a domain database.

The sync process:

1. Computes checksums for canonical files
2. Upserts artifact metadata and checksums
3. Logs each sync action to the audit tables

It does not:

- establish precedence over canonical files
- create or manage operational state
- replace YAML or Markdown as the system of record

Canonical files are always authoritative. If SQLite disagrees with the file on disk, the file wins.

## Mapping table

| Canonical doc | SQLite use |
| --- | --- |
| `docs/project/*.md` | Artifact checksum mirror and sync audit |
| `docs/project/*.yaml` | Artifact checksum mirror and sync audit |
| `docs/project/architecture/*.md` | One artifact row per file |
| `docs/project/decisions/*.md` | One artifact row per file |
| `docs/project/ux/*.md` | One artifact row per file |
| each sync execution | Sync run and failure records |

## Release artifacts

- `docs/project/releases.yaml` is the operational source of truth for release state transitions.
- `docs/project/releases.md` is the human-readable companion artifact for notes and rollout context.
- Sync stores both so dashboards and audits can distinguish operational state from narrative documentation.

## Checksum strategy

Each file's SHA-256 checksum is stored in SQLite. During sync:

1. Compute the current checksum
2. Compare it with the stored checksum
3. If unchanged, skip the write
4. If changed, upsert the mirror row and record the sync activity

## Error recovery

If a file fails to process:

1. A sync failure record is written when SQLite is available
2. The sync continues with the next file
3. The overall sync result becomes partial instead of completed
