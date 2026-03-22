use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;

// ── Agent Roles ──────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    PmOrchestrator,
    ProductOwner,
    UxDesigner,
    SoftwareArchitect,
    TechLead,
    BackendDeveloper,
    FrontendDeveloper,
    QaLead,
    DevopsReleaseEngineer,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::PmOrchestrator => "pm-orchestrator",
            Self::ProductOwner => "product-owner",
            Self::UxDesigner => "ux-designer",
            Self::SoftwareArchitect => "software-architect",
            Self::TechLead => "tech-lead",
            Self::BackendDeveloper => "backend-developer",
            Self::FrontendDeveloper => "frontend-developer",
            Self::QaLead => "qa-lead",
            Self::DevopsReleaseEngineer => "devops-release-engineer",
        };
        write!(f, "{s}")
    }
}

impl Role {
    pub fn branch_prefix(&self) -> &'static str {
        match self {
            Self::PmOrchestrator => "product",
            Self::ProductOwner => "product",
            Self::UxDesigner => "ux",
            Self::SoftwareArchitect => "arch",
            Self::TechLead => "techlead",
            Self::BackendDeveloper => "backend",
            Self::FrontendDeveloper => "frontend",
            Self::QaLead => "qa",
            Self::DevopsReleaseEngineer => "ops",
        }
    }
}

// ── Handoff Types ────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffType {
    Normal,
    Escalation,
    Rework,
    Approval,
}

impl fmt::Display for HandoffType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Normal => "normal",
            Self::Escalation => "escalation",
            Self::Rework => "rework",
            Self::Approval => "approval",
        };
        write!(f, "{s}")
    }
}

// ── Handoff Reasons ──────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffReason {
    NewWork,
    NeedsRefinement,
    NeedsRework,
    Blocked,
    ReadyForReview,
    ReadyForRelease,
    ScopeChange,
    TechnicalRisk,
    EnvironmentIssue,
    ClientRejected,
}

impl fmt::Display for HandoffReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::NewWork => "new_work",
            Self::NeedsRefinement => "needs_refinement",
            Self::NeedsRework => "needs_rework",
            Self::Blocked => "blocked",
            Self::ReadyForReview => "ready_for_review",
            Self::ReadyForRelease => "ready_for_release",
            Self::ScopeChange => "scope_change",
            Self::TechnicalRisk => "technical_risk",
            Self::EnvironmentIssue => "environment_issue",
            Self::ClientRejected => "client_rejected",
        };
        write!(f, "{s}")
    }
}

// ── Handoff Status ───────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffStatus {
    Pending,
    Claimed,
    Done,
    Cancelled,
}

impl fmt::Display for HandoffStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Pending => "pending",
            Self::Claimed => "claimed",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
        };
        write!(f, "{s}")
    }
}

impl HandoffStatus {
    /// Returns the valid next states from the current state.
    pub fn valid_transitions(&self) -> &[HandoffStatus] {
        match self {
            Self::Pending => &[Self::Claimed, Self::Cancelled],
            Self::Claimed => &[Self::Done, Self::Cancelled],
            Self::Done => &[],
            Self::Cancelled => &[],
        }
    }
}

// ── Finding Types ────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingType {
    Bug,
    Risk,
    Ambiguity,
    Security,
    Ux,
    Architecture,
}

impl fmt::Display for FindingType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Bug => "bug",
            Self::Risk => "risk",
            Self::Ambiguity => "ambiguity",
            Self::Security => "security",
            Self::Ux => "ux",
            Self::Architecture => "architecture",
        };
        write!(f, "{s}")
    }
}

// ── Finding Source Roles ─────────────────────────────────────────
pub const FINDING_SOURCE_ROLES: &[Role] = &[
    Role::QaLead,
    Role::SoftwareArchitect,
    Role::TechLead,
    Role::DevopsReleaseEngineer,
];

pub const FINDING_TARGET_ROLES: &[Role] = &[
    Role::ProductOwner,
    Role::TechLead,
    Role::PmOrchestrator,
];

// ── Finding Severity ─────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        };
        write!(f, "{s}")
    }
}

// ── Finding Status ───────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    Open,
    Triaged,
    InProgress,
    Resolved,
    WontFix,
}

impl fmt::Display for FindingStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Open => "open",
            Self::Triaged => "triaged",
            Self::InProgress => "in_progress",
            Self::Resolved => "resolved",
            Self::WontFix => "wont_fix",
        };
        write!(f, "{s}")
    }
}

impl FindingStatus {
    pub fn valid_transitions(&self) -> &[FindingStatus] {
        match self {
            Self::Open => &[Self::Triaged, Self::WontFix],
            Self::Triaged => &[Self::InProgress, Self::WontFix],
            Self::InProgress => &[Self::Resolved, Self::WontFix],
            Self::Resolved => &[],
            Self::WontFix => &[],
        }
    }
}

// ── Release Status ───────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseStatus {
    Planning,
    Ready,
    Approved,
    Deployed,
    RolledBack,
}

impl fmt::Display for ReleaseStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Planning => "planning",
            Self::Ready => "ready",
            Self::Approved => "approved",
            Self::Deployed => "deployed",
            Self::RolledBack => "rolled_back",
        };
        write!(f, "{s}")
    }
}

impl ReleaseStatus {
    pub fn valid_transitions(&self) -> &[ReleaseStatus] {
        match self {
            Self::Planning => &[Self::Ready],
            Self::Ready => &[Self::Approved, Self::RolledBack],
            Self::Approved => &[Self::Deployed, Self::RolledBack],
            Self::Deployed => &[],
            Self::RolledBack => &[],
        }
    }
}

// ── Environment ──────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Environment {
    Dev,
    Qa,
    Staging,
    Prod,
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Dev => "dev",
            Self::Qa => "qa",
            Self::Staging => "staging",
            Self::Prod => "prod",
        };
        write!(f, "{s}")
    }
}

// ── Event Types ──────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    DeployStarted,
    DeployFinished,
    DeployFailed,
    HealthDegraded,
    HealthRestored,
    Rollback,
    IncidentDetected,
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::DeployStarted => "deploy_started",
            Self::DeployFinished => "deploy_finished",
            Self::DeployFailed => "deploy_failed",
            Self::HealthDegraded => "health_degraded",
            Self::HealthRestored => "health_restored",
            Self::Rollback => "rollback",
            Self::IncidentDetected => "incident_detected",
        };
        write!(f, "{s}")
    }
}

// ── Conventional Commit Types ────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitType {
    Feat,
    Fix,
    Chore,
    Docs,
    Test,
    Refactor,
    Ci,
    Perf,
    Style,
}

impl fmt::Display for CommitType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Feat => "feat",
            Self::Fix => "fix",
            Self::Chore => "chore",
            Self::Docs => "docs",
            Self::Test => "test",
            Self::Refactor => "refactor",
            Self::Ci => "ci",
            Self::Perf => "perf",
            Self::Style => "style",
        };
        write!(f, "{s}")
    }
}

// ── Validation Status ────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValidationStatus {
    NotRun,
    Passed,
    Warnings,
    Failed,
}

impl fmt::Display for ValidationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::NotRun => "not-run",
            Self::Passed => "passed",
            Self::Warnings => "warnings",
            Self::Failed => "failed",
        };
        write!(f, "{s}")
    }
}
