---
name: prd-to-product-agents
description: >-
  Bootstrap a VS Code + GitHub Copilot multi-agent product-development
  workspace with runtime agents, prompts, canonical Markdown/YAML state,
  passive audit support, and controlled local bootstrap behavior.
user-invocable: true
disable-model-invocation: false
---

# prd-to-product-agents

Use this skill when the user wants to install or maintain the `prd-to-product-agents` workspace template.

## Agent execution requirements

When this skill is invoked, treat it as an operating contract, not as background reading.

- Read this file before answering.
- Apply its terminology and command semantics in the same turn.
- Do not stop at an acknowledgement such as `already read`, `ya lo lei`, or a summary of the file.
- The first substantive reply after reading this skill must do one of these things:
  - perform the requested task
  - explain the concrete next action you are taking under this skill
  - ask for the single missing input that blocks execution
- If the task concerns bootstrap, validation, governance, or readiness, name the exact command or status model that applies.
- Do not introduce unrelated maintenance scope or other meta-context unless the user explicitly asks for it.

## Forbidden response pattern

These are failures when this skill has been invoked:

- replying only that the skill was read
- paraphrasing the skill without applying it to the user request
- adding unrelated scope explanations about maintenance work or skill packaging
- claiming a workspace is ready when the skill only establishes that it is `bootstrapped` or structurally valid

## Required output contract

When this skill is invoked, the agent response must include these literal sections:

- `Status`
- `User command`
- `Workspace deployment touched`
- `Completed tasks`
- `Failed step`
- `Failure reason`
- `Next action`

Rules for those sections:

- `User command` must tell the user, literally and concretely, to run `bootstrap-from-prd` with `pm-orchestrator` and attach the PRD in the same request.
- `Workspace deployment touched` must describe what the skill changed in the deployed workspace or say `none yet` if no workspace files were changed.
- If bootstrap ran, `Workspace deployment touched` must mention the target workspace path and any generated or updated deployment artifacts that apply, including `.state/bootstrap-report.md`, `.state/bootstrap-manifest.txt`, and `.github/workspace-capabilities.yaml`.
- `Completed tasks` must list only the steps that actually finished.
- If the workflow failed or stopped early, `Failed step` and `Failure reason` are mandatory and must be specific.
- If the workflow completed successfully, set `Failed step: none` and `Failure reason: none`.
- `Next action` must point to the next safe step, not a generic closing sentence.

## Required command wording

When the user needs the next command to continue product creation from a PRD,
the response must include this instruction in substance, without paraphrasing it
away:

`Run bootstrap-from-prd with pm-orchestrator and attach the PRD file in the same request.`

If the workspace has not been bootstrapped yet, also state that the workspace
must be bootstrapped first and use the bootstrap command from this skill.

## Required examples

### Good output after successful workspace bootstrap

```text
Status: done
User command: Run bootstrap-from-prd with pm-orchestrator and attach the PRD file in the same request.
Workspace deployment touched: Bootstrapped workspace at <target>; wrote .state/bootstrap-report.md, .state/bootstrap-manifest.txt, and .github/workspace-capabilities.yaml.
Completed tasks:
- preflight dependency detection completed
- workspace bootstrap completed
- structural validation completed
Failed step: none
Failure reason: none
Next action: Read .state/bootstrap-report.md, confirm Governance status and Readiness status, then run bootstrap-from-prd with pm-orchestrator and attach the PRD.
```

### Good output after partial or failed execution

```text
Status: partial
User command: Run bootstrap-from-prd with pm-orchestrator and attach the PRD file in the same request.
Workspace deployment touched: Bootstrapped workspace at <target>; wrote .state/bootstrap-report.md and .github/workspace-capabilities.yaml; .state/bootstrap-manifest.txt was not completed.
Completed tasks:
- preflight dependency detection completed
- target workspace creation completed
Failed step: bootstrap workspace manifest finalization
Failure reason: sqlite3 was unavailable and the workflow stopped before all post-bootstrap steps completed.
Next action: Review .state/bootstrap-report.md, resolve the blocking dependency or rerun bootstrap in degraded mode if supported, then continue with bootstrap-from-prd using the PRD attachment.
```

### Bad output

```text
I read the skill and understand it.
```

## State vocabulary

Use these labels consistently:

- `template`: content inside the bootstrap template
- `bootstrapped`: workspace generated locally but still pending local configuration
- `configured`: placeholders removed and governance configured locally
- `production-ready`: configured workspace with the optional `enterprise` overlay enabled and externally validated

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

- `prd-to-product-agents-cli validate package`: portable skill package integrity for the distributed skill
- `prd-to-product-agents-cli validate all`: maintainer validation from a source checkout, including runtime smoke
- `prdtp-agents-functions-cli validate workspace`: structural validation of a generated workspace
- `prdtp-agents-functions-cli validate governance`: governance validation for configured workspaces
- `prdtp-agents-functions-cli validate readiness`: optional enterprise-overlay readiness validation for production-ready workspaces

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

## Post-bootstrap independence

After bootstrap succeeds, treat the generated workspace as its own operational
surface.

- Runtime work should use the deployed workspace docs and runtime CLI.
- Normal day-to-day workspace operation should continue from the deployed files.
- If a task is outside bootstrap or workspace operation, name that scope explicitly instead of inferring it from runtime work.

## Source-of-truth split

Keep the contracts separated:

- `SKILL.md` is the source of truth for the packaged skill bootstrap and maintenance contract.
- `templates/workspace/docs/runtime/README.md` is the source of truth for deployed-workspace runtime operation.
- Other docs may summarize or reference those contracts, but they must not introduce stronger behavioral claims than those two sources.

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
prd-to-product-agents-cli --skill-root <skill-root> bootstrap workspace --target <workspace>
```

Useful variants:

```shell
prd-to-product-agents-cli --skill-root <skill-root> bootstrap workspace --target <workspace> --preflight-only
prd-to-product-agents-cli --skill-root <skill-root> bootstrap workspace --target <workspace> --dry-run
prd-to-product-agents-cli --skill-root <skill-root> validate package
prd-to-product-agents-cli --skill-root <skill-root> validate all
```

## Post-bootstrap checks

After bootstrap:

1. Read `.state/bootstrap-report.md`.
2. Confirm `Structure validation`, `Governance status`, and `Readiness status`.
3. Treat `bootstrapped` plus `not_ready` as expected until local governance is configured.
4. Use runtime docs inside the generated workspace for day-to-day operation.
