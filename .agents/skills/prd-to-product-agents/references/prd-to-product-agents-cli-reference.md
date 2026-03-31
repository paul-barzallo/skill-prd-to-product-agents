<!-- markdownlint-disable MD012 -->

# prd-to-product-agents-cli Reference

**Purpose**: Skill CLI for bootstrap and validation. It creates new workspaces from templates,
validates skill artifacts, runs preflight environment detection, and manages
workspace dependency availability.

**Scope**: Bootstrap and skill package validation only. This CLI is not used
during daily workspace operation.

**Binary**: `prd-to-product-agents-cli` in the skill `bin/` directory.

**Global flag**: `--skill-root <path>` is required. Pass the packaged skill root.

---

## Commands

### `bootstrap workspace`

Create a new workspace from templates.

```bash
prd-to-product-agents-cli --skill-root <skill-root> bootstrap workspace --target <path>
```

Notes:

- `--target` is optional and defaults to the current directory if omitted.
- `--project-name`, `--github-owner`, and `--github-repo` override template
  values during bootstrap.
- `--skip-db-init`, `--skip-git`, `--dry-run`, and `--preflight-only` change
  bootstrap behavior without changing the skill package itself.

### `bootstrap commit`

Safe git commit of manifest-listed files after bootstrap.

```bash
prd-to-product-agents-cli --skill-root <skill-root> bootstrap commit --target <path>
```

### `validate package`

Run the portable validation surface for the packaged skill.

```bash
prd-to-product-agents-cli --skill-root <skill-root> validate package
```

This command is the supported package-consumer validation path. It does not
assume repository sources and does not require remote services.

### `validate all`

Run all maintainer-side validation checks.

```bash
prd-to-product-agents-cli --skill-root <skill-root> validate all
```

This command is maintainer-oriented. It assumes a source checkout is available
and includes repository-scoped runtime smoke validation in addition to the
portable package checks.

### `validate generated`

Validate generated workspace structure.

```bash
prd-to-product-agents-cli --skill-root <skill-root> validate generated --workspace <path>
```

Notes:

- `--workspace` is optional in the implementation; if omitted, validation runs
  against the current directory.
- `--record-checksums` writes `.state/context-checksums.json` for freshness
  tracking.

### `validate package-hygiene`

Check that the packaged skill contains no runtime artifacts.

```bash
prd-to-product-agents-cli --skill-root <skill-root> validate package-hygiene
```

### `validate platform-claims`

Validate platform compatibility claims.

```bash
prd-to-product-agents-cli --skill-root <skill-root> validate platform-claims
```

### `validate version-metadata`

Verify the root project release metadata is present and readable.

```bash
prd-to-product-agents-cli --skill-root <skill-root> validate version-metadata
```

This check is repository-scoped and is mainly useful to maintainers working
from a source checkout.

### `clean workspace`

Remove bootstrap-deployed artifacts per manifest.

```bash
prd-to-product-agents-cli --skill-root <skill-root> clean workspace --target <path>
```

### `preflight detect`

Detect environment capabilities and write `workspace-capabilities.yaml`.

```bash
prd-to-product-agents-cli --skill-root <skill-root> preflight detect --target <path>
```

### `preflight check`

Quick preflight capability check.

```bash
prd-to-product-agents-cli --skill-root <skill-root> preflight check
```

### `preflight deps`

Check workspace dependency availability. Supports auto-install, Git identity
configuration, and GitHub CLI authentication.

```bash
prd-to-product-agents-cli --skill-root <skill-root> preflight deps
prd-to-product-agents-cli --skill-root <skill-root> preflight deps --install
prd-to-product-agents-cli --skill-root <skill-root> preflight deps \
  --configure-git-identity local \
  --git-user-name "Name" \
  --git-user-email "email@example.com"
prd-to-product-agents-cli --skill-root <skill-root> preflight deps --start-gh-auth
```

Flags:

- `--install`: Attempt auto-install of missing dependencies.
- `--configure-git-identity <scope>`: Configure Git identity as `global` or
  `local`.
- `--git-user-name <name>`: Git user name. Required with
  `--configure-git-identity`.
- `--git-user-email <email>`: Git user email. Required with
  `--configure-git-identity`.
- `--start-gh-auth`: Launch `gh auth login` interactively.



