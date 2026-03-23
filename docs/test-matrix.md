# Test Matrix

This matrix explains what the main repository validation commands protect and
when each one should be used.

| Command | Use when | Blocks release | Protects | Notes |
| --- | --- | --- | --- | --- |
| `cargo test --manifest-path cli-tools/skill-dev-cli/Cargo.toml` | changing project maintenance CLI code | No, by itself | project CLI correctness | run for Rust changes affecting `skill-dev-cli` |
| `cargo test --manifest-path cli-tools/prd-to-product-agents-cli/Cargo.toml` | changing packaged CLI code | No, by itself | packaged CLI correctness | run for Rust changes affecting packaged artifacts |
| `cargo test --manifest-path cli-tools/prdtp-agents-functions-cli/Cargo.toml` | changing shared or packaged Rust code that touches runtime tooling | No, by itself | runtime CLI correctness from the repository side | repository task may still need this if shared code moved |
| `cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test markdown` | changing docs or markdown contracts | No, by itself | repository markdown quality and drift control | fastest required docs check |
| `cargo run --manifest-path cli-tools/prd-to-product-agents-cli/Cargo.toml -- --skill-root . validate all` | changing packaged artifacts, templates, prompts, bundles, or packaging claims | No, by itself | packaged artifact integrity | only required when current repo work affects packaged artifacts |
| `cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test smoke` | changing bootstrap-sensitive paths or wanting broader confidence | Indirectly | end-to-end repository smoke coverage | included inside release gate |
| `cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test workflow-release-gate` | wanting to simulate `.github/workflows/build-skill-binaries.yml` release-gate on the current platform | No, by itself | current-platform collected-binary release-gate behavior | closest local equivalent to the build workflow gate |
| `cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test repo-validation` | changing any path covered by `.github/workflows/repo-validation.yml` or wanting the GitHub-equivalent local check | No, by itself | repository validation workflow plus current-platform release-gate simulation | recommended local command before commit/push |
| `cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test release-gate` | changing structure, release behavior, packaging, or other release-sensitive areas | Yes | aggregated release confidence | canonical blocking release command |

## Coverage notes

- No single command replaces code review.
- `test release-gate` is the final repository gate, not the only useful check.
- `validate all` matters when the repository task changes packaged artifacts.
- Docs-only work should still run `test markdown` at minimum.

## Current gaps in coverage

- No single document currently maps validator coverage to specific failure modes beyond this matrix.
- Published Unix binaries still need explicit git executable bits and workflow hygiene to stay runnable across Unix CI jobs.
- The repository still lacks a dedicated remediation tracker for audit findings.
