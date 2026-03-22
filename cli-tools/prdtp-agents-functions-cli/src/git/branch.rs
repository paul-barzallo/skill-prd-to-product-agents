use anyhow::{bail, Result};
use clap::Args;
use colored::Colorize;
use git2::Repository;
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
    /// GitHub issue ID (optional, e.g. GH-42)
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

    // Validate base
    if args.base != "develop" && args.base != "main" {
        bail!("Base branch must be 'develop' or 'main' (got: '{}')", args.base);
    }

    // Build branch name
    let prefix = args.role.branch_prefix();
    let branch_name = match &args.issue_id {
        Some(id) => {
            // Normalize issue ID to lowercase: GH-42 → gh-42 (matches PR workflow regex)
            let normalized_id = id.to_lowercase();
            format!("{prefix}/{normalized_id}-{}", args.slug)
        }
        None => {
            bail!(
                "--issue-id is required. Branch names must include an issue reference \
                 (e.g. --issue-id GH-42) to pass the PR governance workflow."
            );
        }
    };

    // Validate slug (kebab-case, no spaces)
    let slug_re = regex::Regex::new(r"^[a-z0-9][a-z0-9-]*[a-z0-9]$|^[a-z0-9]$")?;
    if !slug_re.is_match(&args.slug) {
        bail!(
            "Invalid slug '{}'. Use kebab-case (e.g. 'checkout-form')",
            args.slug
        );
    }

    println!("  Branch: {branch_name}");
    println!("  Base: {}", args.base);

    let repo = Repository::open(workspace)
        .map_err(|e| anyhow::anyhow!("Not a git repository: {e}"))?;

    // Fetch origin (best effort)
    println!("  Fetching origin...");
    let fetch_result = fetch_origin(&repo);
    if let Err(e) = &fetch_result {
        eprintln!("  {} fetch origin failed: {e} — continuing", "⚠".yellow());
    }

    // Checkout base branch
    println!("  Checking out {base}...", base = args.base);
    checkout_branch(&repo, &args.base)?;

    // Pull with fast-forward
    if fetch_result.is_ok() {
        println!("  Pulling {base}...", base = args.base);
        if let Err(e) = pull_ff(&repo, &args.base) {
            eprintln!("  {} pull failed: {e} — continuing", "⚠".yellow());
        }
    }

    // Check if branch exists
    let branch_exists = repo.find_branch(&branch_name, git2::BranchType::Local).is_ok();

    if branch_exists {
        println!("  Branch '{branch_name}' exists — switching");
        checkout_branch(&repo, &branch_name)?;

        // Rebase onto base (best effort)
        println!("  Rebasing onto {base}...", base = args.base);
        // Using git CLI for rebase since libgit2's rebase API is complex
        let rebase = std::process::Command::new("git")
            .args(["rebase", &args.base])
            .current_dir(workspace)
            .output();
        match rebase {
            Ok(output) if output.status.success() => {
                println!("  {} Rebase successful", "✓".green());
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Abort rebase on conflict
                let _ = std::process::Command::new("git")
                    .args(["rebase", "--abort"])
                    .current_dir(workspace)
                    .output();
                bail!("Rebase conflict — aborted automatically. Resolve manually.\n{stderr}");
            }
            Err(e) => {
                eprintln!("  {} rebase command failed: {e}", "⚠".yellow());
            }
        }
    } else {
        println!("  Creating new branch '{branch_name}'");
        let head = repo.head()?.peel_to_commit()?;
        repo.branch(&branch_name, &head, false)?;
        checkout_branch(&repo, &branch_name)?;
    }

    println!(
        "{} On branch '{branch_name}'",
        "OK:".green().bold()
    );
    Ok(())
}

fn fetch_origin(repo: &Repository) -> Result<()> {
    let mut remote = repo.find_remote("origin")?;
    remote.fetch(&["refs/heads/*:refs/remotes/origin/*"], None, None)?;
    Ok(())
}

fn checkout_branch(repo: &Repository, name: &str) -> Result<()> {
    // Try local branch first
    let reference = format!("refs/heads/{name}");
    if let Ok(r) = repo.find_reference(&reference) {
        let obj = r.peel(git2::ObjectType::Commit)?;
        repo.checkout_tree(&obj, Some(git2::build::CheckoutBuilder::new().force()))?;
        repo.set_head(&reference)?;
        return Ok(());
    }

    // Try remote tracking branch
    let remote_ref = format!("refs/remotes/origin/{name}");
    if let Ok(r) = repo.find_reference(&remote_ref) {
        let obj = r.peel(git2::ObjectType::Commit)?;
        // Create local branch from remote
        let commit = obj.peel_to_commit()?;
        repo.branch(name, &commit, false)?;
        let local_ref = format!("refs/heads/{name}");
        let local = repo.find_reference(&local_ref)?;
        let local_obj = local.peel(git2::ObjectType::Commit)?;
        repo.checkout_tree(&local_obj, Some(git2::build::CheckoutBuilder::new().force()))?;
        repo.set_head(&local_ref)?;
        return Ok(());
    }

    bail!("Branch '{name}' not found locally or in origin");
}

fn pull_ff(repo: &Repository, branch: &str) -> Result<()> {
    let remote_ref = format!("refs/remotes/origin/{branch}");
    if let Ok(r) = repo.find_reference(&remote_ref) {
        let remote_commit = r.peel_to_commit()?;
        let local_ref = format!("refs/heads/{branch}");
        if let Ok(mut lr) = repo.find_reference(&local_ref) {
            lr.set_target(remote_commit.id(), "fast-forward pull")?;
            let obj = remote_commit.into_object();
            repo.checkout_tree(&obj, Some(git2::build::CheckoutBuilder::new().force()))?;
        }
    }
    Ok(())
}
