# Known Limitations

This file records repository-level limitations that should stay visible so they
do not get rediscovered, re-sold as solved, or confused with closed work.

## Validation limitations

- Repository validation is only as strong as the commands maintainers actually run; documentation alone does not enforce compliance.
- The GitHub workflow validates the repository on CI, but local maintenance still depends on contributors using the documented checks.
- Pre-commit hooks reduce drift but are not a substitute for CI or release-gate execution.
- The packaged skill still governs `execute` through prompt/tool contracts and capability policy rather than a universal command sandbox; that boundary must remain explicit in enterprise claims.

## Packaging limitations

- Published binary integrity is now checked through checksum manifests, SPDX SBOMs, provenance policy files, a reviewed refresh PR, dependency review, and CI build provenance attestation.
- `prd-to-product-agents-cli validate package` is the portable consumer validation surface; `validate all` remains maintainer-only because it adds repository-scoped runtime smoke and version checks.
- Portable package validation now verifies local bundle materials and provenance-policy structure, but it does not perform mandatory remote attestation verification in the default consumer path.
- The packaged workspace template is maintained through source-side runtime commands such as agent assembly; those commands now log to temp for the packaged template, but future maintenance changes still need regression coverage so the distributed template does not silently regain runtime residue.
- Local Rust build outputs are still easy to confuse with shipped artifacts if hygiene slips.
- Release packaging discipline depends on maintainers respecting the documented boundaries around `bin/` and build outputs.
- `core-local` is intentionally not marketed as compliance-grade evidence: it keeps local hash-chained audit data, but only `enterprise` requires remote audit acknowledgement.

## Documentation limitations

- Repository docs can still drift if code changes are made without updating the corresponding maintainer references.
- Repository docs do not replace code and validator review when contract-sensitive behavior changes.
- Temporary audit knowledge now stays outside the repo, so durable follow-up still depends on maintainers summarizing the real conclusions back into stable docs.
- The repository, skill package, and deployed workspace still share source control, so boundary drift can reappear if scope labels are not kept explicit in docs and tests.

## Process limitations

- Maintainer support and escalation flow is still minimal even though the runbook and issue templates now exist.
- Binary refresh now routes through a reviewed PR, but reviewers still need to inspect tracked binaries, SBOMs, provenance policies, and checksums before merge.
- The published skill intentionally excludes GitHub issue and PR mutation wrappers; GitHub-connected runtime operations stay narrower than the full `gh` CLI, and docs must not imply those hidden maintainer-only paths are part of the shipped contract unless code, tests, and published binaries change together.
- Enterprise readiness still depends on real remote infrastructure: the maintained sandbox workflow can now produce evidence, but GitHub API credentials, remote branch protection targets, sandbox variables/secrets, and a reachable remote audit sink must still exist outside the repo for the strongest profile to be meaningful.
- Remote audit acknowledgement currently means only that the configured sink returned a non-empty `ack_id`; it does not by itself prove immutable retention, independent timestamping, or a cryptographic receipt.
- Immutable governance currently proves separate reviewer identities plus approval from one declared immutable-governance reviewer; it should not be described as formal dual-control unless validation is intentionally strengthened.
- Decision history is only now being formalized; some older choices still live mainly in code and scattered docs.

## Scope limitations

- Repository validation can prove consistency of the source repository, but it cannot fully prove that every downstream bootstrapped workspace will remain clean if users modify generated files manually.
- The skill package and deployed workspace remain intentionally related by source provenance, so documentation must keep saying when a statement is about repository maintenance, packaging, or runtime behavior.
