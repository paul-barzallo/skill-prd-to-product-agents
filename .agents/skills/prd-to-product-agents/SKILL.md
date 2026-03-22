---
name: prd-to-product-agents
description: >-
  Bootstrap a VS Code + GitHub Copilot multi-agent product-development
  workspace with runtime agents, prompts, canonical Markdown/YAML state,
  passive audit support, and controlled local bootstrap behavior.
user-invocable: true
disable-model-invocation: false
---

<!-- skill-version: 1.0.0 -->

# prd-to-product-agents

Use this skill when the user wants to install or maintain the `prd-to-product-agents` workspace template.

## Scope model

Keep these scopes separate:

| Scope | Location | Purpose |
| --- | --- | --- |
| Project repo | repository root, `docs/`, `cli-tools/skill-dev-cli/` | Develop, test, and release the skill |
| Skill package | `.agents/skills/prd-to-product-agents/` | Installable skill, bootstrap CLI, template, and package docs |
| Deployed workspace | generated target workspace | Runtime agents, governance, state files, and runtime CLI |

## State vocabulary

Use these labels consistently:

- `template`: content inside the skill package template
- `bootstrapped`: workspace generated locally but still pending local configuration
- `configured`: placeholders removed and governance configured locally
- `production-ready`: configured workspace with hardened governance enabled

## What bootstrap really does

`prd-to-product-agents-cli bootstrap workspace`:

- copies the template into a target workspace
- preserves user files and creates overlay proposals instead of overwriting collisions
- writes `.state/bootstrap-report.md` and `.state/bootstrap-manifest.txt`
- initializes SQLite only when available
- detects capabilities and writes `.github/workspace-capabilities.yaml`
- marks governance as pending local configuration when placeholders remain

It does not:

- provision GitHub remotely
- render a real `CODEOWNERS` policy from governance data
- declare a fresh workspace operationally ready

## Readiness and validation

A fresh bootstrap may pass structural validation and still be not ready.

- `Validation: PASS` means structure is intact.
- `Governance status` reports whether local governance still contains placeholders.
- `Readiness status` stays `not_ready` until local governance is configured.

Command semantics:

- `skill-dev-cli test release-gate`: repository release gate
- `prd-to-product-agents-cli validate all`: skill package integrity, including template encoding and agent consistency
- `prdtp-agents-functions-cli validate workspace`: structural validation of a generated workspace
- `prdtp-agents-functions-cli validate governance`: governance validation for configured workspaces
- `prdtp-agents-functions-cli validate readiness`: operational readiness validation for configured workspaces

## Dependency contract

Before running bootstrap for real, detect:

- `git`
- Git identity
- `sqlite3`
- `gh`
- `node`
- `npm`
- `markdownlint`

`workspace-capabilities.yaml` is the persisted capability snapshot and policy contract for commands that consult it.

Important:

- YAML validation uses the native Rust CLI and `serde_yaml`; `js-yaml` is not a hard runtime dependency.
- `markdownlint` is optional and may be disabled by policy.
- Missing `sqlite3` is degraded mode, not a hard stop.

## Platform contract

| Surface | VS Code + GitHub Copilot | GitHub.com |
| --- | --- | --- |
| Multi-agent orchestration | supported | degraded |
| `model:` routing | supported | ignored |
| Local runtime CLI + `.state/` | supported | degraded / runner-dependent |

Do not claim GitHub.com parity. The supported contract is Copilot-first in a local workspace.

## How to run it

Preferred command:

```shell
prd-to-product-agents-cli --skill-root <repo-or-skill-root> bootstrap workspace --target <workspace>
```

Useful variants:

```shell
prd-to-product-agents-cli --skill-root <repo-or-skill-root> bootstrap workspace --target <workspace> --preflight-only
prd-to-product-agents-cli --skill-root <repo-or-skill-root> bootstrap workspace --target <workspace> --dry-run
prd-to-product-agents-cli --skill-root <repo-or-skill-root> validate all
```

## Post-bootstrap checks

After bootstrap:

1. Read `.state/bootstrap-report.md`.
2. Confirm `Structure validation`, `Governance status`, and `Readiness status`.
3. Treat `bootstrapped` plus `not_ready` as expected until local governance is configured.
4. Use runtime docs inside the generated workspace for day-to-day operation.
