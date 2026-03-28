# PMEM-011 - Define provider architecture, config precedence, and security policy

## Type
Feature

## Priority
P0

## Objective
Define the provider contract and configuration policy before adding more semantic backends.

## Context
`project-memory-cli` already has retrieval infrastructure, but provider behavior must stay coherent as backends multiply. The architecture needs an explicit contract for local defaults, fallback semantics, config precedence, and secret handling so future provider work does not fragment the CLI surface.

## Scope
- provider taxonomy for `local_hashed_v1`, `local_microservice`, and `openai_compatible`
- configuration precedence across flags, env vars, config file, and safe defaults
- security policy for local-only default behavior and remote opt-in

## Out Of Scope
- implementing every provider backend in this issue
- cost accounting or request limiting details beyond the policy contract

## Dependencies
- PMEM-010

## Technical Tasks
- define the provider abstraction boundaries in code and docs
- define precedence as `flag > env > .project-memory/config.toml > safe default`
- define the default provider and the allowed fallback policy
- define secret-handling rules and remote enablement requirements

## Acceptance Criteria
- the repository documents one canonical provider policy
- local default behavior is explicit and no-cost
- the config precedence and secret rules are unambiguous

## Risks
- unclear precedence or security policy will create brittle behavior across future provider implementations

## Definition Of Done
- provider architecture is documented and reflected in the repository-side CLI contract
- the policy is referenced from the command docs and current-status docs

## Expected Artifacts
- `cli-tools/project-memory-cli/src/config.rs`
- `cli-tools/project-memory-cli/src/cli.rs`
- `cli-tools/project-memory-cli/README.md`
- `docs/project-memory-cli-reference.md`
- `docs/current-status.md`