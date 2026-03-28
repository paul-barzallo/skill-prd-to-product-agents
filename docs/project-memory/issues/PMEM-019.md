# PMEM-019 - Test matrix and repository validation for provider modes

## Type
Feature

## Priority
P0

## Objective
Extend repository validation so provider-backed retrieval remains testable without real credentials or external cost.

## Context
The provider initiative is only credible if the repository can validate local hashed, local microservice, and OpenAI-compatible paths in CI without relying on paid external services. The test matrix must cover provider modes, safety gates, Azure-compatible shaping, and fallback behavior.

## Scope
- provider-mode test coverage for the repository-side CLI
- repo-validation updates where needed
- mock-based coverage for OpenAI-compatible and Azure-compatible paths

## Out Of Scope
- live integration tests against paid external providers in default CI
- packaging or runtime workspace validation beyond repository-side tooling

## Dependencies
- PMEM-013
- PMEM-014
- PMEM-015
- PMEM-016
- PMEM-017
- PMEM-018

## Technical Tasks
- add tests for `local_hashed_v1`, `local_microservice`, and `openai_compatible`
- add mock coverage for Azure-compatible request shaping
- cover safety-gate failures and allowed remote paths
- update repository validation docs and matrix if command expectations change

## Acceptance Criteria
- CI can validate provider modes without real credentials
- Azure-compatible behavior is covered through mocks
- repository docs describe the validation expectations for provider-enabled retrieval

## Risks
- inadequate provider-mode testing will turn retrieval behavior into a drift-prone manual workflow

## Definition Of Done
- provider modes are covered by automated tests in the repository
- validation docs and test matrix reflect the provider wave accurately

## Expected Artifacts
- `cli-tools/project-memory-cli/tests/project_memory_e2e.rs`
- `cli-tools/project-memory-cli/README.md`
- `docs/project-memory-cli-reference.md`
- `docs/test-matrix.md`
- `cli-tools/skill-dev-cli/src/test_cmd.rs`