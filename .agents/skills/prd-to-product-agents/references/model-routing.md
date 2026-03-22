
# Model Routing Policy

This reference defines the canonical model-routing contract for the workspace.

## Goals

- Keep agent model selection explicit and reviewable.
- Use only official GA model names.
- Separate availability fallback from task-size escalation.
- Keep GitHub.com behavior honest: it ignores `model:`.

## Canonical policy file

The machine-readable source of truth is:

`templates/workspace/.github/agent-model-policy.yaml`

Validation scripts read that file and compare it against:

- `identity/*.md`
- assembled `.agent.md` files
- prompt overrides in `.github/prompts/*.prompt.md`

## Allowed models for v1

| Model | Why it is allowed |
| ----- | ----------------- |
| `Claude Opus 4.5` | Deep implementation, complex refactors, multi-file debugging |
| `GPT-4.1` | Strong generalist for coordination, architecture, QA, and DevOps |
| `Gemini 2.5 Pro` | Secondary quality axis for frontend and UX work |
| `Claude Haiku 4.5` | Short, bounded, low-ambiguity tasks |

## Disallowed names for v1

These names are rejected by validation:

- `GPT-4`
- `Gemini 3.1 Pro`
- `Gemini 3 Flash`

Reason: this skill uses only official GA names that are stable and documented in the current VS Code / Copilot environment.

## Agent default routing

`model:` in agent frontmatter is an ordered fallback list.

It does **not** mean:

- "always use the most expensive model"
- "rotate models manually per turn"
- "GitHub.com will obey the same order"

It **does** mean:

- if the first model is unavailable, the next entry is the approved fallback
- the order is part of the contract and can be linted

## Prompt-level escalation

Task-size or ambiguity changes are handled with prompt overrides, for example:

- `small-backend-change.prompt.md`
- `small-frontend-change.prompt.md`
- `deep-architecture-analysis.prompt.md`
- `release-incident-analysis.prompt.md`
- `small-doc-update.prompt.md`

This keeps the default agent identity stable while still allowing cheaper or deeper routes when the work pattern changes.

## Platform rule

- **VS Code / IDE:** `model:` is active and part of the agent/prompt contract.
- **GitHub.com:** GitHub.com ignores `model:`. Treat GitHub.com as degraded mode for model routing.

## Linux / WSL tooling note

For Linux and WSL validation, install Node.js natively and then install `markdownlint-cli` globally:

```bash
curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -
sudo apt-get install -y nodejs
sudo npm install -g markdownlint-cli
```

Validation warns when `node`, `npm`, or `markdownlint` resolve through `/mnt/c/...` instead of native Linux paths.
