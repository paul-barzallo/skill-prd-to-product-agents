use anyhow::{bail, Result};
use clap::Args;
use colored::Colorize;
use git2::{BranchType, Repository, StatusOptions};
use serde_json::json;
use std::path::Path;

use crate::common::enums::Role;

#[derive(Args)]
pub struct CheckoutTaskBranchArgs {
    /// Agent role (determines branch prefix)
    #[arg(long, value_enum)]
    role: Role,
    /// Branch slug (kebab-case description)
    #[arg(long)]
    slug: String,
    /// GitHub issue ID (required, e.g. GH-42)
    #[arg(long)]
    issue_id: Option<String>,
    /// Base branch (develop or main)
    #[arg(long, default_value = "develop")]
    base: String,
}

pub fn run(workspace: &Path, args: CheckoutTaskBranchArgs) -> Result<()> {
    println!("{}", "=== Checkout Task Branch ===".cyan().bold());
    crate::common::capability_contract::require_policy_enabled(
        workspace,
        "git",
        "git checkout-task-branch",
    )?;

    if args.base != "develop" && args.base != "main" {
        bail!(
            "Base branch must be 'develop' or 'main' (got: '{}')",
            args.base
        );
    }

    let prefix = args.role.branch_prefix();
    let branch_name = match &args.issue_id {
        Some(id) => format!("{prefix}/{}-{}", id.to_lowercase(), args.slug),
        None => bail!(
            "--issue-id is required. Branch names must include an issue reference \
             (e.g. --issue-id GH-42) to pass the PR governance workflow."
        ),
    };

    let slug_re = regex::Regex::new(r"^[a-z0-9][a-z0-9-]*[a-z0-9]$|^[a-z0-9]$")?;
    if !slug_re.is_match(&args.slug) {
        bail!(
            "Invalid slug '{}'. Use kebab-case (e.g. 'checkout-form')",
            args.slug
        );
    }

    let repo = Repository::open(workspace)
        .map_err(|error| anyhow::anyhow!("Not a git repository: {error}"))?;

    let dirty_paths = dirty_paths(&repo)?;
    if !dirty_paths.is_empty() {
        bail!(
            "Refusing to switch branches with local changes present. Commit, stash, or discard them first:\n{}",
            dirty_paths
                .iter()
                .map(|path| format!("  - {path}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    println!("  Branch: {branch_name}");
    println!("  Base: {}", args.base);

    println!("  Fetching origin...");
    if let Err(error) = fetch_origin(&repo) {
        eprintln!(
            "  {} fetch origin failed: {error} - continuing",
            "!".yellow()
        );
    }

    if repo.find_branch(&branch_name, BranchType::Local).is_ok() {
        println!("  Branch '{branch_name}' exists locally - switching safely");
        checkout_local_branch(&repo, &branch_name)?;
    } else if let Ok(remote_branch) = repo.find_branch(&branch_name, BranchType::Remote) {
        println!("  Branch '{branch_name}' exists on origin - creating local tracking branch");
        let commit = remote_branch.get().peel_to_commit()?;
        repo.branch(&branch_name, &commit, false)?;
        checkout_local_branch(&repo, &branch_name)?;
    } else {
        println!("  Creating new branch '{branch_name}' from {}", args.base);
        let base_commit = resolve_branch_commit(&repo, &args.base)?;
        repo.branch(&branch_name, &base_commit, false)?;
        checkout_local_branch(&repo, &branch_name)?;
    }

    let _ = crate::audit::events::record_sensitive_action(
        workspace,
        "git.checkout-task-branch",
        &args.role.to_string(),
        "success",
        json!({
            "branch": branch_name,
            "base": args.base
        }),
    );
    println!("{} On branch '{branch_name}'", "OK:".green().bold());
    Ok(())
}

fn dirty_paths(repo: &Repository) -> Result<Vec<String>> {
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .include_unmodified(false)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true);

    let statuses = repo.statuses(Some(&mut options))?;
    let dirty = statuses
        .iter()
        .filter_map(|entry| entry.path().map(str::to_string))
        .collect::<Vec<_>>();
    Ok(dirty)
}

fn fetch_origin(repo: &Repository) -> Result<()> {
    let mut remote = repo.find_remote("origin")?;
    remote.fetch(&["refs/heads/*:refs/remotes/origin/*"], None, None)?;
    Ok(())
}

fn resolve_branch_commit<'repo>(
    repo: &'repo Repository,
    name: &str,
) -> Result<git2::Commit<'repo>> {
    if let Ok(reference) = repo.find_reference(&format!("refs/remotes/origin/{name}")) {
        return reference.peel_to_commit().map_err(Into::into);
    }
    if let Ok(reference) = repo.find_reference(&format!("refs/heads/{name}")) {
        return reference.peel_to_commit().map_err(Into::into);
    }
    bail!("Branch '{name}' not found locally or in origin");
}

fn checkout_local_branch(repo: &Repository, name: &str) -> Result<()> {
    let reference = format!("refs/heads/{name}");
    repo.find_reference(&reference)?;
    repo.set_head(&reference)?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().safe()))?;
    Ok(())
}
