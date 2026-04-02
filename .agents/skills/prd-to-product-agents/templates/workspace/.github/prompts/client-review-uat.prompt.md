---
description: Coordinate external review or UAT before release.
agent: pm-orchestrator
tools:
  - search
  - read
  - execute
  - edit/editFiles
---


# client-review-uat

## Purpose

Capture client or stakeholder feedback, update validation status and route rework or approval.

## Context scope

- `docs/project/scope.md`, `docs/project/releases.yaml`, `docs/project/releases.md`, `docs/project/refined-stories.yaml`, `docs/project/acceptance-criteria.md`
- `docs/project/backlog.yaml` for items ready for review
- `docs/project/handoffs.yaml` for pending handoffs

## Process

### 1. Identify reviewable items

Read canonical docs for operational state:

- Parse `docs/project/refined-stories.yaml` and `docs/project/backlog.yaml` to identify items ready for client review.
- Read `docs/project/releases.yaml` to identify releases pending client approval (status = `ready`).
- Read `docs/project/releases.md` only for narrative review notes or stakeholder context.

### 2. Collect feedback

For each reviewable item, record the client decision in `docs/project/releases.md` as narrative notes:

- Add a review section under the relevant release with: reviewer, result, notes, date.

Valid results: `approved`, `changes_requested`, `rejected`, `not_applicable`.

### 3. Route outcomes

| Client result | Action |
| -------------------- | ---------------------------------------------------------------------------------------- |
| `approved` | Run `state handoff create` -> `devops-release-engineer` with reason `ready_for_release` |
| `changes_requested` | Run `state handoff create` -> `product-owner` with reason `needs_refinement`; optionally run a second `state handoff create` -> `tech-lead` (type `rework`) if implementation rework is already clear |
| `rejected` | Run `state handoff create` -> `product-owner` with reason `scope_change`; run `state handoff create` -> `pm-orchestrator` only if coordination escalation is needed |
| `not_applicable` | Log review in releases.md, no further action |

Example - approved result:

```shell
prdtp-agents-functions-cli --workspace . state handoff create \
  --from-role     pm-orchestrator \
  --to-role       devops-release-engineer \
  --handoff-type  approval \
  --entity        "release/{release_ref}" \
  --reason        ready_for_release \
  --details       "Client approved all items for release"
```

Example - changes_requested result:

```shell
# Route refinement to product-owner
prdtp-agents-functions-cli --workspace . state handoff create \
  --from-role     pm-orchestrator \
  --to-role       product-owner \
  --handoff-type  normal \
  --entity        "US-007" \
  --reason        needs_refinement \
  --details       "Client requested changes to checkout flow"

# Optional rework handoff when implementation impact is already known
prdtp-agents-functions-cli --workspace . state handoff create \
  --from-role     pm-orchestrator \
  --to-role       tech-lead \
  --handoff-type  rework \
  --entity        "US-007" \
  --reason        needs_rework \
  --details       "Client requested implementation rework after review"
```

### 4. Update release status

If all items in a release are approved, do not mutate `releases.yaml` directly from this prompt. Route approval with `state handoff create` to `devops-release-engineer`; release status transitions remain a `devops-release-engineer` responsibility.

If changes were requested, mark affected items in `docs/project/refined-stories.yaml` as needing rework.

## Write

- Update `docs/project/releases.md` with review outcomes and supporting notes
- Run `prdtp-agents-functions-cli --workspace . state handoff create` for routing rework or approval
- Update `docs/project/refined-stories.yaml` when changes are requested
- Do NOT write YAML directly to `handoffs.yaml`, `findings.yaml`, or `releases.yaml` - always use `prdtp-agents-functions-cli --workspace . state *`

## Exit

Present results to the user with:

- **Task**: client review / UAT coordination
- **Status**: done | blocked | partial
- **Summary**: Up to 3 sentences covering review outcomes
- **Artifacts changed**: files created or modified
- **Decisions routed**: list of handoffs created (approval, rework, etc.)
- **Next recommendation**: suggested next step
