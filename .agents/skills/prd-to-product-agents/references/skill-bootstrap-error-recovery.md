
# Bootstrap Error Recovery

This reference documents the current Rust CLI behavior for
`prd-to-product-agents-cli` package validation and workspace bootstrap.
It replaces older shell-script recovery guidance.

## Package validation fails before bootstrap

Do not continue bootstrapping from a package that fails validation.

- Run `prd-to-product-agents-cli --skill-root <skill-root> validate package`.
- Common causes: missing bundle metadata, checksum drift, unexpected `.state/`
  residue in the distributed skill, or published command-surface drift.
- Recovery: fix the packaged skill contents at the source, then rerun
  validation. Do not patch a generated workspace to hide a package failure.

## Bootstrap finishes with `Status: DEGRADED`

`DEGRADED` means the workspace files were written, but an optional runtime step
could not be completed during bootstrap.

- Read `.state/bootstrap-report.md` for the recorded reason.
- The most common current cause is deferred or failed database initialization.
- If SQLite should be active, intentionally authorize it and then run
  `prdtp-agents-functions-cli --workspace <workspace> database init`.
- If SQLite remains unauthorized, spool-only audit behavior is expected and is
  not itself a bootstrap failure.

## File collision or host merge

Bootstrap preserves existing user files rather than overwriting them silently.

- The existing file stays in place.
- The proposed replacement is written under `.bootstrap-overlays/`.
- Recovery: review the overlay, merge it manually if needed, and rerun
  bootstrap once the host file is in the desired state.

## Git unavailable, disabled, or not yet initialized

Bootstrap does not require Git to generate the workspace.

- If `.git/` is missing, the commit and hook-installation path is skipped.
- If `capabilities.git.authorized.enabled=false`, Git-backed task flow remains
  out of contract after bootstrap.
- Recovery: initialize the repository, run
  `prdtp-agents-functions-cli --workspace <workspace> capabilities detect`,
  intentionally authorize Git if desired, and then run
  `prdtp-agents-functions-cli --workspace <workspace> git install-hooks`.

## Governance or readiness still blocked after bootstrap

A fresh workspace is expected to be incomplete.

- `Workspace State: bootstrapped` plus `Readiness status: not_ready` is normal.
- Recovery: run
  `prdtp-agents-functions-cli --workspace <workspace> governance configure`
  first, then use `validate governance` for the configured local gate.
- Use `validate readiness` only for the stronger optional enterprise overlay.

## Template tokens remain unreplaced

If `{{PROJECT_NAME}}` remains in generated files, treat it as a template-source
or bootstrap-interruption problem.

- Search the generated workspace for `{{PROJECT_NAME}}`.
- Rerun bootstrap with `--project-name "Your Name"` if the original run omitted
  an intentional name.
- If a collision prevented replacement, review `.bootstrap-overlays/` or edit
  the canonical file manually.

## Historical shell notes

Older PowerShell and Bash recovery notes about CRLF stderr suppression, shell
wrapper parsing, or `.ps1` em-dash handling are historical only.

- The supported bootstrap path is now the Rust CLI.
- Generated text files are normalized to LF during bootstrap.
- Current failures should be diagnosed from `validate package`,
  `.state/bootstrap-report.md`, and the runtime validation commands rather than
  from historical shell-script behavior.
