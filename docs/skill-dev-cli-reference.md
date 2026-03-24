# skill-dev-cli Reference

**Purpose**: Project CLI for maintaining the `prd-to-product-agents` skill repository.

**Scope**: Project-only development checks. This binary owns smoke, unit, and markdown checks. It is not shipped in the skill package and it is not part of generated workspace deployment or workspace runtime operation.

The repository may still validate packaged skill sources because those sources
live here, but that does not make `skill-dev-cli` part of the packaged skill or
the deployed workspace runtime.

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

### `test workflow-release-gate`

Build the three CLIs for the current platform, stage them into a temporary
`collected/`-style directory, and run `test release-gate` from the staged
`skill-dev-cli` binary. This simulates the `release-gate` job from
`.github/workflows/build-skill-binaries.yml` on the current platform.

This command is included inside `test repo-validation`. Use it standalone when
you need to isolate the current-platform simulation of the GitHub build
workflow gate.

```bash
skill-dev-cli --skill-root <repo-or-skill-root> test workflow-release-gate
skill-dev-cli --skill-root <repo-or-skill-root> test workflow-release-gate --target <temp-workspace>
```

### `test repo-validation`

Run the repository validation workflow chain plus the current-platform release-gate
simulation from `.github/workflows/build-skill-binaries.yml`. This is the command
that local hooks should call before commit or push.

It is aligned with the Ubuntu repository validation workflow and adds the local
current-platform simulation of the build workflow release gate. It does not
replace GitHub's real multi-OS coverage.

The GitHub workflow runs when repository-maintenance paths or skill-package
source paths change because both are maintained in this repository. That trigger
scope is about source ownership, not about runtime dependency of the deployed
workspace.

It runs, in order:

1. `cargo test --manifest-path cli-tools/skill-dev-cli/Cargo.toml`
2. `cargo test --manifest-path cli-tools/prd-to-product-agents-cli/Cargo.toml`
3. `cargo test --manifest-path cli-tools/prdtp-agents-functions-cli/Cargo.toml`
4. `cargo test --manifest-path cli-tools/project-memory-cli/Cargo.toml`
5. `test markdown`
6. `prd-to-product-agents-cli --skill-root <repo-or-skill-root> validate all`
7. `test release-gate`
8. `test workflow-release-gate`

```bash
skill-dev-cli --skill-root <repo-or-skill-root> test repo-validation
skill-dev-cli --skill-root <repo-or-skill-root> test repo-validation --target <temp-workspace>
```

### `test release-gate`

Run the aggregated release-blocking validation chain. The command stops at the
first failure and runs, in order:

1. `test unit`
2. `cargo test --manifest-path cli-tools/prdtp-agents-functions-cli/Cargo.toml --test runtime_contract`
3. `validate version-metadata` for package `VERSION` metadata
4. `validate package-hygiene`
5. `validate platform-claims`
6. `test smoke`

The effective blocking intent is:

- repository contract integrity
- runtime workspace contract parity with CI
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
