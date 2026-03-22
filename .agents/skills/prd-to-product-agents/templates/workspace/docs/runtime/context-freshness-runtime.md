
# Context Freshness - Runtime Remediation

When the skill validation detects stale agent context, these are the
runtime CLI steps to refresh it.

## What to do when stale

1. Run the appropriate `enrich-agents-from-*` prompt to update canonical context sources.
2. Run `prdtp-agents-functions-cli agents assemble` to regenerate `.agent.md` files from the updated sources.
3. Optionally, infrastructure can run `prdtp-agents-functions-cli audit sync` to update audit ledger checksums (agents do not run this directly).

## Freshness recovery sequence

| Canonical doc changed | Re-run prompt | Then run |
| ---------------------- | --------------- | -------- |
| `vision.md`, `scope.md`, `backlog.yaml`, `stakeholders.md`, `glossary.md` | `enrich-agents-from-prd` | `prdtp-agents-functions-cli agents assemble` |
| `architecture/overview.md`, `decisions/`, `refined-stories.yaml` | `enrich-agents-from-architecture` | `prdtp-agents-functions-cli agents assemble` |
| `refined-stories.yaml` (implementation_map) | `enrich-agents-from-implementation` | `prdtp-agents-functions-cli agents assemble` |

## Notes

- Context injection uses **versioned replace** semantics (full section replacement, not append).
- Freshness is a warning, not a blocking error. Agents can still operate with stale context.
- After assembly, optionally commit the updated `.agent.md` files via `prdtp-agents-functions-cli git finalize`.
