# Known Limitations

This file records repository-level limitations that should stay visible so they
do not get rediscovered, re-sold as solved, or confused with closed work.

## Validation limitations

- Repository validation is only as strong as the commands maintainers actually run; documentation alone does not enforce compliance.
- The GitHub workflow validates the repository on CI, but local maintenance still depends on contributors using the documented checks.
- Pre-commit hooks reduce drift but are not a substitute for CI or release-gate execution.

## Packaging limitations

- Published binary integrity is checked through checksum manifests, but that is not the same as full enterprise provenance or attestation.
- Local Rust build outputs are still easy to confuse with shipped artifacts if hygiene slips.
- Release packaging discipline depends on maintainers respecting the documented boundaries around `bin/` and build outputs.

## Documentation limitations

- Repository docs can still drift if code changes are made without updating the corresponding maintainer references.
- Repository docs do not replace code and validator review when contract-sensitive behavior changes.
- Audit knowledge is only beginning to be structured; historical context is not yet fully consolidated under `docs/audits/`.
- The repository, skill package, and deployed workspace still share source control, so boundary drift can reappear if scope labels are not kept explicit in docs and tests.

## Process limitations

- The repository still lacks a full maintainer runbook.
- The repository still lacks issue templates for turning findings into consistent tracked work.
- Decision history is only now being formalized; some older choices still live mainly in code and scattered docs.

## Scope limitations

- Repository validation can prove consistency of the source repository, but it cannot fully prove that every downstream bootstrapped workspace will remain clean if users modify generated files manually.
- The skill package and deployed workspace remain intentionally related by source provenance, so documentation must keep saying when a statement is about repository maintenance, packaging, or runtime behavior.
