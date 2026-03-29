use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand, ValueEnum};
use colored::Colorize;
use serde_json::json;
use std::path::Path;
use std::process::Command;

#[derive(Subcommand)]
pub enum GithubCommands {
    /// GitHub issue operations routed through the runtime CLI
    Issue {
        #[command(subcommand)]
        sub: GithubIssueCommands,
    },
}

#[derive(Subcommand)]
pub enum GithubIssueCommands {
    /// Create a GitHub issue
    Create(CreateIssueArgs),
    /// Update a GitHub issue
    Update(UpdateIssueArgs),
    /// Add a comment to a GitHub issue
    Comment(CommentIssueArgs),
    /// Add or remove labels on a GitHub issue
    Label(LabelIssueArgs),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum IssueState {
    Open,
    Closed,
}

#[derive(Args)]
pub struct CreateIssueArgs {
    #[arg(long)]
    pub title: String,
    #[arg(long)]
    pub body: String,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long = "label")]
    pub labels: Vec<String>,
    #[arg(long = "assignee")]
    pub assignees: Vec<String>,
}

#[derive(Args)]
pub struct UpdateIssueArgs {
    #[arg(long)]
    pub issue_ref: String,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long)]
    pub state: Option<IssueState>,
    #[arg(long = "add-label")]
    pub add_labels: Vec<String>,
    #[arg(long = "remove-label")]
    pub remove_labels: Vec<String>,
}

#[derive(Args)]
pub struct CommentIssueArgs {
    #[arg(long)]
    pub issue_ref: String,
    #[arg(long)]
    pub body: String,
    #[arg(long)]
    pub repo: Option<String>,
}

#[derive(Args)]
pub struct LabelIssueArgs {
    #[arg(long)]
    pub issue_ref: String,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long = "add")]
    pub add: Vec<String>,
    #[arg(long = "remove")]
    pub remove: Vec<String>,
}

pub fn run(workspace: &Path, sub: GithubCommands) -> Result<()> {
    match sub {
        GithubCommands::Issue { sub } => match sub {
            GithubIssueCommands::Create(args) => create_issue(workspace, args),
            GithubIssueCommands::Update(args) => update_issue(workspace, args),
            GithubIssueCommands::Comment(args) => comment_issue(workspace, args),
            GithubIssueCommands::Label(args) => label_issue(workspace, args),
        },
    }
}

fn create_issue(workspace: &Path, args: CreateIssueArgs) -> Result<()> {
    println!("{}", "=== GitHub Issue Create ===".cyan().bold());
    crate::common::capability_contract::require_policy_enabled(
        workspace,
        "gh",
        "github issue create",
    )?;
    let repo = resolve_repo(workspace, args.repo.as_deref())?;
    let mut command = Command::new("gh");
    command
        .current_dir(workspace)
        .args(["issue", "create", "--repo", &repo, "--title", &args.title, "--body", &args.body]);
    for label in &args.labels {
        command.args(["--label", label]);
    }
    for assignee in &args.assignees {
        command.args(["--assignee", assignee]);
    }
    let output = command.output().context("running gh issue create")?;
    ensure_success("gh issue create", &output)?;
    let _ = crate::audit::events::record_sensitive_action(
        workspace,
        "github.issue.create",
        "runtime-cli",
        "success",
        json!({
            "repo": repo,
            "title": args.title,
            "labels": args.labels,
            "assignees": args.assignees
        }),
    );
    println!(
        "{} {}",
        "OK:".green().bold(),
        String::from_utf8_lossy(&output.stdout).trim()
    );
    Ok(())
}

fn update_issue(workspace: &Path, args: UpdateIssueArgs) -> Result<()> {
    println!("{}", "=== GitHub Issue Update ===".cyan().bold());
    crate::common::capability_contract::require_policy_enabled(
        workspace,
        "gh",
        "github issue update",
    )?;
    let repo = resolve_repo(workspace, args.repo.as_deref())?;
    let issue_ref = normalize_issue_ref(&args.issue_ref);
    let needs_edit = args.title.is_some()
        || args.body.is_some()
        || !args.add_labels.is_empty()
        || !args.remove_labels.is_empty();
    if !needs_edit && args.state.is_none() {
        bail!("github issue update requires at least one field change or --state");
    }

    if needs_edit {
        let mut command = Command::new("gh");
        command
            .current_dir(workspace)
            .args(["issue", "edit", &issue_ref, "--repo", &repo]);
        if let Some(title) = &args.title {
            command.args(["--title", title]);
        }
        if let Some(body) = &args.body {
            command.args(["--body", body]);
        }
        for label in &args.add_labels {
            command.args(["--add-label", label]);
        }
        for label in &args.remove_labels {
            command.args(["--remove-label", label]);
        }
        let output = command.output().context("running gh issue edit")?;
        ensure_success("gh issue edit", &output)?;
    }

    if let Some(state) = args.state {
        apply_issue_state(workspace, &repo, &issue_ref, state)?;
    }

    let _ = crate::audit::events::record_sensitive_action(
        workspace,
        "github.issue.update",
        "runtime-cli",
        "success",
        json!({
            "repo": repo,
            "issue_ref": issue_ref,
            "state": args.state.map(issue_state),
            "add_labels": args.add_labels,
            "remove_labels": args.remove_labels
        }),
    );
    println!("{} updated issue {}", "OK:".green().bold(), issue_ref);
    Ok(())
}

fn apply_issue_state(workspace: &Path, repo: &str, issue_ref: &str, state: IssueState) -> Result<()> {
    let subcommand = match state {
        IssueState::Open => "reopen",
        IssueState::Closed => "close",
    };
    let output = Command::new("gh")
        .current_dir(workspace)
        .args(["issue", subcommand, issue_ref, "--repo", repo])
        .output()
        .with_context(|| format!("running gh issue {}", subcommand))?;
    ensure_success(&format!("gh issue {}", subcommand), &output)?;
    Ok(())
}

fn comment_issue(workspace: &Path, args: CommentIssueArgs) -> Result<()> {
    println!("{}", "=== GitHub Issue Comment ===".cyan().bold());
    crate::common::capability_contract::require_policy_enabled(
        workspace,
        "gh",
        "github issue comment",
    )?;
    let repo = resolve_repo(workspace, args.repo.as_deref())?;
    let issue_ref = normalize_issue_ref(&args.issue_ref);
    let output = Command::new("gh")
        .current_dir(workspace)
        .args([
            "issue",
            "comment",
            &issue_ref,
            "--repo",
            &repo,
            "--body",
            &args.body,
        ])
        .output()
        .context("running gh issue comment")?;
    ensure_success("gh issue comment", &output)?;
    let _ = crate::audit::events::record_sensitive_action(
        workspace,
        "github.issue.comment",
        "runtime-cli",
        "success",
        json!({
            "repo": repo,
            "issue_ref": issue_ref
        }),
    );
    println!("{} commented on issue {}", "OK:".green().bold(), issue_ref);
    Ok(())
}

fn label_issue(workspace: &Path, args: LabelIssueArgs) -> Result<()> {
    println!("{}", "=== GitHub Issue Label ===".cyan().bold());
    crate::common::capability_contract::require_policy_enabled(
        workspace,
        "gh",
        "github issue label",
    )?;
    if args.add.is_empty() && args.remove.is_empty() {
        bail!("github issue label requires at least one --add or --remove value");
    }
    let repo = resolve_repo(workspace, args.repo.as_deref())?;
    let issue_ref = normalize_issue_ref(&args.issue_ref);
    let mut command = Command::new("gh");
    command
        .current_dir(workspace)
        .args(["issue", "edit", &issue_ref, "--repo", &repo]);
    for label in &args.add {
        command.args(["--add-label", label]);
    }
    for label in &args.remove {
        command.args(["--remove-label", label]);
    }
    let output = command.output().context("running gh issue edit for labels")?;
    ensure_success("gh issue edit", &output)?;
    let _ = crate::audit::events::record_sensitive_action(
        workspace,
        "github.issue.label",
        "runtime-cli",
        "success",
        json!({
            "repo": repo,
            "issue_ref": issue_ref,
            "add": args.add,
            "remove": args.remove
        }),
    );
    println!("{} labels updated for issue {}", "OK:".green().bold(), issue_ref);
    Ok(())
}

fn resolve_repo(workspace: &Path, explicit: Option<&str>) -> Result<String> {
    if let Some(repo) = explicit {
        return Ok(repo.trim().to_string());
    }

    if let Ok(governance) =
        crate::validate::readiness::load_governance(&workspace.join(".github/github-governance.yaml"))
    {
        if let (Some(owner), Some(name)) = (
            crate::validate::readiness::yaml_string(&governance, &["github", "repository", "owner"]),
            crate::validate::readiness::yaml_string(&governance, &["github", "repository", "name"]),
        ) {
            if !owner.contains("REPLACE_ME") && !name.contains("REPLACE_ME") {
                return Ok(format!("{owner}/{name}"));
            }
        }
    }

    let output = Command::new("gh")
        .current_dir(workspace)
        .args(["repo", "view", "--json", "nameWithOwner", "-q", ".nameWithOwner"])
        .output()
        .context("running gh repo view")?;
    ensure_success("gh repo view", &output)?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn normalize_issue_ref(input: &str) -> String {
    let trimmed = input.trim();
    if let Some(rest) = trimmed.strip_prefix("GH-") {
        return rest.to_string();
    }
    trimmed.trim_start_matches('#').to_string()
}

fn issue_state(state: IssueState) -> &'static str {
    match state {
        IssueState::Open => "open",
        IssueState::Closed => "closed",
    }
}

fn ensure_success(label: &str, output: &std::process::Output) -> Result<()> {
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        bail!("{label} failed with exit code {:?}", output.status.code());
    }
    bail!("{label} failed: {stderr}");
}
