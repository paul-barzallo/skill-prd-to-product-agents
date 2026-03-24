# ADR-0003 Scope Boundaries Between Repo, Skill, And Workspace

- status: accepted

## Context

This repository is the source of truth for three related but different
surfaces:

1. the project repository used to maintain and release the skill
2. the packaged skill surface shipped from `.agents/skills/prd-to-product-agents/`
3. the deployed workspace copied from `templates/workspace/`

The source lives together, but the contracts do not. Earlier docs and tests
were at risk of mixing repository maintenance concerns with deployed-workspace
behavior, especially around `docs/project/`, runtime binaries, and validation
responsibilities.

## Decision

The repository will treat these as separate scopes with explicit ownership:

- The project repo owns maintenance docs, release workflows, project-scope
  binaries, and repository-only CLIs such as `skill-dev-cli` and
  `project-memory-cli`.
- The skill package owns the bootstrap CLI, package references, packaged
  binaries, and the workspace template source.
- The deployed workspace owns runtime instructions, runtime binaries under
  `.agents/bin/`, canonical product memory under `docs/project/`, and
  operational workflows after bootstrap.

The deployed workspace must remain operationally self-contained after
bootstrap. Repository documentation must not describe workspace runtime rules as
if they were repository policy, and workspace runtime docs must not require
knowledge of repository-maintenance tooling.

## Consequences

- Repository docs should mark workspace-only paths such as `docs/project/` as
  deployed-workspace concerns when they need to be mentioned.
- Skill and workspace docs should be reviewed together when bootstrap or
  packaging changes touch runtime behavior.
- Tests should prefer explicit scope resolution over hard-coded assumptions
  about the source repository layout.
- Validation documentation should say which commands are repository-only,
  skill-package, or deployed-workspace focused.

## Related docs

- `docs/architecture-map.md`
- `docs/current-status.md`
- `docs/open-gaps.md`
- `docs/known-limitations.md`
- `.agents/skills/prd-to-product-agents/README.md`