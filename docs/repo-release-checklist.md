# Repository Release Checklist

Use this checklist before publishing or promoting a new repository release of
the `prd-to-product-agents` skill.

## 1. Version and contract integrity

- Confirm `VERSION` matches the `skill-version:` marker in `SKILL.md`.
- Confirm packaged artifacts and their documentation still match the current release contract when packaging changed.
- Review recent Markdown edits for drift inside repository-level documentation and release guidance.

## 2. Project CLI validation

- Run `skill-dev-cli --skill-root <repo-or-skill-root> test unit`.
- Run `skill-dev-cli --skill-root <repo-or-skill-root> test markdown`.
- Run `skill-dev-cli --skill-root <repo-or-skill-root> test smoke`.
- Run `skill-dev-cli --skill-root <repo-or-skill-root> test release-gate` for
  the aggregated blocking chain before release tagging.
- Ensure `test release-gate` fails on:
  - stale `.agent.md` files
  - prompt encoding issues
  - orphan/legacy tracked artifacts
  - packaging drift

## 3. Packaged artifact validation

- Run `prd-to-product-agents-cli --skill-root <repo-or-skill-root> validate all`.
- Confirm `validate all` checks package integrity, template encoding, and agent assembly consistency.
- Run `prd-to-product-agents-cli --skill-root <repo-or-skill-root> validate package-hygiene`.
- Run `prd-to-product-agents-cli --skill-root <repo-or-skill-root> validate platform-claims`.
- Run `prd-to-product-agents-cli --skill-root <repo-or-skill-root> validate skill-version`.

## 4. Packaging and scope review

- Confirm project-level binaries and docs remain coherent with the repository release contract.
- Confirm `bin/` contains only publishable binaries and that legacy artifacts
  such as ad hoc ZIPs, `target/`, or orphan manifests are excluded from release.

## 5. Release readiness review

- Review any maintainer-facing command or flag changes added in Rust code.
- Review generated binary names or supported platform claims if build outputs changed.

## 6. Publish decision

- If any blocking validation fails, do not release.
- If docs and code disagree, fix the docs before release.
- Record the release decision through the normal repository process after all
  validation steps pass.
