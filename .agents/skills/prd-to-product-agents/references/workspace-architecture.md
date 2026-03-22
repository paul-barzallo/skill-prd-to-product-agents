
# Workspace Architecture

This workspace separates four concerns:

1. **Agents** in `.github/agents/`
2. **Prompt workflows** in `.github/prompts/`
3. **Internal skills** in `.agents/skills/`
4. **Memory** split into canonical docs (operational truth) and a passive SQLite audit ledger (infrastructure-managed telemetry)

## Internal skills layer

The `.agents/skills/` tree is a packaged capability layer, not an extra agent pool.
It holds reusable workflow knowledge and release-time checks that must ship coherently with agents, prompts, and scripts.
This matters operationally because packaging drift in this layer can break real bootstrap and validation paths even when the top-level agent set is unchanged.

## Base agents

The workspace always uses these nine base agents:

- pm-orchestrator
- product-owner
- ux-designer
- software-architect
- tech-lead
- backend-developer
- frontend-developer
- qa-lead
- devops-release-engineer

## Why only nine

Keeping the model small reduces:

- context transfer overhead
- contradictory instructions
- handoff sprawl
- role ambiguity

## Critical authority rules

- software-architect designs, but does not command developers
- tech-lead is the only technical authority over backend/frontend developers
- qa-lead triages only to product-owner or tech-lead
- security is a workflow, not an agent
- internal skills are capability packs, not authorities in the handoff graph
- the system does not stop at go-live; post-release monitoring can reopen the cycle
