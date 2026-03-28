
# Workspace Binary Scope

This directory is reserved for workspace-local helper binaries inside generated projects.

- The workspace runtime CLI binaries live under `.agents/bin/prd-to-product-agents`.
- These binaries are the runtime copy used by the deployed workspace after bootstrap.
- Workspace deployment must not depend on binaries outside the deployed workspace.
