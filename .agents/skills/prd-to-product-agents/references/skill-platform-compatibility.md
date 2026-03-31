# Skill Platform Compatibility

Compatibility notes for the skill package, bootstrap CLI, and template contract.

## Agent frontmatter compatibility

| Property | VS Code / IDE | GitHub.com | Notes |
| --- | --- | --- | --- |
| `description` | supported | supported | required |
| `tools` | supported | supported | restricts tools when honored |
| `agents` | supported | ignored | delegation contract is IDE-oriented |
| `handoffs` | supported | ignored | safe to include |
| `model` | supported | ignored | GitHub.com chooses its own model |
| `user-invocable` | supported | ignored | safe to include |
| `disable-model-invocation` | supported | ignored | safe to include |

## Skill CLI evidence

Status vocabulary:

- `Verified` means the capability is exercised automatically in tests or CI.
- `Best-effort` means the surface exists but is not backed by the same end-to-end coverage on every platform.

| Capability / surface | Windows | Ubuntu/Linux | Evidence |
| --- | --- | --- | --- |
| `bootstrap workspace` | Best-effort | Best-effort | exercised in maintainer smoke and CI, but the distributed skill does not embed portable per-platform evidence artifacts |
| `validate package` | Best-effort | Best-effort | exercised in maintainer checks, bundle integrity, and Copilot contract checks, but not sealed inside the package as platform evidence |
| `validate all` | Best-effort | Best-effort | maintainer validation from source checkout, including runtime smoke; not a consumer-facing package proof |
| `bootstrap --skip-git` | Best-effort | Best-effort | exercised in maintainer smoke and CI, but not evidenced inside the published package |

## Honest compatibility statement

- VS Code / IDE execution is the primary multi-agent mode.
- GitHub.com remains a degraded surface for the orchestration layer.
- `model:` and handoff UI expectations are VS Code / IDE only.
- The package should not claim GitHub provisioning or full GitHub.com orchestration unless backed by implementation and CI evidence.

## Bootstrap dependencies

| Tool | Minimum version | Notes |
| --- | --- | --- |
| git | 2.30 | required for git-backed bootstrap paths |
| gh | 2.0 | optional; used only when enabled by local capability policy |
| sqlite3 | any | optional; degraded mode is supported |
| node / npm | current LTS recommended | optional for markdown tooling, not for core YAML validation |
