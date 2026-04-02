# Runtime Claims Coverage

This matrix defines the strongest runtime claims the generated workspace is
allowed to make and the evidence expected for each one.

Do not strengthen runtime wording beyond this table unless the evidence source
is updated in the same change.

| Claim | Evidence | Boundaries |
| --- | --- | --- |
| `core-local` is the primary validated operating path. | `prdtp-agents-functions-cli validate workspace`, `validate governance`, and the workspace-portable `validate ci` helpers. | This is the default delivered contract. It does not imply remote GitHub governance or remote audit enforcement. |
| Supported enterprise auth mode: `token-api` only. | `prdtp-agents-functions-cli governance configure`, `validate governance`, and `validate readiness` all require `github.auth.mode=token-api` for `operating_profile=enterprise`. | Other enterprise auth modes are out of the current supported contract until a real end-to-end implementation exists. |
| `enterprise` production-ready claims require `validate readiness`, `governance provision-enterprise`, and `audit sink test`. | `prdtp-agents-functions-cli governance provision-enterprise`, `validate readiness`, `audit sink health`, and `audit sink test` returning a non-empty `ack_id`. | The claim is valid only for `operating_profile=enterprise`, `audit.mode=remote`, real reviewer logins, and a reachable remote audit sink. It does not by itself imply immutable retention or a cryptographic receipt. |
| Enterprise proof is repeatable rather than rhetorical. | The publisher-maintained enterprise sandbox workflow bootstraps an isolated packaged skill copy, records package validation, and uploads a reviewable evidence artifact, plus the manual runbook in `enterprise-readiness-sandbox.md`. | This is release evidence from the publishing repository, not a default workspace-local guarantee. |
| Release and immutable governance approvals are threshold-based. | `prdtp-agents-functions-cli validate governance`, `validate pr-governance`, `validate release-gate`, and the `github.*.approval_quorum` fields in `.github/github-governance.yaml`. | Missing quorum fields default to `1` for backward compatibility. Stronger dual control requires higher quorum plus enough distinct reviewer logins to satisfy it. |
| Execute tables define an intended role call set, not a hard runtime sandbox. | Prompt frontmatter, `validate ci prompt-tool-contracts`, `validate ci copilot-runtime-contract`, CODEOWNERS, PR governance, and immutable-governance review gates. | Runtime containment still depends on platform support and repository governance. No technical role broker is in scope for this P0. Do not describe the table as arbitrary-shell enforcement. |
| GitHub.com support is degraded best-effort, not parity with VS Code complete mode. | `runtime-platform-compatibility.md` plus `validate ci copilot-runtime-contract`. | The workspace may run there, but multi-agent routing and layered instruction behavior are not equivalent. |
