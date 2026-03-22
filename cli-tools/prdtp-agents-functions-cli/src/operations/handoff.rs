use anyhow::{bail, Result};
use clap::Args;
use colored::Colorize;
use std::path::Path;

use crate::common::audit;
use crate::common::enums::{HandoffReason, HandoffStatus, HandoffType, Role};
use crate::common::yaml_ops;

#[derive(Args)]
pub struct CreateHandoffArgs {
    /// Source role
    #[arg(long, value_enum)]
    pub(crate) from_role: Role,
    /// Target role
    #[arg(long, value_enum)]
    pub(crate) to_role: Role,
    /// Handoff type
    #[arg(long, value_enum)]
    pub(crate) handoff_type: HandoffType,
    /// Entity reference (e.g. US-001, fi-abc123)
    #[arg(long)]
    pub(crate) entity: String,
    /// Handoff reason
    #[arg(long, value_enum)]
    pub(crate) reason: HandoffReason,
    /// Optional details
    #[arg(long)]
    pub(crate) details: Option<String>,
    /// Optional custom ID (auto-generated if omitted)
    #[arg(long)]
    pub(crate) id: Option<String>,
}

#[derive(Args)]
pub struct UpdateHandoffArgs {
    /// Handoff ID to update
    #[arg(long)]
    pub(crate) handoff_id: String,
    /// New status
    #[arg(long, value_enum)]
    pub(crate) new_status: HandoffStatus,
    /// Agent role performing the update
    #[arg(long, value_enum)]
    pub(crate) agent_role: Role,
}

const HANDOFFS_FILE: &str = "docs/project/handoffs.yaml";

pub fn create(workspace: &Path, args: CreateHandoffArgs) -> Result<()> {
    tracing::info!(
        workspace = %workspace.display(),
        from_role = %args.from_role,
        to_role = %args.to_role,
        entity = %args.entity,
        reason = %args.reason,
        "creating handoff"
    );
    println!("{}", "=== Create Handoff ===".cyan().bold());

    let yaml_path = workspace.join(HANDOFFS_FILE);
    let header = "# Handoff queue - operational state (source of truth)\n\nhandoffs: []\n";
    let content = yaml_ops::ensure_yaml_file(&yaml_path, header)?;

    let id = args.id.unwrap_or_else(|| yaml_ops::new_auto_id("ho-"));
    if yaml_ops::entry_exists(&content, &id) {
        bail!("Handoff with ID '{id}' already exists");
    }

    let to_role = args.to_role.to_string();
    let reason = args.reason.to_string();
    let duplicate_pending = serde_yaml::from_str::<serde_yaml::Value>(&content)
        .ok()
        .and_then(|yaml| yaml.get("handoffs").and_then(serde_yaml::Value::as_sequence).cloned())
        .map(|entries| {
            entries.iter().any(|entry| {
                entry.get("entity").and_then(serde_yaml::Value::as_str) == Some(args.entity.as_str())
                    && entry.get("to").and_then(serde_yaml::Value::as_str) == Some(to_role.as_str())
                    && entry.get("reason").and_then(serde_yaml::Value::as_str) == Some(reason.as_str())
                    && entry.get("status").and_then(serde_yaml::Value::as_str) == Some("pending")
            })
        })
        .unwrap_or(false);
    if duplicate_pending {
        bail!(
            "Duplicate handoff detected: entity={}, to={}, reason={} already pending",
            args.entity,
            args.to_role,
            args.reason
        );
    }

    let today = yaml_ops::today_utc();
    let details_line = match &args.details {
        Some(details) => format!("\n    details: \"{}\"", yaml_ops::yaml_escape(details)),
        None => String::new(),
    };

    let entry = format!(
        "  - id: {id}\n    from: {}\n    to: {}\n    type: {}\n    entity: {}\n    reason: {}\n    status: pending\n    created: {today}{details_line}\n",
        args.from_role, args.to_role, args.handoff_type, args.entity, args.reason
    );

    let _lock = yaml_ops::YamlLock::acquire(&yaml_path)?;

    let updated = if content.contains("handoffs: []") {
        content.replace("handoffs: []", &format!("handoffs:\n{entry}"))
    } else if content.trim().ends_with("handoffs:") {
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
        &args.from_role.to_string(),
        "handoff_created",
        "handoff",
        &id,
        &format!("to={}, entity={}, reason={}", args.to_role, args.entity, args.reason),
    );

    tracing::info!(handoff_id = %id, entity = %args.entity, "handoff created");
    println!("{} Created handoff {id}", "OK:".green().bold());
    Ok(())
}

pub fn update(workspace: &Path, args: UpdateHandoffArgs) -> Result<()> {
    tracing::info!(
        workspace = %workspace.display(),
        handoff_id = %args.handoff_id,
        new_status = %args.new_status,
        agent_role = %args.agent_role,
        "updating handoff"
    );
    println!("{}", "=== Update Handoff ===".cyan().bold());

    let yaml_path = workspace.join(HANDOFFS_FILE);
    if !yaml_path.exists() {
        bail!("handoffs.yaml not found");
    }

    let content = std::fs::read_to_string(&yaml_path)?;
    let content = yaml_ops::normalize_lf(&content);

    let current_status_str = yaml_ops::read_yaml_entry_field(&content, &args.handoff_id, "status")
        .ok_or_else(|| anyhow::anyhow!("Handoff '{}' not found", args.handoff_id))?;

    let current_status = match current_status_str.as_str() {
        "pending" => HandoffStatus::Pending,
        "claimed" => HandoffStatus::Claimed,
        "done" => HandoffStatus::Done,
        "cancelled" => HandoffStatus::Cancelled,
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

    let target_role = yaml_ops::read_yaml_entry_field(&content, &args.handoff_id, "to")
        .unwrap_or_default();

    match args.new_status {
        HandoffStatus::Claimed | HandoffStatus::Done => {
            if args.agent_role.to_string() != target_role {
                bail!(
                    "Only the target role ({target_role}) can transition to {}",
                    args.new_status
                );
            }
        }
        HandoffStatus::Cancelled => {
            if args.agent_role != Role::PmOrchestrator {
                bail!("Only pm-orchestrator can cancel handoffs");
            }
        }
        _ => {}
    }

    let _lock = yaml_ops::YamlLock::acquire(&yaml_path)?;

    let old_pattern = format!("id: {}\n", args.handoff_id);
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
        bail!("Could not locate entry block for '{}'", args.handoff_id);
    }

    let _ = audit::try_audit_activity(
        workspace,
        &args.agent_role.to_string(),
        "handoff_updated",
        "handoff",
        &args.handoff_id,
        &format!("{current_status} -> {}", args.new_status),
    );

    tracing::info!(handoff_id = %args.handoff_id, from_status = %current_status, to_status = %args.new_status, "handoff updated");
    println!(
        "{} Updated handoff {} → {}",
        "OK:".green().bold(),
        args.handoff_id,
        args.new_status
    );
    Ok(())
}
