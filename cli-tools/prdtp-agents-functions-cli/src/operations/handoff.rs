use anyhow::{bail, Result};
use clap::Args;
use colored::Colorize;
use std::path::Path;

use crate::common::audit;
use crate::common::enums::{HandoffReason, HandoffStatus, HandoffType, Role};
use crate::common::yaml_ops;
use crate::operations::state_store::{self, HandoffEntry};

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

    let id = args.id.unwrap_or_else(|| yaml_ops::new_auto_id("ho-"));
    let today = yaml_ops::today_utc();
    let id_for_entry = id.clone();
    let id_for_check = id.clone();
    let to_role = args.to_role.to_string();
    let reason = args.reason.to_string();
    let entity = args.entity.clone();
    let details = args.details.clone();
    let from_role = args.from_role.to_string();
    let handoff_type = args.handoff_type.to_string();
    state_store::mutate_handoffs(workspace, move |document| {
        if document.handoffs.iter().any(|entry| entry.id == id_for_check) {
            bail!("Handoff with ID '{id_for_check}' already exists");
        }
        if document.handoffs.iter().any(|entry| {
            entry.entity == entity
                && entry.to == to_role
                && entry.reason == reason
                && entry.status == "pending"
        }) {
            bail!(
                "Duplicate handoff detected: entity={}, to={}, reason={} already pending",
                entity,
                to_role,
                reason
            );
        }

        document.handoffs.push(HandoffEntry {
            id: id_for_entry,
            from: from_role,
            to: to_role,
            handoff_type,
            entity,
            reason,
            status: "pending".to_string(),
            created: today,
            details,
        });
        Ok(())
    })?;

    let _ = audit::try_audit_activity(
        workspace,
        &args.from_role.to_string(),
        "handoff_created",
        "handoff",
        &id,
        &format!(
            "to={}, entity={}, reason={}",
            args.to_role, args.entity, args.reason
        ),
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
    let handoff_id = args.handoff_id.clone();
    let agent_role = args.agent_role.to_string();
    let new_status_value = args.new_status.to_string();
    let new_status = args.new_status;
    let current_status = state_store::mutate_handoffs(workspace, move |document| {
        let entry = document
            .handoffs
            .iter_mut()
            .find(|entry| entry.id == handoff_id)
            .ok_or_else(|| anyhow::anyhow!("Handoff '{}' not found", handoff_id))?;

        let current_status = match entry.status.as_str() {
            "pending" => HandoffStatus::Pending,
            "claimed" => HandoffStatus::Claimed,
            "done" => HandoffStatus::Done,
            "cancelled" => HandoffStatus::Cancelled,
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

        match new_status {
            HandoffStatus::Claimed | HandoffStatus::Done => {
                if agent_role != entry.to {
                    bail!(
                        "Only the target role ({}) can transition to {}",
                        entry.to,
                        new_status
                    );
                }
            }
            HandoffStatus::Cancelled => {
                if agent_role != Role::PmOrchestrator.to_string() {
                    bail!("Only pm-orchestrator can cancel handoffs");
                }
            }
            _ => {}
        }

        entry.status = new_status_value;
        Ok(current_status)
    })?;

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
