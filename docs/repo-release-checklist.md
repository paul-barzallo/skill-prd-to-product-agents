# Repository Release Checklist

Use this checklist before publishing or promoting a new repository release of
the `prd-to-product-agents` skill.

## 1. Version and contract integrity

- Confirm the root `VERSION` file is current for the repository release you are preparing.
- Confirm packaged artifacts and their documentation still match the current release contract when packaging changed.
- Review recent Markdown edits for drift inside repository-level documentation and release guidance.

## 2. Project CLI validation

Shorthand:

- Run `skill-dev-cli --skill-root <repo-or-skill-root> test repo-validation` as the GitHub-aligned local validation command before release work.
- Use `skill-dev-cli --skill-root <repo-or-skill-root> test workflow-release-gate` when you specifically need the current-platform simulation of the build workflow gate.

- Run `skill-dev-cli --skill-root <repo-or-skill-root> test unit`.
- Run `cargo test --manifest-path cli-tools/project-memory-cli/Cargo.toml` when the current release work touches `project-memory-cli`.
- Run `skill-dev-cli --skill-root <repo-or-skill-root> test markdown`.
- Run `skill-dev-cli --skill-root <repo-or-skill-root> test smoke`.
- Run `skill-dev-cli --skill-root <repo-or-skill-root> test workflow-release-gate` to simulate the build workflow release-gate on your current platform.
- Run `skill-dev-cli --skill-root <repo-or-skill-root> test release-gate` for
  the aggregated blocking chain before release tagging.
- Ensure `test release-gate` fails on:
  - runtime workspace contract regressions
  - stale `.agent.md` files
  - prompt encoding issues
  - orphan/legacy tracked artifacts
  - packaging drift
- If runtime governance or readiness changed, run `cargo test --manifest-path cli-tools/prdtp-agents-functions-cli/Cargo.toml` and confirm the new typed validators (`validate pr-governance`, `validate release-gate`) still pass their negative and positive coverage.
- If runtime audit, operating profiles, or GitHub wrappers changed, confirm `audit sink health/test`, typed state lifecycle tests, and the `core-local` versus `enterprise` contract are reflected in the same release.

## 3. Packaged artifact validation

- Run `prd-to-product-agents-cli --skill-root <repo-or-skill-root> validate all`.
- Confirm `validate all` checks package integrity, template encoding, and agent assembly consistency.
- Run `prd-to-product-agents-cli --skill-root <repo-or-skill-root> validate package-hygiene`.
- Run `prd-to-product-agents-cli --skill-root <repo-or-skill-root> validate platform-claims`.
- Run `prd-to-product-agents-cli --skill-root <repo-or-skill-root> validate version-metadata` to verify the root project `VERSION` is readable and present.

## 4. Packaging and scope review

- Confirm project-level binaries and docs remain coherent with the repository release contract.
- Treat these as the only tracked published binary scopes:
  - `bin/` for `skill-dev-cli`
  - `.agents/skills/prd-to-product-agents/bin/` for `prd-to-product-agents-cli`
  - `.agents/skills/prd-to-product-agents/templates/workspace/.agents/bin/prd-to-product-agents/` for `prdtp-agents-functions-cli`
- Confirm `bin/` contains only publishable binaries and that legacy artifacts
  such as ad hoc ZIPs, `target/`, or orphan manifests are excluded from release.
- Confirm the publish path still goes through a reviewed PR and not a direct push of tracked binaries to `main`.
- The only supported refresh path for tracked binaries is `.github/workflows/build-skill-binaries.yml`:
  `build -> test -> release-gate -> publish PR`.
- Do not hand-refresh tracked binaries, `checksums.sha256`, `sbom.spdx.json`, or
  `provenance-policy.json` from a local workstation as a publish path.
- Local binary rebuilds are diagnostic only and must not become the published
  state of tracked artifacts.

## 5. Release readiness review

- Review any maintainer-facing command or flag changes added in Rust code.
- Review generated binary names or supported platform claims if build outputs changed.
- For releases that touch runtime governance, readiness, or the operational contract, run the manual workflow `.github/workflows/enterprise-readiness-sandbox.yml` or execute the equivalent steps from `.agents/skills/prd-to-product-agents/templates/workspace/docs/runtime/enterprise-readiness-sandbox.md`.
- Do not approve release if the sandbox evidence is missing and the change affects `validate readiness`, governance configuration, release-gate semantics, or execution-boundary enforcement.
- Confirm `.github/workflows/dependency-review.yml` still covers dependency review and `cargo deny`, and do not release if that gate is broken or silently bypassed.
- Confirm `.github/workflows/build-skill-binaries.yml` still emits CI build provenance attestation for non-PR publication runs.
- Confirm the publish step refreshes `checksums.sha256`, `sbom.spdx.json`, and `provenance-policy.json` for every published bundle scope.
- Treat missing attestation, broken checksums, SBOM drift, provenance-policy drift, or undocumented binary refresh steps as release blockers for `production-ready` claims.

## 6. Publish decision

- If any blocking validation fails, do not release.
- If docs and code disagree, fix the docs before release.
- If the build workflow proposes a tracked binary refresh PR, review and merge that PR instead of pushing binary updates directly to `main`.
- Treat any attempt to bypass the workflow PR path for tracked binaries as a release-process failure, not as an acceptable exception.
- Record the release decision through the normal repository process after all
  validation steps pass.
