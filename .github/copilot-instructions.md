# Copilot Instructions for This Repository

These instructions apply to repository maintenance work for `prd-to-product-agents`.

## Primary objective

Keep repository maintenance, validation, and release work coherent at the
project root.

## Active root

For these instructions, the active root is the repository root.

- `docs/` is the maintainer documentation area.
- `cli-tools/skill-dev-cli/` is the project maintenance CLI area.
- `.github/` contains repository automation and review scaffolding.

These instructions do not govern the packaged skill surface or a bootstrapped
workspace unless the current repository task explicitly targets those scopes.

Do not expand these instructions into packaged or generated surfaces unless the
user explicitly asks for that scope.

## Source of truth

When in doubt, prefer the most local source of truth for the scope you are touching:

1. Rust code for actual behavior and command semantics.
2. Validation code and workflows for what is enforced.
3. Repository docs under `docs/` for maintainer process.
4. Root summaries and contributor guidance.

If docs and code disagree, fix the docs or the code in the same change.

## Files to consult before editing

- `README.md`
- `AGENTS.md`
- `docs/README.md`
- `docs/architecture-map.md`
- `docs/current-status.md`
- `docs/repo-release-checklist.md`

If the change touches packaging, also inspect:

- `bin/README.md`
- `.github/workflows/build-skill-binaries.yml`
- `.github/workflows/repo-validation.yml`

## Required validation by change type

### Markdown or docs

```bash
cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test markdown
```

### Skill package, template, prompts, bundles

```bash
cargo run --manifest-path cli-tools/prd-to-product-agents-cli/Cargo.toml -- --skill-root . validate all
```

Use this only when the current repository task explicitly affects packaged artifacts.

### Rust crates

Run tests for each affected crate:

```bash
cargo test --manifest-path cli-tools/skill-dev-cli/Cargo.toml
cargo test --manifest-path cli-tools/prd-to-product-agents-cli/Cargo.toml
cargo test --manifest-path cli-tools/prdtp-agents-functions-cli/Cargo.toml
cargo test --manifest-path cli-tools/project-memory-cli/Cargo.toml
```

### Structural or release-sensitive changes

```bash
cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test release-gate
```

Do not claim verification if the required checks were not run.

## Maintenance expectations

- Keep repository-level context in `docs/`.
- Record active work, blockers, and next actions in `docs/current-status.md`.
- Record historical reviews and assessments under `docs/audits/`.
- Avoid ad hoc root notes, duplicate summaries, or undocumented decisions.

## Packaging hygiene

- `cli-tools/*/target/` and `cli-tools/*/target-staging/` are local build outputs.
- `bin/` is for publishable project-scope binaries only.
- Do not hand-edit published binaries or checksum manifests unless the task is a release update.

## How to avoid re-opening settled decisions

Before changing architecture, packaging, or governance claims:

1. Check `docs/architecture-map.md`.
2. Check `docs/current-status.md` for active constraints.
3. Check `docs/audits/` for prior findings.
4. If a tradeoff needs to be made explicit, add or update an ADR later when the repo has a formal decisions directory.
