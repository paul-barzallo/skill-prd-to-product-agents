use anyhow::{bail, Result};
use clap::Args;
use colored::Colorize;
use std::path::Path;

use crate::common::audit;
use crate::common::enums::{ReleaseStatus, Role};
use crate::common::yaml_ops;
use crate::operations::state_store::{self, ReleaseEntry};

#[derive(Args)]
pub struct CreateReleaseArgs {
    /// Release name
    #[arg(long)]
    pub(crate) name: String,
    /// Target date (YYYY-MM-DD)
    #[arg(long)]
    pub(crate) target_date: String,
    /// Agent role (must be devops-release-engineer)
    #[arg(long, value_enum)]
    pub(crate) agent_role: Role,
    /// Comma-separated story IDs
    #[arg(long)]
    pub(crate) stories: Option<String>,
    /// Optional notes
    #[arg(long)]
    pub(crate) notes: Option<String>,
    /// Optional custom ID (auto-generated as R<N+1>)
    #[arg(long)]
    pub(crate) id: Option<String>,
}

#[derive(Args)]
pub struct UpdateReleaseArgs {
    /// Release ID (e.g. R1, R2)
    #[arg(long)]
    pub(crate) release_ref: String,
    /// New status
    #[arg(long, value_enum)]
    pub(crate) new_status: ReleaseStatus,
    /// Agent role (must be devops-release-engineer)
    #[arg(long, value_enum)]
    pub(crate) agent_role: Role,
}

const RELEASES_FILE: &str = "docs/project/releases.yaml";

pub fn create(workspace: &Path, args: CreateReleaseArgs) -> Result<()> {
    tracing::info!(
        workspace = %workspace.display(),
        name = %args.name,
        target_date = %args.target_date,
        agent_role = %args.agent_role,
        "creating release"
    );
    println!("{}", "=== Create Release ===".cyan().bold());

    // Authority check
    if args.agent_role != Role::DevopsReleaseEngineer {
        bail!(
            "Only devops-release-engineer can create releases (got: {})",
            args.agent_role
        );
    }

    // Date format validation
    let date_re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$")?;
    if !date_re.is_match(&args.target_date) {
        bail!(
            "Invalid date format '{}'. Expected YYYY-MM-DD",
            args.target_date
        );
    }

    let id = args.id.unwrap_or_else(|| {
        let yaml_path = workspace.join(RELEASES_FILE);
        let raw = std::fs::read_to_string(&yaml_path).unwrap_or_default();
        yaml_ops::next_release_id(&raw)
    });

    let today = yaml_ops::today_utc();
    let id_for_entry = id.clone();
    let id_for_check = id.clone();
    let agent_role = args.agent_role.to_string();
    let name = args.name.clone();
    let target_date = args.target_date.clone();
    let notes = args.notes.clone();
    let stories = args
        .stories
        .as_deref()
        .map(|value| {
            value
                .split(',')
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    state_store::mutate_releases(workspace, move |document| {
        if document.releases.iter().any(|entry| entry.id == id_for_check) {
            bail!("Release with ID '{id_for_check}' already exists");
        }
        document.releases.push(ReleaseEntry {
            id: id_for_entry,
            name,
            target_date,
            agent_role,
            created: today,
            status: "planning".to_string(),
            stories,
            notes,
        });
        Ok(())
    })?;

    let _ = audit::try_audit_activity(
        workspace,
        &args.agent_role.to_string(),
        "release_created",
        "release",
        &id,
        &format!("name={}, target={}", args.name, args.target_date),
    );

    tracing::info!(release_id = %id, name = %args.name, target_date = %args.target_date, "release created");
    println!("{} Created release {id}", "OK:".green().bold());
    Ok(())
}

pub fn update(workspace: &Path, args: UpdateReleaseArgs) -> Result<()> {
    tracing::info!(
        workspace = %workspace.display(),
        release_ref = %args.release_ref,
        new_status = %args.new_status,
        agent_role = %args.agent_role,
        "updating release"
    );
    println!("{}", "=== Update Release ===".cyan().bold());

    if args.agent_role != Role::DevopsReleaseEngineer {
        bail!(
            "Only devops-release-engineer can update releases (got: {})",
            args.agent_role
        );
    }

    let yaml_path = workspace.join(RELEASES_FILE);
    if !yaml_path.exists() {
        bail!("releases.yaml not found");
    }
    let release_ref = args.release_ref.clone();
    let new_status = args.new_status;
    let new_status_value = args.new_status.to_string();
    let current_status = state_store::mutate_releases(workspace, move |document| {
        let entry = document
            .releases
            .iter_mut()
            .find(|entry| entry.id == release_ref)
            .ok_or_else(|| anyhow::anyhow!("Release '{}' not found", release_ref))?;

        let current_status = match entry.status.as_str() {
            "planning" => ReleaseStatus::Planning,
            "ready" => ReleaseStatus::Ready,
            "approved" => ReleaseStatus::Approved,
            "deployed" => ReleaseStatus::Deployed,
            "rolled_back" => ReleaseStatus::RolledBack,
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

        entry.status = new_status_value;
        Ok(current_status)
    })?;

    let _ = audit::try_audit_activity(
        workspace,
        &args.agent_role.to_string(),
        "release_updated",
        "release",
        &args.release_ref,
        &format!("{current_status} -> {}", args.new_status),
    );

    tracing::info!(release_ref = %args.release_ref, from_status = %current_status, to_status = %args.new_status, "release updated");
    println!(
        "{} Updated release {} → {}",
        "OK:".green().bold(),
        args.release_ref,
        args.new_status
    );
    Ok(())
}
