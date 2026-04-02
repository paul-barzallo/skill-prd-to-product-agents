# Bootstrap Usage Reference

This document supplements `SKILL.md` with package-level bootstrap details for `prd-to-product-agents-cli`.

## Parameters

| Parameter | Required | Default | Description |
| --- | --- | --- | --- |
| `--target` | No | `.` | Target workspace directory |
| `--project-name` | No | target folder name | Internal override; usually do not pass it |
| `--dry-run` | No | `false` | Preview without writing files |
| `--preflight-only` | No | `false` | Show dependency status and exit |
| `--skip-db-init` | No | `false` | Skip SQLite initialization |
| `--skip-git` | No | `false` | Skip git init and initial commit attempt |

## Re-running bootstrap

Bootstrap is re-runnable and preserves observable stability on rerun:

- identical files are skipped
- differing files produce overlay proposals
- reports and manifests may be regenerated in place
- capability detection is refreshed

Bootstrap preserves existing user files. It does not silently overwrite them.

On first bootstrap, sensitive capabilities such as `git`, `gh`, and
`markdownlint` are refreshed into `.github/workspace-capabilities.yaml` with
default-deny authorization unless the workspace is preserving an intentional
prior authorization state from a rerun.

## What is generated

Bootstrap seeds:

- workspace config files
- agent files and prompts
- canonical docs under `docs/project/`
- runtime docs under `docs/runtime/`
- `.github/workspace-capabilities.yaml`
- `.github/github-governance.yaml`
- `.state/bootstrap-report.md`
- `.state/bootstrap-manifest.txt`

## What is runtime-generated later

These may appear only after later runtime operations:

- `.state/project_memory.db`
- `.state/reporting/report-snapshot.json`
- `.state/local-history/*`
- `.state/work-units/*`

## Post-bootstrap verification

After bootstrap:

1. Check the command exit code.
2. Read `.state/bootstrap-report.md`.
3. Confirm `Structure validation`, `Governance status`, and `Readiness status`.
4. Expect a fresh workspace to be `bootstrapped` and often `not_ready`.
5. Run `prdtp-agents-functions-cli --workspace <workspace> governance configure` before treating the workspace as locally configured.
6. Refresh and review `.github/workspace-capabilities.yaml` with `prdtp-agents-functions-cli --workspace <workspace> capabilities detect`.
7. Intentionally authorize any sensitive capability you want to use, for example `git`, `gh`, `sqlite`, or `markdownlint`, via `prdtp-agents-functions-cli --workspace <workspace> capabilities authorize ...`.
8. If the workspace will use Git-backed task flow, run `prdtp-agents-functions-cli --workspace <workspace> git install-hooks` after `.git/` exists.
9. Use `validate governance` for the configured local gate and `validate readiness` only for the stronger optional `production-ready` enterprise overlay.
