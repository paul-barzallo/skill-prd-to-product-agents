
# Refined Story Example

```yaml
stories:
  - id: US-001
    title: Obtain minimum data for a first quote
    status: ready
    priority: high
    owner_role: product-owner
    acceptance_ref: docs/project/acceptance-criteria.md#us-001
    refined_by:
      - product-owner
      - tech-lead
    dependencies: []
    functional_notes:
      - Ask only the minimum data needed for an initial quote.
    tech_notes:
      - Requires POST /quote/draft.
    edge_cases:
      - Ambiguous date
      - User changes destination mid-flow
    qa_notes:
      - Validate minimum interaction path
    implementation_map:
      - action: create
        path: src/modules/quotes/controller.ts
        description: Draft quote controller
      - action: modify
        path: src/core/db/schema.sql
        description: Add draft_sessions table
      - action: reuse
        path: src/shared/validators.ts
        description: Reuse common validators
```
