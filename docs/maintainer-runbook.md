# Maintainer Runbook

This runbook is the practical operating guide for maintaining the repository.

## 1. Before changing anything

Read these first:

- `README.md`
- `AGENTS.md`
- `docs/README.md`
- `docs/current-status.md`
- `docs/repo-release-checklist.md`

If the change is structural or affects release behavior, also read:

- `docs/architecture-map.md`
- `docs/open-gaps.md`
- `docs/known-limitations.md`
- `docs/decisions/README.md`

## 2. Common maintenance paths

### Docs-only change

Run:

```bash
cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test markdown
```

Also update related docs in the same change if a contract or claim moved.

### Rust change in one or more CLIs

Run tests for each affected crate:

```bash
cargo test --manifest-path cli-tools/skill-dev-cli/Cargo.toml
cargo test --manifest-path cli-tools/prd-to-product-agents-cli/Cargo.toml
cargo test --manifest-path cli-tools/prdtp-agents-functions-cli/Cargo.toml
```

If the change affects packaging, prompts, templates, or bundles, also run:

```bash
cargo run --manifest-path cli-tools/prd-to-product-agents-cli/Cargo.toml -- --skill-root . validate all
```

### Structural or release-sensitive change

Run:

```bash
cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test repo-validation
cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test workflow-release-gate
cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test release-gate
```

Use `test repo-validation` before commit or push when the touched paths overlap
the repository validation workflow. Use `test workflow-release-gate` when you
need the current-platform equivalent of the build workflow gate. Treat
`test release-gate` as the blocking repository release command inside that
broader local check.

Also ensure local hooks are installed:

```bash
pre-commit install --hook-type pre-commit --hook-type pre-push
```

## 3. Packaging and binary hygiene

- `cli-tools/*/target/` and `cli-tools/*/target-staging/` are local build outputs.
- `bin/` is only for publishable project-scope binaries.
- Do not hand-edit binary bundles or checksum manifests unless you are intentionally performing release maintenance.
- If build outputs changed, review `.github/workflows/build-skill-binaries.yml` and `docs/repo-release-checklist.md` together.

## 4. Pull request expectations

Before opening a PR:

- use `.github/PULL_REQUEST_TEMPLATE.md`
- state what repo area changed
- record which validations were actually run
- note rollback risk if packaging or release behavior changed

## 5. Audits and findings

When a review or audit matters for future decisions:

1. store or summarize it under `docs/audits/`
2. update `docs/current-status.md` if priorities or risks changed
3. update `docs/open-gaps.md` if it exposed a real gap
4. write an ADR if it closed a long-lived structural question

## 6. When to stop and ask for review

Stop and escalate if you find:

- docs contradicting release or validation behavior
- uncertainty about whether a change alters the repository contract
- unexpected binary or checksum drift
- changes that require a new ADR rather than an implicit code edit

## 7. End-of-change checklist

- [ ] documentation updated where claims changed
- [ ] required commands executed for the change type
- [ ] no local build garbage added as source content
- [ ] current status updated if priorities or risks moved
- [ ] audit or decision records updated when needed
