use anyhow::{bail, Result};
use clap::Args;
use colored::Colorize;
use std::path::Path;

use crate::common::audit;
use crate::common::enums::{
    FindingStatus, FindingType, Role, Severity, FINDING_SOURCE_ROLES, FINDING_TARGET_ROLES,
};
use crate::common::yaml_ops;
use crate::operations::state_store::{self, FindingEntry};

#[derive(Args)]
pub struct CreateFindingArgs {
    /// Source role creating the finding
    #[arg(long, value_enum)]
    pub(crate) source_role: Role,
    /// Target role for the finding
    #[arg(long, value_enum)]
    pub(crate) target_role: Role,
    /// Finding type
    #[arg(long, value_enum)]
    pub(crate) finding_type: FindingType,
    /// Severity level
    #[arg(long, value_enum)]
    pub(crate) severity: Severity,
    /// Entity reference (e.g. US-003)
    #[arg(long)]
    pub(crate) entity: String,
    /// Finding title (min 3 chars)
    #[arg(long)]
    pub(crate) title: String,
    /// Optional custom ID
    #[arg(long)]
    pub(crate) id: Option<String>,
}

#[derive(Args)]
pub struct UpdateFindingArgs {
    /// Finding ID to update
    #[arg(long)]
    pub(crate) finding_id: String,
    /// New status
    #[arg(long, value_enum)]
    pub(crate) new_status: FindingStatus,
    /// Agent role performing the update
    #[arg(long, value_enum)]
    pub(crate) agent_role: Role,
}

const FINDINGS_FILE: &str = "docs/project/findings.yaml";

pub fn create(workspace: &Path, args: CreateFindingArgs) -> Result<()> {
    tracing::info!(
        workspace = %workspace.display(),
        source_role = %args.source_role,
        target_role = %args.target_role,
        finding_type = %args.finding_type,
        severity = %args.severity,
        entity = %args.entity,
        "creating finding"
    );
    println!("{}", "=== Create Finding ===".cyan().bold());

    if !FINDING_SOURCE_ROLES.contains(&args.source_role) {
        bail!(
            "Role '{}' is not authorized to create findings. Allowed: {:?}",
            args.source_role,
            FINDING_SOURCE_ROLES
        );
    }

    if !FINDING_TARGET_ROLES.contains(&args.target_role) {
        bail!(
            "Role '{}' is not a valid finding target. Allowed: {:?}",
            args.target_role,
            FINDING_TARGET_ROLES
        );
    }

    if args.title.len() < 3 {
        bail!("Title must be at least 3 characters long");
    }

    let id = args.id.unwrap_or_else(|| yaml_ops::new_auto_id("fi-"));
    let today = yaml_ops::today_utc();
    let id_for_entry = id.clone();
    let id_for_check = id.clone();
    let source_role = args.source_role.to_string();
    let target_role = args.target_role.to_string();
    let finding_type = args.finding_type.to_string();
    let severity = args.severity.to_string();
    let entity = args.entity.clone();
    let title = args.title.clone();
    state_store::mutate_findings(workspace, move |document| {
        if document.findings.iter().any(|entry| entry.id == id_for_check) {
            bail!("Finding with ID '{id_for_check}' already exists");
        }
        document.findings.push(FindingEntry {
            id: id_for_entry,
            source: source_role,
            target: target_role,
            finding_type,
            severity,
            entity,
            title,
            status: "open".to_string(),
            created: today,
        });
        Ok(())
    })?;

    let _ = audit::try_audit_activity(
        workspace,
        &args.source_role.to_string(),
        "finding_created",
        "finding",
        &id,
        &format!(
            "type={}, severity={}, target={}",
            args.finding_type, args.severity, args.target_role
        ),
    );

    tracing::info!(finding_id = %id, entity = %args.entity, severity = %args.severity, "finding created");
    println!("{} Created finding {id}", "OK:".green().bold());
    Ok(())
}

pub fn update(workspace: &Path, args: UpdateFindingArgs) -> Result<()> {
    tracing::info!(
        workspace = %workspace.display(),
        finding_id = %args.finding_id,
        new_status = %args.new_status,
        agent_role = %args.agent_role,
        "updating finding"
    );
    println!("{}", "=== Update Finding ===".cyan().bold());

    let yaml_path = workspace.join(FINDINGS_FILE);
    if !yaml_path.exists() {
        bail!("findings.yaml not found");
    }
    let finding_id = args.finding_id.clone();
    let agent_role = args.agent_role.to_string();
    let new_status = args.new_status;
    let new_status_value = args.new_status.to_string();
    let current_status = state_store::mutate_findings(workspace, move |document| {
        let entry = document
            .findings
            .iter_mut()
            .find(|entry| entry.id == finding_id)
            .ok_or_else(|| anyhow::anyhow!("Finding '{}' not found", finding_id))?;

        let current_status = match entry.status.as_str() {
            "open" => FindingStatus::Open,
            "triaged" => FindingStatus::Triaged,
            "in_progress" => FindingStatus::InProgress,
            "resolved" => FindingStatus::Resolved,
            "wont_fix" => FindingStatus::WontFix,
            other => bail!("Unknown current status '{other}'"),
        };

        let valid = current_status.valid_transitions();
        if !valid.contains(&new_status) {
            bail!(
                "Invalid transition: {} -> {} (valid: {:?})",
                current_status,
                new_status,
                valid
            );
        }

        let authorized = agent_role == Role::QaLead.to_string()
            || agent_role == Role::PmOrchestrator.to_string()
            || agent_role == entry.target;
        if !authorized {
            bail!(
                "Role '{}' not authorized to update this finding (target: {})",
                agent_role,
                entry.target
            );
        }

        entry.status = new_status_value;
        Ok(current_status)
    })?;

    let _ = audit::try_audit_activity(
        workspace,
        &args.agent_role.to_string(),
        "finding_updated",
        "finding",
        &args.finding_id,
        &format!("{current_status} -> {}", args.new_status),
    );

    tracing::info!(finding_id = %args.finding_id, from_status = %current_status, to_status = %args.new_status, "finding updated");
    println!(
        "{} Updated finding {} → {}",
        "OK:".green().bold(),
        args.finding_id,
        args.new_status
    );
    Ok(())
}
