# PMEM-016 - Add secret handling, remote safety gates, and spend protection

## Type
Feature

## Priority
P0

## Objective
Prevent accidental remote activation, secret leakage, and unbounded cost when remote semantic providers are enabled.

## Context
Once remote providers exist, configuration alone is not enough. The CLI must make external network use explicit, reject unsafe secret placement, and provide basic protections against accidental spend during local experimentation or automated runs.

## Scope
- explicit remote enablement gate
- secret-handling policy and config validation
- basic request-count or execution-budget protections

## Out Of Scope
- full billing integration or cloud account governance
- enterprise secret vault integrations beyond env var references

## Dependencies
- PMEM-014
- PMEM-015

## Technical Tasks
- require explicit remote enablement before any external request path runs
- reject secrets stored directly in `.project-memory/config.toml`
- support `api_key_env`-style secret references
- add request caps or equivalent spend-protection controls per execution
- make remote-network usage explicit in CLI diagnostics

## Acceptance Criteria
- remote providers cannot activate accidentally through a clone or default run
- config validation blocks direct secret storage patterns
- operators can see when a command used external network access

## Risks
- weak safety gates could create real cost or secret leakage from a repository-side tool

## Definition Of Done
- remote safety gates are enforced in code and documented
- tests cover blocked and allowed remote-provider scenarios

## Expected Artifacts
- `cli-tools/project-memory-cli/src/config.rs`
- `cli-tools/project-memory-cli/src/embeddings.rs`
- `cli-tools/project-memory-cli/src/query.rs`
- `cli-tools/project-memory-cli/tests/project_memory_e2e.rs`
- `docs/project-memory-cli-reference.md`