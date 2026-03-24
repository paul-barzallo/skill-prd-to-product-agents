# PMEM-006 - Add consistency and coverage validation over indexed memory

## Type
Feature

## Priority
P0

## Objective
Detect obvious consistency gaps and missing downstream coverage using the indexed and traced project memory.

## Context
Validation is one of the main reasons to build the CLI instead of relying on ad hoc agent prompting. Once ingest and trace exist, the system should report missing links and broken assumptions explicitly.

## Scope
- validate missing requirement coverage
- validate broken references and stale trace targets
- emit machine-readable findings with severity and evidence

## Out Of Scope
- domain-specific business rule engines
- speculative contradiction detection based on language models

## Dependencies
- PMEM-005

## Technical Tasks
- define validation finding schema
- implement missing coverage and stale-reference checks
- support JSON and non-zero exit behavior for failing conditions
- add tests for clean and failing fixture states

## Acceptance Criteria
- `validate` produces structured findings with evidence
- failing validation returns a non-zero exit code
- coverage gaps and broken references are distinguishable in output

## Risks
- noisy findings could reduce trust in the CLI quickly

## Definition Of Done
- validation rules exist for the core trace model
- failing cases are covered by automated tests

## Expected Artifacts
- validation module and finding schema
- validation fixtures and expected outputs
- `cli-tools/project-memory-cli/src/validate.rs`
- `cli-tools/project-memory-cli/src/model.rs`
- `cli-tools/project-memory-cli/tests/project_memory_e2e.rs`