# Project Binaries

This directory is for repository-level binaries used to maintain the
`prd-to-product-agents` project itself.

## Scope

- `skill-dev-cli` belongs here because it is a project-only development tool.
- These binaries are not part of the generated workspace runtime contract.
- Packaged skill binaries live under
  `.agents/skills/prd-to-product-agents/bin/` instead.

## Boundary rules

- Do not document runtime commands here.
- Do not move workspace runtime binaries into this directory.
- Keep project maintenance tooling separate from shipped skill artifacts.
- Do not confuse this directory with `cli-tools/*/target/`, which contains only
  local build outputs and may be cleaned before handoff or packaging review.
