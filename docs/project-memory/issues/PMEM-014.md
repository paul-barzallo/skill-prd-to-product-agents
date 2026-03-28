# PMEM-014 - Add local project config file and remote-provider opt-in

## Type
Feature

## Priority
P0

## Objective
Introduce a local config surface for provider selection while keeping remote providers explicitly opt-in.

## Context
Provider selection and endpoint details should be configurable without hardcoding them into the repository or requiring interactive setup. The config path must remain local to the indexed project, avoid committed secrets, and work cleanly with flags and env var overrides.

## Scope
- `.project-memory/config.toml` as the local config surface
- provider, endpoint, model, timeout, and fallback configuration
- explicit remote-provider opt-in policy

## Out Of Scope
- secret storage inside the config file
- cloud-specific provider implementation details

## Dependencies
- PMEM-011
- PMEM-013
- PMEM-015

## Technical Tasks
- define the config schema and default file location
- wire config loading into CLI startup with documented precedence
- require explicit remote enablement before remote providers are used
- document unversioned local configuration expectations

## Acceptance Criteria
- a local config file can select provider behavior without changing tracked repository files
- flags and env vars override config deterministically
- remote providers remain disabled until explicitly enabled

## Risks
- weak config semantics could lead to accidental network use after a simple clone or local rerun

## Definition Of Done
- the config contract is implemented and documented
- tests cover precedence and opt-in behavior

## Expected Artifacts
- `cli-tools/project-memory-cli/src/config.rs`
- `cli-tools/project-memory-cli/src/cli.rs`
- `cli-tools/project-memory-cli/tests/project_memory_e2e.rs`
- `cli-tools/project-memory-cli/README.md`
- `docs/project-memory-cli-reference.md`