
# Workspace Binary Scope

This directory is reserved for workspace-local helper binaries inside generated projects.

- The workspace runtime CLI binaries live under `.agents/bin/prd-to-product-agents`.
- The skill bootstrap CLI remains under `.agents/skills/prd-to-product-agents/bin`.
- Workspace deployment must not depend on project-scope binaries.
