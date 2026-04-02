# Enterprise Readiness Sandbox

Use this manual maintainer runbook when a reviewer needs repeatable evidence
that a workspace can satisfy the optional `enterprise` overlay for the current
`production-ready` gate against a real GitHub repository.

This is not part of the default `core-local` path.

## Current contract

- The supported execution layer is GitHub Issues, branches, commits, and PRs.
- `docs/project/board.md` is a derived issues/PR snapshot only.
- `github.project.enabled` stays `false`. GitHub Project metadata is reserved
  for a future extension and is out of the current supported contract.

## Sandbox prerequisites

Prepare a dedicated GitHub sandbox repository with:

- branch protection enabled for the protected branch pattern declared in `.github/github-governance.yaml`
- real reviewer identities for the release gate
- a non-interactive GitHub API token available to the CLI as `PRDTP_GITHUB_TOKEN`, `GITHUB_TOKEN`, or `GH_TOKEN`; the supported enterprise path is `github.auth.mode=token-api`
- a reachable remote audit sink plus the auth header env var named in `audit.remote.auth_header_env`
- permissions to read repository metadata and PR reviews and to apply branch protection if provisioning is exercised

## Manual acceptance flow

1. Bootstrap a temporary workspace from the packaged skill release under review.
   This step happens outside the deployed workspace and must use the bootstrap
   command provided by that packaged release, targeting a fresh
   `<temp-workspace>` directory. The maintained publisher workflow first stages
   that packaged skill in an isolated temporary directory, validates the
   package there, and only then bootstraps the temporary workspace from that
   isolated copy.

2. Configure local governance skeleton values:

   ```text
   prdtp-agents-functions-cli --workspace <temp-workspace> governance configure \
     --owner <owner> \
     --repo <repo> \
     --release-gate-login <login> \
     --reviewer-product <handle> \
     --reviewer-architecture <handle> \
     --reviewer-tech-lead <handle> \
     --reviewer-qa <handle> \
     --reviewer-devops <handle> \
     --reviewer-infra <handle> \
       --reviewer-infra-login <login> \
     --operating-profile enterprise \
     --github-auth-mode token-api \
     --audit-mode remote \
     --audit-remote-endpoint <https-endpoint> \
     --audit-remote-auth-header-env <AUTH_HEADER_ENV>
   ```

    If enterprise policy requires more than one release-gate reviewer or explicit immutable-governance dual control, add `--release-gate-extra-logins`, `--release-gate-approval-quorum`, and `--immutable-governance-approval-quorum 2` intentionally.

3. Refresh local capability detection:

   ```text
   prdtp-agents-functions-cli --workspace <temp-workspace> capabilities detect
   ```

4. Validate the local configured gate first:

   ```text
   prdtp-agents-functions-cli --workspace <temp-workspace> validate governance
   ```

5. Provision the remote enterprise controls:

   ```text
   prdtp-agents-functions-cli --workspace <temp-workspace> governance provision-enterprise
   ```

6. Validate the strong remote gate:

   ```text
   prdtp-agents-functions-cli --workspace <temp-workspace> validate readiness
   ```

   In the published skill contract, `production-ready` is not promoted by a
   local helper. Treat it as an externally reviewed governance state that must
   already be present before the remote gate can pass.

7. Verify remote audit configuration before sending the probe:

   ```text
   prdtp-agents-functions-cli --workspace <temp-workspace> audit sink health
   ```

8. Prove the remote audit path accepts a live event:

   ```text
   prdtp-agents-functions-cli --workspace <temp-workspace> audit sink test
   ```

The current remote-audit proof is intentionally narrow. `audit sink test` must
receive a non-empty `ack_id` from the configured sink. That proves remote
acknowledgement for the probe event only; it does not by itself prove
immutable retention, independent timestamping, or a cryptographic receipt.

## Expected evidence

Successful acceptance should prove all of these:

- package validation succeeds against the isolated packaged skill candidate before bootstrap begins
- `validate governance` passes after local reviewer and repository identifiers are real
- bootstrap report and bootstrap manifest from the isolated bootstrap run are preserved with the evidence artifact
- `governance provision-enterprise` applies or confirms remote branch protection and governance labels
- `validate readiness` passes only when the workspace is `production-ready`
- branch protection is visible remotely
- release gate reviewer logins are real and readable
- `audit sink test` succeeds and returns a non-empty `ack_id` from the configured remote sink
- no GitHub Project dependency is required for a passing result

## Optional maintainer workflow

The maintainer repository that publishes the packaged skill may expose a manual
workflow for the same flow. Treat that as external release evidence, not as a
workspace-local capability or a default `core-local` guarantee. The maintained
publisher proof path is `.github/workflows/enterprise-readiness-sandbox.yml`,
which stages an isolated packaged skill copy, records package validation,
bootstraps from that staged copy, and uploads an evidence artifact for the
sandbox run.

## Failure interpretation

- If `validate governance` fails, the workspace is still locally incomplete.
- If package validation fails before bootstrap, the distributed skill candidate is not coherent enough to use as enterprise release evidence.
- If `validate readiness` fails on GitHub API identity, the sandbox token is missing, unreadable, or not valid for `token-api` use.
- If `governance provision-enterprise` fails, the remote repository is missing required permissions or branch targets.
- If `audit sink test` fails, or the response omits a non-empty `ack_id`, the remote audit sink is not ready for the current enterprise contract.
- If `validate readiness` fails on branch protection or reviewer logins, the remote GitHub controls are incomplete.
- If `validate readiness` fails because `github.project.enabled=true`, the workspace is outside the current supported contract.
