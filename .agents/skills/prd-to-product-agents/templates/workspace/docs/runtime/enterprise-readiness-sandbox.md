# Enterprise Readiness Sandbox

Use this runbook when a maintainer or reviewer needs repeatable evidence that a
workspace can satisfy the current `production-ready` gate against a real GitHub
repository.

## Current contract

- The supported execution layer is GitHub Issues, branches, commits, and PRs.
- `docs/project/board.md` is a derived issues/PR snapshot only.
- `github.project.enabled` stays `false`. GitHub Project metadata is reserved
  for a future extension and is out of the current supported contract.

## Sandbox prerequisites

Prepare a dedicated GitHub sandbox repository with:

- branch protection enabled for the protected branch pattern declared in `.github/github-governance.yaml`
- real reviewer identities for the release gate
- `gh auth status` working for the operator or CI identity
- permissions to read repository metadata and PR reviews

## Manual acceptance flow

1. Bootstrap a temporary workspace from the packaged skill release under review.
   This step happens outside the deployed workspace and must use the bootstrap
   command provided by that packaged release, targeting a fresh
   `<temp-workspace>` directory.

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
     --reviewer-infra <handle>
   ```

3. Edit `.github/github-governance.yaml` in the temporary workspace:
   - set `readiness.status=production-ready`
   - set `github.branch_protection.enabled=true`
   - keep `github.project.enabled=false`

4. Refresh local capability detection:

   ```text
   prdtp-agents-functions-cli --workspace <temp-workspace> capabilities detect
   ```

5. Validate the local configured gate first:

   ```text
   prdtp-agents-functions-cli --workspace <temp-workspace> validate governance
   ```

6. Validate the strong remote gate:

   ```text
   prdtp-agents-functions-cli --workspace <temp-workspace> validate readiness
   ```

## Expected evidence

Successful acceptance should prove all of these:

- `validate governance` passes after local reviewer and repository identifiers are real
- `validate readiness` passes only when the workspace is `production-ready`
- branch protection is visible remotely
- release gate reviewer logins are real and readable
- no GitHub Project dependency is required for a passing result

## Optional maintainer workflow

The maintainer repository that publishes the packaged skill may expose a manual
workflow for the same flow. Treat that as external release evidence, not as a
workspace-local capability.

## Failure interpretation

- If `validate governance` fails, the workspace is still locally incomplete.
- If `validate readiness` fails on `gh auth status`, the sandbox identity is not ready.
- If `validate readiness` fails on branch protection or reviewer logins, the remote GitHub controls are incomplete.
- If `validate readiness` fails because `github.project.enabled=true`, the workspace is outside the current supported contract.
