# PMEM-013 - Add local microservice provider and localhost-only contract

## Type
Feature

## Priority
P0

## Objective
Support a local microservice as the main semantic backend while keeping the repository default local, safe, and no-cost.

## Context
The preferred product direction is an internal microservice before external provider dependencies. That requires a stable loopback-oriented HTTP contract for embedding requests and responses, plus basic operational expectations around timeout, batching, and health checks.

## Scope
- `local_microservice` provider implementation
- loopback-only endpoint contract and validation
- basic batching, timeout, and health-check behavior

## Out Of Scope
- external OpenAI or Azure requests
- generalized service discovery or orchestration

## Dependencies
- PMEM-011
- PMEM-012

## Technical Tasks
- define the HTTP request and response contract for embeddings
- enforce loopback-only endpoint validation by default
- add timeout handling, basic batching, and a health-check path or equivalent readiness probe
- add repository tests with a local mock server

## Acceptance Criteria
- ingest and retrieve can use a local microservice provider without changing the repository default
- loopback-only validation rejects accidental non-local exposure
- tests cover happy path and error handling

## Risks
- a vague HTTP contract will make client and service implementations drift quickly

## Definition Of Done
- `local_microservice` works in repository tests with a local mock server
- the contract and safety boundary are documented

## Expected Artifacts
- `cli-tools/project-memory-cli/src/embeddings.rs`
- `cli-tools/project-memory-cli/src/config.rs`
- `cli-tools/project-memory-cli/tests/project_memory_e2e.rs`
- `cli-tools/project-memory-cli/README.md`
- `docs/project-memory-cli-reference.md`