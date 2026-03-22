use anyhow::{bail, Result};
use clap::Args;
use colored::Colorize;
use std::path::Path;

use crate::common::audit;
use crate::common::enums::{
    FindingStatus, FindingType, Role, Severity, FINDING_SOURCE_ROLES, FINDING_TARGET_ROLES,
};
use crate::common::yaml_ops;

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

    let yaml_path = workspace.join(FINDINGS_FILE);
    let header = "# Findings register - operational state (source of truth)\n\nfindings: []\n";
    let content = yaml_ops::ensure_yaml_file(&yaml_path, header)?;

    let id = args.id.unwrap_or_else(|| yaml_ops::new_auto_id("fi-"));
    if yaml_ops::entry_exists(&content, &id) {
        bail!("Finding with ID '{id}' already exists");
    }

    let today = yaml_ops::today_utc();
    let entry = format!(
        "  - id: {id}\n    source: {}\n    target: {}\n    type: {}\n    severity: {}\n    entity: {}\n    title: \"{}\"\n    status: open\n    created: {today}\n",
        args.source_role,
        args.target_role,
        args.finding_type,
        args.severity,
        args.entity,
        yaml_ops::yaml_escape(&args.title)
    );

    let _lock = yaml_ops::YamlLock::acquire(&yaml_path)?;

    let updated = if content.contains("findings: []") {
        content.replace("findings: []", &format!("findings:\n{entry}"))
    } else if content.trim().ends_with("findings:") {
        format!("{content}\n{entry}")
    } else {
        format!("{content}{entry}")
    };

    yaml_ops::atomic_write(&yaml_path, &updated)?;

    let written = std::fs::read_to_string(&yaml_path)?;
    if !yaml_ops::entry_exists(&written, &id) {
        bail!("Post-write verification failed: ID '{id}' not found after write");
    }

    let _ = audit::try_audit_activity(
        workspace,
        &args.source_role.to_string(),
        "finding_created",
        "finding",
        &id,
        &format!("type={}, severity={}, target={}", args.finding_type, args.severity, args.target_role),
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

    let content = std::fs::read_to_string(&yaml_path)?;
    let content = yaml_ops::normalize_lf(&content);

    let current_status_str = yaml_ops::read_yaml_entry_field(&content, &args.finding_id, "status")
        .ok_or_else(|| anyhow::anyhow!("Finding '{}' not found", args.finding_id))?;

    let current_status = match current_status_str.as_str() {
        "open" => FindingStatus::Open,
        "triaged" => FindingStatus::Triaged,
        "in_progress" => FindingStatus::InProgress,
        "resolved" => FindingStatus::Resolved,
        "wont_fix" => FindingStatus::WontFix,
        other => bail!("Unknown current status '{other}'"),
    };

    let valid = current_status.valid_transitions();
    if !valid.contains(&args.new_status) {
        bail!(
            "Invalid transition: {} -> {} (valid: {:?})",
            current_status,
            args.new_status,
            valid
        );
    }

    let target_role_str = yaml_ops::read_yaml_entry_field(&content, &args.finding_id, "target")
        .unwrap_or_default();
    let authorized = args.agent_role == Role::QaLead
        || args.agent_role == Role::PmOrchestrator
        || args.agent_role.to_string() == target_role_str;
    if !authorized {
        bail!(
            "Role '{}' not authorized to update this finding (target: {target_role_str})",
            args.agent_role
        );
    }

    let _lock = yaml_ops::YamlLock::acquire(&yaml_path)?;

    let old_pattern = format!("id: {}\n", args.finding_id);
    if let Some(start) = content.find(&old_pattern) {
        let block_start = content[..start].rfind("  - id:").unwrap_or(start);
        let rest = &content[block_start..];
        let next_entry = rest[1..]
            .find("  - id:")
            .map(|index| block_start + 1 + index)
            .unwrap_or(content.len());
        let block = &content[block_start..next_entry];

        let old_status_line = format!("status: {current_status}");
        let new_status_line = format!("status: {}", args.new_status);
        let new_block = block.replace(&old_status_line, &new_status_line);
        let updated = format!("{}{new_block}{}", &content[..block_start], &content[next_entry..]);

        yaml_ops::atomic_write(&yaml_path, &updated)?;
    } else {
        bail!("Could not locate entry block for '{}'", args.finding_id);
    }

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
