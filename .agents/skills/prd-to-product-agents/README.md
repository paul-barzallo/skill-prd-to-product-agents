# prd-to-product-agents

Human-oriented README for the `prd-to-product-agents` skill package.

This document is for people using the packaged skill to bootstrap a workspace.
Daily operation after bootstrap belongs to the generated workspace documentation.

## What it does

The skill generates a product-development workspace with:

- 9 custom agents
- canonical Markdown/YAML project state under `docs/project/`
- a runtime CLI for governance, state transitions, audit sync, and reporting
- local capability detection in `.github/workspace-capabilities.yaml`
- local governance placeholders in `.github/github-governance.yaml`

## Boundaries

This document covers only the packaged skill and the workspace it deploys.

| Scope | Purpose |
| --- | --- |
| Skill package | ship the bootstrap CLI, template, and package docs |
| Deployed workspace | run the agents and operational tooling in client projects |

After bootstrap, the deployed workspace must operate from its own files,
runtime binaries, and runtime documentation.

## Current bootstrap contract

Bootstrap creates a local workspace and preserves existing user files by using overlays for collisions.

Bootstrap does not:

- provision GitHub remotely
- make a new workspace operationally ready
- guarantee real reviewer identities or branch protection

Freshly generated workspaces should be described as:

- `template` inside the skill package
- `bootstrapped` once copied into a target repo
- `configured` only after local governance placeholders are replaced
- `production-ready` only after the stricter governance expectations are intentionally enabled

After bootstrap, the generated workspace uses its own runtime docs and local
runtime binaries. The skill package remains the delivery source, not an
operational dependency for normal workspace execution.

## Support matrix

| Surface | VS Code + GitHub Copilot | GitHub.com |
| --- | --- | --- |
| Multi-agent orchestration | supported | degraded |
| `model:` routing | supported | ignored |
| Runtime CLI + local `.state/` | supported | degraded / runner-dependent |

GitHub.com is intentionally documented as a degraded surface. The supported contract is Copilot-first in a local workspace.

## Command semantics

| Command | Scope |
| --- | --- |
| `prd-to-product-agents-cli validate all` | skill package integrity |
| `prdtp-agents-functions-cli validate workspace` | workspace structural validation |
| `prdtp-agents-functions-cli validate governance` | configured workspace governance validation |
| `prdtp-agents-functions-cli validate readiness` | configured workspace operational readiness |

## References

- `SKILL.md`: bootstrap contract for agent use
- `references/skill-bootstrap-usage.md`: package-level bootstrap reference
- `references/memory-model.md`: state and audit model
- `references/skill-platform-compatibility.md`: support claims for the skill package
