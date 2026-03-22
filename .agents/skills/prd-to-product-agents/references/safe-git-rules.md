
# Safe Git Rules

## Goal

Commit only the files touched by bootstrap.

## Rules

- never use `git add .`
- use `.state/bootstrap-manifest.txt` as the allow-list
- if `.git` does not exist, run `git init`
- if there are unresolved collisions that require human review, do not auto-commit blindly
- if nothing changed, do not create an empty commit
