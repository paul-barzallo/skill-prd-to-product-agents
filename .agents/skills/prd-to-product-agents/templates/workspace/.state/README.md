
# State Directory

- `memory-schema.sql`: Audit ledger schema (passive - managed by infrastructure, not by agents).
- `project_memory.db`: Created during bootstrap if audit ledger init succeeds.
- `bootstrap-manifest.txt`: Generated at runtime; records files touched by bootstrap using `path`, `kind`, `ownership`, and `cleanup_action` columns.
- `bootstrap-report.md`: Generated at runtime; bootstrap summary.
- `workspace-validation.md`: Generated during bootstrap validation and should not ship in the template.
- `sqlite-bootstrap.pending.md`: Generated at runtime only when SQLite init is deferred.
- `sqlite-bootstrap.report.md`: Generated after successful SQLite initialization.
- `logs/`: Generated at runtime only for CLI diagnostics and must not ship in the template.

## Notes

- Files in this directory mix template-owned inputs, such as `memory-schema.sql`,
  with runtime-generated evidence.
- Runtime-generated reports and logs must not be checked into the template.
- If Git capability is disabled, local evidence is written under
  `.state/local-history/`.
- If reporting is enabled, snapshot artifacts live under `.state/reporting/`.
