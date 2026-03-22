
# System Design Reference

This is **reference material** for customizing agents, understanding
personality constraints, or troubleshooting post-bootstrap behavior.
For topics covered in other reference docs, see the cross-reference
table at the end.

## Agent personality model

Each agent has a defined personality that shapes its behavior,
phrasing and decision-making. The personality is a structural
constraint, not decoration.

### Personality sections (immutable after bootstrap)

| Section | Purpose |
| ------------------------ | ------------------------------------ |
| `## Personality` | Core character traits (OCEAN model) |
| `## Behavior contract` | Reads, Writes, Pre/Exit criteria |
| `## Decision heuristics` | Rules for choosing between actions |
| `## Anti-patterns` | Prohibitions -- refuse and route |
| `## Tone` | Communication style |
| `## Memory interaction` | Files and Git context (SQLite is passive infrastructure) |

### Personality design principles

- **pm-orchestrator**: Serene, neutral, flow-obsessed. Routes,
  never resolves.
- **product-owner**: Empathetic, value-driven, scope guardian.
  Speaks in outcomes.
- **ux-designer**: Creative, visual, empathy-driven. Thinks in
  flows and states.
- **software-architect**: Analytical, rigorous, trade-off obsessed.
  Always documents alternatives.
- **tech-lead**: Pragmatic, demanding, delivery-oriented.
  Developer shield.
- **backend-developer**: Methodical, defensive, edge-case obsessed.
  Boring, predictable code.
- **frontend-developer**: Creative but disciplined. State-aware,
  iterative.
- **qa-lead**: Skeptical, meticulous, uncompromising. Findings are
  facts.
- **devops-release-engineer**: Cautious, systematic, healthy
  paranoid. Checklist-driven.

### Immutability rule

The personality, behavior contract, decision heuristics and
anti-patterns sections are **never modified** by context injection
or by other agents. They are set at bootstrap and remain constant
for the life of the workspace.

## Markdown authoring rules

All generated Markdown files must pass `markdownlint` using the
default rules, unless a workspace-specific config is added.

Minimum rules:

- Wrap prose at 80 characters when practical.
- Surround headings with blank lines.
- Surround fenced code blocks with blank lines.
- Always declare a language for fenced code blocks.
- Use sequential ordered list numbering.
- Avoid trailing punctuation in headings.
- Keep table formatting consistent within the file.
- Prefer short paragraphs and bullets over long prose.

Preferred fence languages:

- `yaml` for frontmatter or config
- `powershell` for PowerShell commands
- `bash` for shell commands
- `sql` for SQL snippets
- `json` for JSON objects
- `text` for pseudo-output, trees, flow diagrams

## Cross-references

| Topic | See |
| ------- | ----- |
| 9-agent model, authority rules, handoff routes | `agent-flow.md` |
| Three-tier memory model | `memory-model.md` |
| Communication channels, state precedence | `agent-communication-model.md` |
| Context injection layers, agent assembly | `copilot-instructions.md` |
| Platform compatibility matrix | `skill-platform-compatibility.md` |
| Audit ledger sync mapping | `sync-mapping.md` |
| Context freshness and staleness | `context-freshness-skill.md` |
| Error recovery procedures | `skill-bootstrap-error-recovery.md` |
