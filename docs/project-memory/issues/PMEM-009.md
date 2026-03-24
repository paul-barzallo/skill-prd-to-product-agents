# PMEM-009 - Document, test, and integrate project-memory-cli into repository validation

## Type
Task

## Priority
P0

## Objective
Make the MVP maintainable in this repository by adding documentation, fixtures, examples, and validation coverage.

## Context
This repository expects documentation and verification to land with the feature, not after it. The CLI should be discoverable and verifiable from the start.

## Scope
- add CLI README and usage examples
- add end-to-end tests with realistic fixtures
- update repository docs and validation guidance as needed

## Out Of Scope
- release publication of packaged binaries unless explicitly requested

## Dependencies
- PMEM-002
- PMEM-003
- PMEM-004
- PMEM-005
- PMEM-006

## Technical Tasks
- add `README.md` for the new CLI
- add E2E tests covering ingest, query, trace, validate, and impact
- update repository docs that describe the CLI landscape and validation matrix
- ensure required validation commands are documented for future contributors

## Acceptance Criteria
- a maintainer can discover the CLI and understand how to run it
- key command flows are covered by automated tests
- repository docs mention the new CLI where appropriate

## Risks
- missing docs or test coverage will cause the initiative to drift after merge

## Definition Of Done
- documentation and tests are merged with the MVP code
- repo-level docs reflect the new CLI accurately

## Expected Artifacts
- CLI README and examples
- E2E test coverage
- updated repo-level documentation
- `cli-tools/project-memory-cli/README.md`
- `cli-tools/project-memory-cli/tests/project_memory_e2e.rs`
- `cli-tools/skill-dev-cli/src/test_cmd.rs`
- `docs/project-memory-cli-reference.md`