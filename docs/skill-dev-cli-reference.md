# skill-dev-cli Reference

**Purpose**: Project CLI for maintaining the `prd-to-product-agents` skill repository.

**Scope**: Project-only development checks. This binary owns smoke, unit, and markdown checks. It is not shipped in the skill package and it is not part of generated workspace deployment or workspace runtime operation.

**Binary**: `skill-dev-cli` in the project root `bin/` directory.

**Global flag**: `--skill-root <path>` is required. The CLI accepts either the skill root itself or the repository root that contains the skill.

---

## Commands

### `test smoke`

Run the end-to-end skill smoke suite. It covers preflight, dry run,
full bootstrap, workspace evidence files, skill package checks, and optional
workspace runtime CLI integration checks.

```bash
skill-dev-cli --skill-root <repo-or-skill-root> test smoke
```

### `test unit`

Run unit tests for project CLI helper behavior.

```bash
skill-dev-cli --skill-root <repo-or-skill-root> test unit
```

### `test markdown`

Run `markdownlint-cli` through the Rust CLI. By default, it uses the skill
`templates/workspace/.markdownlint.json` config and lints `**/*.md` relative to
the resolved skill root.

```bash
skill-dev-cli --skill-root <repo-or-skill-root> test markdown
skill-dev-cli --skill-root <repo-or-skill-root> test markdown --path "references/**/*.md"
```

### `test release-gate`

Run the aggregated release-blocking validation chain. The command stops at the
first failure and runs, in order:

1. `test unit`
2. `validate skill-version`
3. `validate package-hygiene`
4. `validate platform-claims`
5. `test smoke`

The effective blocking intent is:

- repository contract integrity
- package hygiene
- template encoding
- assembled agent consistency
- runtime smoke on a freshly bootstrapped workspace

`test unit` also rejects repository orphan artifacts that are outside the
declared contract, for example legacy tracked files that do not belong to the
project repo, skill package, or deployed workspace scopes.

```bash
skill-dev-cli --skill-root <repo-or-skill-root> test release-gate
skill-dev-cli --skill-root <repo-or-skill-root> test release-gate --target <temp-workspace>
```
