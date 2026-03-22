---
name: Bug report
about: Report a repository bug or maintenance regression
title: "bug: "
labels: [bug]
assignees: []
---

## Summary

- What is broken?
- Where does it happen?

## Impact

- User or maintainer impact:
- Release impact:

## Evidence

- Files affected:
- Commands affected:
- Logs or outputs:

## Reproduction

1.
2.
3.

## Expected behavior

-

## Actual behavior

-

## Validation status

- [ ] `cargo test --manifest-path cli-tools/skill-dev-cli/Cargo.toml`
- [ ] `cargo test --manifest-path cli-tools/prd-to-product-agents-cli/Cargo.toml`
- [ ] `cargo test --manifest-path cli-tools/prdtp-agents-functions-cli/Cargo.toml`
- [ ] `cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test markdown`
- [ ] `cargo run --manifest-path cli-tools/skill-dev-cli/Cargo.toml -- --skill-root . test release-gate`
- Notes:
