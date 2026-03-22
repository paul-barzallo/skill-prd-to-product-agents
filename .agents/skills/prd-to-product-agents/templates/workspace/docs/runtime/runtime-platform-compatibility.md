# Runtime Platform Compatibility

Cross-platform evidence for the workspace runtime CLI (`prdtp-agents-functions-cli`).

## Support matrix

| Surface | VS Code / GitHub Copilot | GitHub.com | Residual risk |
| --- | --- | --- | --- |
| Agent routing and `model:` frontmatter | supported | degraded / ignored | GitHub.com does not honor per-agent routing layers |
| Runtime CLI from workspace-local binary | supported | best-effort via CI/runner surfaces | GitHub.com is not the canonical multi-agent runtime |
| Governance + readiness workflow | supported | degraded | local workspace validation is the source of truth |
| Reporting UI and local `.state/` surfaces | supported | unavailable | GitHub.com has no equivalent persistent local runtime state |

## Runtime CLI platform evidence

Status vocabulary:

- `Verified` means the capability is exercised automatically in CI on the stated platform.
- `Best-effort` means the surface exists but is not backed by an end-to-end CI assertion on that platform.

| Capability / surface | Windows | Ubuntu/Linux | Evidence |
| --- | --- | --- | --- |
| `database init` | Verified | Verified | Exercised by bootstrap smoke and post-bootstrap validation |
| `audit sync` | Verified | Verified | Smoke and CI exercise normal and degraded sync paths |
| `state *` | Verified | Verified | CI runs create/update lifecycle checks for handoffs, findings, and releases |
| `git pre-commit-validate` | Verified | Verified | Shared validators are invoked directly in smoke tests and CI |
| `git install-hooks` | Verified | Verified | Hook installation is exercised in smoke tests |

## Required tool versions

| Tool | Minimum version | Notes |
| --- | --- | --- |
| git | 2.30 | Required for branch and commit operations |
| gh | 2.0 | Optional; used for board sync and GitHub automation |
| gitleaks | 8.0 | CI gate downloads a pinned version; local hook warns when missing |
| sqlite3 | any | Optional; used by the SQLite mirror |

Bash and PowerShell are not runtime dependencies for the main CLI surfaces; the CLIs are native Rust binaries.

## Linux tooling note

Linux / WSL verification assumes native Linux binaries for `node`, `npm`, and `markdownlint` when those tools are enabled.
If those commands resolve through `/mnt/c/...`, validation emits warnings because the environment is effectively using a Windows wrapper.

macOS is currently a best-effort surface.

GitHub.com is a degraded execution surface for the orchestration layer. Do not claim feature parity with VS Code + GitHub Copilot.
