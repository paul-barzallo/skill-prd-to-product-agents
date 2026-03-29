# Known Limitations

This file records repository-level limitations that should stay visible so they
do not get rediscovered, re-sold as solved, or confused with closed work.

## Validation limitations

- Repository validation is only as strong as the commands maintainers actually run; documentation alone does not enforce compliance.
- The GitHub workflow validates the repository on CI, but local maintenance still depends on contributors using the documented checks.
- Pre-commit hooks reduce drift but are not a substitute for CI or release-gate execution.

## Packaging limitations

- Published binary integrity is now checked through checksum manifests, SPDX SBOMs, provenance policy files, a reviewed refresh PR, dependency review, and CI build provenance attestation.
- Consumer-side attestation verification is strict for packaged consumption, but the repository source checkout still skips remote attestation verification intentionally so local maintainer development remains possible.
- Local Rust build outputs are still easy to confuse with shipped artifacts if hygiene slips.
- Release packaging discipline depends on maintainers respecting the documented boundaries around `bin/` and build outputs.

## Documentation limitations

- Repository docs can still drift if code changes are made without updating the corresponding maintainer references.
- Repository docs do not replace code and validator review when contract-sensitive behavior changes.
- Temporary audit knowledge now stays outside the repo, so durable follow-up still depends on maintainers summarizing the real conclusions back into stable docs.
- The repository, skill package, and deployed workspace still share source control, so boundary drift can reappear if scope labels are not kept explicit in docs and tests.

## Process limitations

- Maintainer support and escalation flow is still minimal even though the runbook and issue templates now exist.
- Binary refresh now routes through a reviewed PR, but reviewers still need to inspect tracked binaries, SBOMs, provenance policies, and checksums before merge.
- GitHub issue and PR mutation are wrapped by the runtime CLI, but the broader GitHub write surface is still intentionally narrower than the full `gh` CLI and must stay that way unless new wrappers, tests, and docs land together.
- Decision history is only now being formalized; some older choices still live mainly in code and scattered docs.

## Scope limitations

- Repository validation can prove consistency of the source repository, but it cannot fully prove that every downstream bootstrapped workspace will remain clean if users modify generated files manually.
- The skill package and deployed workspace remain intentionally related by source provenance, so documentation must keep saying when a statement is about repository maintenance, packaging, or runtime behavior.
