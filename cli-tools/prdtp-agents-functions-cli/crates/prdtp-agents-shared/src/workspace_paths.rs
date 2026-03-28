//! Single source of truth for generated-workspace file paths and agent names.
//!
//! Both the bootstrap CLI (`prd-to-product-agents-cli`) and the functions CLI
//! (`prdtp-agents-functions-cli`) should reference these constants rather than
//! maintaining their own lists. These constants describe the deployed-workspace
//! contract only; they are not a repository-maintenance file inventory.

/// Files that MUST exist after a successful bootstrap.
pub const REQUIRED_FILES: &[&str] = &[
    "AGENTS.md",
    ".instructions.md",
    ".gitignore",
    ".gitattributes",
    ".github/workspace-capabilities.yaml",
    ".github/github-governance.yaml",
    ".github/agent-model-policy.yaml",
    ".github/copilot-instructions.md",
    ".github/immutable-files.txt",
    "docs/project/vision.md",
    "docs/project/scope.md",
    "docs/project/stakeholders.md",
    "docs/project/backlog.yaml",
    "docs/project/refined-stories.yaml",
    "docs/project/acceptance-criteria.md",
    "docs/project/handoffs.yaml",
    "docs/project/findings.yaml",
    "docs/project/releases.yaml",
    "docs/project/risks.md",
    "docs/project/management-dashboard.md",
    "docs/project/architecture/overview.md",
    "docs/project/ux/journeys.md",
];

/// Additional files expected by the functions CLI `validate workspace`.
pub const EXTENDED_REQUIRED_FILES: &[&str] = &[
    "docs/project/board.md",
    "docs/project/glossary.md",
    "docs/project/open-questions.md",
    "docs/project/context-summary.md",
    "docs/project/change-log.md",
    "docs/project/quality-gates.yaml",
    "docs/project/source-of-truth-map.md",
    "docs/project/releases.md",
    "schemas/handoffs.schema.yaml",
    "schemas/findings.schema.yaml",
    "schemas/releases.schema.yaml",
    ".agents/bin/prd-to-product-agents/prdtp-agents-functions-cli-windows-x64.exe",
    ".agents/bin/prd-to-product-agents/prdtp-agents-functions-cli-linux-x64",
    ".agents/bin/prd-to-product-agents/prdtp-agents-functions-cli-darwin-arm64",
    ".agents/bin/prd-to-product-agents/checksums.sha256",
];

/// YAML files that should pass structural validation.
pub const YAML_FILES: &[&str] = &[
    "docs/project/handoffs.yaml",
    "docs/project/findings.yaml",
    "docs/project/releases.yaml",
    "docs/project/refined-stories.yaml",
    "docs/project/backlog.yaml",
    "docs/project/quality-gates.yaml",
    ".github/github-governance.yaml",
    ".github/agent-model-policy.yaml",
    ".github/workspace-capabilities.yaml",
];

/// Canonical agent names in hierarchy order (L0, then L1, then L2).
pub const AGENT_NAMES: &[&str] = &[
    "pm-orchestrator",
    "product-owner",
    "ux-designer",
    "software-architect",
    "tech-lead",
    "backend-developer",
    "frontend-developer",
    "qa-lead",
    "devops-release-engineer",
];

/// Agents that coordinate other agents (have `agents:` property).
pub const COORDINATOR_AGENTS: &[&str] = &["pm-orchestrator", "tech-lead"];

/// Level-2 agents (report to tech-lead only).
pub const L2_AGENTS: &[&str] = &["backend-developer", "frontend-developer"];

/// Path to the immutable-files manifest.
pub const IMMUTABLE_FILES_PATH: &str = ".github/immutable-files.txt";
