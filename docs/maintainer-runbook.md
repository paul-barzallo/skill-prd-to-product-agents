# Maintainer Runbook

This runbook is the practical operating guide for maintaining the repository.

These instructions are for the repository maintenance scope only. They do not
replace the operational guidance shipped inside the packaged skill or the
generated workspace.

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
cargo test --manifest-path cli-tools/project-memory-cli/Cargo.toml
```

If the change affects packaging, prompts, templates, or bundles, also run:

```bash
cargo run --manifest-path cli-tools/prd-to-product-agents-cli/Cargo.toml -- --skill-root . validate all
```

Scope reminder:

- `skill-dev-cli` and `project-memory-cli` are repository-only tools.
- `prd-to-product-agents-cli` validates the packaged skill surface.
- `prdtp-agents-functions-cli` validates the deployed-workspace runtime contract from the repository side.

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

If release automation, binary publication, dependency policy, or provenance
claims changed, review `.github/workflows/build-skill-binaries.yml`,
`.github/workflows/dependency-review.yml`, and `docs/repo-release-checklist.md`
together in the same change.

For changes under `cli-tools/**`, `.agents/skills/prd-to-product-agents/**`,
`bin/**`, or `.github/workflows/**`, GitHub now runs the multi-OS build and
release-gate workflow before merge in addition to the Ubuntu repository
validation workflow.

Also ensure local hooks are installed:

```bash
pre-commit install --hook-type pre-commit --hook-type pre-push
```

## 3. Packaging and binary hygiene

- `cli-tools/*/target/` and `cli-tools/*/target-staging/` are local build outputs.
- `bin/` is only for publishable project-scope binaries.
- Do not hand-edit binary bundles or checksum manifests unless you are intentionally performing release maintenance.
- The build workflow now proposes tracked binary refreshes through a PR; do not bypass that reviewed path with direct binary pushes to `main`.
- If build outputs changed, review `.github/workflows/build-skill-binaries.yml` and `docs/repo-release-checklist.md` together.
- `test repo-validation` is the local regression proof for release-doc/workflow drift and for published Unix executable-bit integrity.
- Published Unix binaries under `bin/`, `.agents/skills/prd-to-product-agents/bin/`, and `.agents/skills/prd-to-product-agents/templates/workspace/.agents/bin/prd-to-product-agents/` must stay `100755` in the git index.

## 4. Pull request expectations

Before opening a PR:

- use `.github/PULL_REQUEST_TEMPLATE.md`
- state what repo area changed
- record which validations were actually run
- note rollback risk if packaging or release behavior changed

## 5. Audits and findings

When a review or audit matters for future decisions:

1. keep the working notes outside the repo if they are temporary or exploratory
2. update `docs/current-status.md` if priorities or risks changed
3. update `docs/open-gaps.md` or `docs/known-limitations.md` if it exposed a real gap or durable limit
4. write an ADR if it closed a long-lived structural question
5. record any repository-contract change in `CHANGELOG.md`

For durable follow-up, use the repository issue templates instead of ad hoc
notes:

- `.github/ISSUE_TEMPLATE/audit-finding.md` for confirmed audit or review gaps
	that need tracked remediation
- `.github/ISSUE_TEMPLATE/release-regression.md` for release-gate, packaging,
	checksum, provenance, or publication-path regressions

When one of those issues is closed, copy the durable conclusion back into the
relevant source of truth (`docs/current-status.md`, `docs/open-gaps.md`,
`docs/known-limitations.md`, `CHANGELOG.md`, or an ADR) before considering the
follow-up complete.

## 6. Support and Escalation

- Use the normal issue templates for repository-scoped maintenance and release
	follow-up.
- Use `SECURITY.md` for sensitive reports or anything that should not be filed
	in a public issue.
- If a release path is blocked by workflow, binary, checksum, SBOM, or
	provenance drift, stop release work and file a `release-regression` follow-up.
- If a confirmed audit gap changes repository priorities or risks, record it in
	`docs/current-status.md` or `docs/open-gaps.md` in the same change that opens
	or closes the issue.

## 7. When to stop and ask for review

Stop and escalate if you find:

- docs contradicting release or validation behavior
- uncertainty about whether a change alters the repository contract
- unexpected binary or checksum drift
- changes that require a new ADR rather than an implicit code edit

## 8. End-of-change checklist

- [ ] documentation updated where claims changed
- [ ] required commands executed for the change type
- [ ] no local build garbage added as source content
- [ ] current status updated if priorities or risks moved
- [ ] audit or decision records updated when needed
