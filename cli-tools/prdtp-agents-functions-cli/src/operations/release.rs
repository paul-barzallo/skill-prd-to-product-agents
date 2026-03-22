use anyhow::{bail, Result};
use clap::Args;
use colored::Colorize;
use std::path::Path;

use crate::common::enums::{ReleaseStatus, Role};
use crate::common::yaml_ops;
use crate::common::audit;

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
        bail!("Only devops-release-engineer can create releases (got: {})", args.agent_role);
    }

    // Date format validation
    let date_re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$")?;
    if !date_re.is_match(&args.target_date) {
        bail!("Invalid date format '{}'. Expected YYYY-MM-DD", args.target_date);
    }

    let yaml_path = workspace.join(RELEASES_FILE);
    let header = "# Release tracker - operational state (source of truth)\n\nreleases: []\n";
    let content = yaml_ops::ensure_yaml_file(&yaml_path, header)?;

    let id = args.id.unwrap_or_else(|| yaml_ops::next_release_id(&content));

    if yaml_ops::entry_exists(&content, &id) {
        bail!("Release with ID '{id}' already exists");
    }

    let today = yaml_ops::today_utc();
    let stories_yaml = match &args.stories {
        Some(s) => {
            let items: Vec<&str> = s.split(',').map(|i| i.trim()).filter(|i| !i.is_empty()).collect();
            if items.is_empty() {
                String::new()
            } else {
                let lines: Vec<String> = items.iter().map(|i| format!("      - {i}")).collect();
                format!("\n    stories:\n{}", lines.join("\n"))
            }
        }
        None => String::new(),
    };
    let notes_yaml = match &args.notes {
        Some(n) => format!("\n    notes: \"{}\"", yaml_ops::yaml_escape(n)),
        None => String::new(),
    };

    let entry = format!(
        "  - id: {id}\n    name: \"{}\"\n    target_date: {}\n    agent_role: {}\n    created: {today}\n    status: planning{stories_yaml}{notes_yaml}\n",
        yaml_ops::yaml_escape(&args.name),
        args.target_date,
        args.agent_role
    );

    let _lock = yaml_ops::YamlLock::acquire(&yaml_path)?;

    let updated = if content.contains("releases: []") {
        content.replace("releases: []", &format!("releases:\n{entry}"))
    } else if content.trim().ends_with("releases:") {
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
        bail!("Only devops-release-engineer can update releases (got: {})", args.agent_role);
    }

    let yaml_path = workspace.join(RELEASES_FILE);
    if !yaml_path.exists() {
        bail!("releases.yaml not found");
    }

    let content = std::fs::read_to_string(&yaml_path)?;
    let content = yaml_ops::normalize_lf(&content);

    let current_status_str = yaml_ops::read_yaml_entry_field(&content, &args.release_ref, "status")
        .ok_or_else(|| anyhow::anyhow!("Release '{}' not found", args.release_ref))?;

    let current_status = match current_status_str.as_str() {
        "planning" => ReleaseStatus::Planning,
        "ready" => ReleaseStatus::Ready,
        "approved" => ReleaseStatus::Approved,
        "deployed" => ReleaseStatus::Deployed,
        "rolled_back" => ReleaseStatus::RolledBack,
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

    let _lock = yaml_ops::YamlLock::acquire(&yaml_path)?;

    let old_pattern = format!("id: {}\n", args.release_ref);
    if let Some(start) = content.find(&old_pattern) {
        let block_start = content[..start].rfind("  - id:").unwrap_or(start);
        let rest = &content[block_start..];
        let next_entry = rest[1..].find("  - id:").map(|i| block_start + 1 + i).unwrap_or(content.len());
        let block = &content[block_start..next_entry];

        let old_status_line = format!("status: {current_status}");
        let new_status_line = format!("status: {}", args.new_status);
        let new_block = block.replace(&old_status_line, &new_status_line);
        let updated = format!("{}{new_block}{}", &content[..block_start], &content[next_entry..]);

        yaml_ops::atomic_write(&yaml_path, &updated)?;
    } else {
        bail!("Could not locate entry block for '{}'", args.release_ref);
    }

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
