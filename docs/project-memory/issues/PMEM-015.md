# PMEM-015 - Add OpenAI-compatible provider bridge with Azure-first compatibility

## Type
Feature

## Priority
P0

## Objective
Implement one remote provider bridge for OpenAI-compatible endpoints, with Azure OpenAI treated as a priority compatibility mode rather than as a separate architecture.

## Context
The repository needs optional remote semantic retrieval, but the design should not split into unrelated provider trees too early. A single `openai_compatible` bridge can cover standard OpenAI-compatible APIs and Azure-specific variants through endpoint and deployment configuration.

## Scope
- remote provider bridge for OpenAI-compatible embedding APIs
- Azure-compatible request shaping and configuration fields
- provider integration through the existing provider abstraction

## Out Of Scope
- a fully separate Azure-only backend
- remote cost control or secret policy beyond the provider API bridge itself

## Dependencies
- PMEM-011
- PMEM-012
- PMEM-014

## Technical Tasks
- define the `openai_compatible` config shape
- support `base_url`, `deployment`, `api_version`, `model`, and `api_key_env`
- implement request/response handling for generic and Azure-compatible modes
- add HTTP mock tests for both compatibility paths

## Acceptance Criteria
- one provider bridge supports standard OpenAI-compatible endpoints and Azure-oriented variants
- Azure compatibility does not require a separate top-level provider architecture
- errors are explicit when required remote fields are missing

## Risks
- premature Azure-specific branching will fragment provider behavior and duplicate logic

## Definition Of Done
- `openai_compatible` works against repository tests and documented config examples
- the Azure-compatible path is validated without real credentials in CI

## Expected Artifacts
- `cli-tools/project-memory-cli/src/embeddings.rs`
- `cli-tools/project-memory-cli/src/config.rs`
- `cli-tools/project-memory-cli/tests/project_memory_e2e.rs`
- `cli-tools/project-memory-cli/README.md`
- `docs/project-memory-cli-reference.md`