
# Context Freshness Policy (Skill Scope)

Defines when agent context is stale and needs re-injection.
This document covers the freshness detection and validation contract
owned by the skill CLI.

## Shared context file

`.github/agents/context/shared-context.md` is the shared context source for all 9 agents. It contains:

- **Project Context** (Layer 1) -- injected by `product-owner` via `enrich-agents-from-prd`
- **Technical Context** (Layer 2) -- injected by `software-architect` via `enrich-agents-from-architecture`

## When context is stale

Context is stale when canonical docs have changed since the last injection. The skill-root `prd-to-product-agents-cli validate` entrypoint compares SHA-256 checksums against the target workspace.

| Canonical doc changed | Context section affected | Re-run prompt |
| ---------------------- | ------------------------ | --------------- |
| `vision.md`, `scope.md`, `backlog.yaml`, `stakeholders.md`, `glossary.md` | Project Context | `enrich-agents-from-prd` |
| `architecture/overview.md`, `decisions/`, `refined-stories.yaml` | Technical Context | `enrich-agents-from-architecture` |
| `refined-stories.yaml` (implementation_map) | Implementation Context | `enrich-agents-from-implementation` |

## Freshness metadata

Each injected section includes a version comment:

```markdown
<!-- injected: 2025-01-15 by product-owner -->
```

The shared-context.md file also has:

```markdown
<!-- context-version: 2025-01-15 -->
```

## Validation checks

The skill-root `prd-to-product-agents-cli validate` entrypoint uses SHA-256 checksums for content-based freshness detection:

1. After context injection, run `prd-to-product-agents-cli validate generated --workspace <workspace-root>` to save baseline hashes to `.state/context-checksums.json`
2. On subsequent runs, that skill-root validation entrypoint computes current SHA-256 of `vision.md`, `scope.md`, `architecture/overview.md` and compares to stored hashes
3. Emits a **warning** (not error) if any canonical doc content has changed

## Token budgets

Assembled `.agent.md` files are injected into the LLM context window. Oversized files waste tokens and may crowd out user messages.

| Component | Max lines | Rationale |
| ----------- | ----------- | ----------- |
| `identity/{name}.md` | 250 | Personality, contracts, heuristics |
| `context/shared-context.md` | 150 | Shared across all 9 agents |
| `context/{name}.md` | 200 | Per-agent overlay |
| Assembled `.agent.md` | 500 | identity + divider + shared + overlay |

The skill-root validation entrypoint emits a **SIZE-BUDGET** warning when any assembled `.agent.md` exceeds 500 lines.

These are advisory limits. An agent that exceeds the budget still works but may lose user-message context in long conversations.

## Rules

- Freshness is a **warning**, not a blocking error.
- Agents can still operate with stale context -- it's the coordinator's responsibility to trigger re-injection.
- Context injection uses **versioned replace** semantics (full section replacement, not append).
- Token budgets are advisory -- skill-root validation warns but does not fail.
