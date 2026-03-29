
# Runtime Documentation

Documentation for the workspace runtime CLI (`prdtp-agents-functions-cli`)
and operational contracts for generated workspaces.

All runtime CLI examples assume an explicit workspace root via
`--workspace <path>`.

This directory is the source of truth for deployed-workspace runtime behavior.
Package-level docs may reference it, but they must not add stronger runtime
claims than the documents listed here.

## Contents

- [prdtp-agents-functions-cli-reference.md](prdtp-agents-functions-cli-reference.md) - Full command reference.
- [context-system-runtime.md](context-system-runtime.md) - Files-first context retrieval system, derivative surfaces, and recovery rules.
- [runtime-operations.md](runtime-operations.md) - Operational summary and command overview.
- [runtime-error-recovery.md](runtime-error-recovery.md) - Error recovery procedures.
- [enterprise-readiness-sandbox.md](enterprise-readiness-sandbox.md) - Manual and CI-backed sandbox readiness evidence for enterprise review.
- [runtime-platform-compatibility.md](runtime-platform-compatibility.md) - Cross-platform evidence.
- [capability-contract.md](capability-contract.md) - Workspace capability contract and degraded modes.
- [state-sync-design.md](state-sync-design.md) - State sync and audit ledger design.
- [context-freshness-runtime.md](context-freshness-runtime.md) - Context freshness remediation steps.
