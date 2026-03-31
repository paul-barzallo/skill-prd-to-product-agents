use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand, ValueEnum};
use colored::Colorize;
use serde_json::json;
use serde_yaml::Value as YamlValue;
use std::path::Path;
use std::process::Command;

#[derive(Subcommand)]
pub enum GithubCommands {
    /// GitHub issue operations routed through the runtime CLI
    Issue {
        #[command(subcommand)]
        sub: GithubIssueCommands,
    },
    /// GitHub pull-request operations routed through the runtime CLI
    Pr {
        #[command(subcommand)]
        sub: GithubPrCommands,
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

#[derive(Subcommand)]
pub enum GithubPrCommands {
    /// Create a GitHub pull request
    Create(CreatePrArgs),
    /// Update a GitHub pull request
    Update(UpdatePrArgs),
    /// Add a comment to a GitHub pull request
    Comment(CommentPrArgs),
    /// Add or remove labels on a GitHub pull request
    Label(LabelPrArgs),
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

#[derive(Args)]
pub struct CreatePrArgs {
    #[arg(long)]
    pub title: String,
    #[arg(long)]
    pub body: String,
    #[arg(long)]
    pub base: String,
    #[arg(long)]
    pub head: Option<String>,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long = "label")]
    pub labels: Vec<String>,
    #[arg(long)]
    pub draft: bool,
}

#[derive(Args)]
pub struct UpdatePrArgs {
    #[arg(long)]
    pub pr_ref: String,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long)]
    pub base: Option<String>,
    #[arg(long = "add-label")]
    pub add_labels: Vec<String>,
    #[arg(long = "remove-label")]
    pub remove_labels: Vec<String>,
    #[arg(long)]
    pub ready: bool,
    #[arg(long)]
    pub draft: bool,
}

#[derive(Args)]
pub struct CommentPrArgs {
    #[arg(long)]
    pub pr_ref: String,
    #[arg(long)]
    pub body: String,
    #[arg(long)]
    pub repo: Option<String>,
}

#[derive(Args)]
pub struct LabelPrArgs {
    #[arg(long)]
    pub pr_ref: String,
    #[arg(long)]
    pub repo: Option<String>,
    #[arg(long = "add")]
    pub add: Vec<String>,
    #[arg(long = "remove")]
    pub remove: Vec<String>,
}

enum MutationBackend {
    CoreLocal,
    Enterprise { governance: YamlValue },
}

pub fn run(workspace: &Path, sub: GithubCommands) -> Result<()> {
    let backend = mutation_backend(workspace)?;
    match sub {
        GithubCommands::Issue { sub } => match sub {
            GithubIssueCommands::Create(args) => create_issue(workspace, &backend, args),
            GithubIssueCommands::Update(args) => update_issue(workspace, &backend, args),
            GithubIssueCommands::Comment(args) => comment_issue(workspace, &backend, args),
            GithubIssueCommands::Label(args) => label_issue(workspace, &backend, args),
        },
        GithubCommands::Pr { sub } => match sub {
            GithubPrCommands::Create(args) => create_pr(workspace, &backend, args),
            GithubPrCommands::Update(args) => update_pr(workspace, &backend, args),
            GithubPrCommands::Comment(args) => comment_pr(workspace, &backend, args),
            GithubPrCommands::Label(args) => label_pr(workspace, &backend, args),
        },
    }
}

fn create_issue(workspace: &Path, backend: &MutationBackend, args: CreateIssueArgs) -> Result<()> {
    println!("{}", "=== GitHub Issue Create ===".cyan().bold());
    let repo = resolve_repo(workspace, backend, args.repo.as_deref())?;
    match backend {
        MutationBackend::CoreLocal => {
            let mut command = gh_command(workspace, &repo, "issue", "create");
            command.args(["--title", &args.title, "--body", &args.body]);
            for label in &args.labels {
                command.args(["--label", label]);
            }
            for assignee in &args.assignees {
                command.args(["--assignee", assignee]);
            }
            let output = command.output().context("running gh issue create")?;
            ensure_success("gh issue create", &output)?;
            println!(
                "{} {}",
                "OK:".green().bold(),
                String::from_utf8_lossy(&output.stdout).trim()
            );
        }
        MutationBackend::Enterprise { governance } => {
            let response = crate::github_api::api_post_json(
                governance,
                &format!("repos/{repo}/issues"),
                &json!({
                    "title": args.title,
                    "body": args.body,
                    "labels": args.labels,
                    "assignees": args.assignees,
                }),
            )?;
            println!(
                "{} issue #{} {}",
                "OK:".green().bold(),
                response["number"].as_u64().unwrap_or_default(),
                response["html_url"].as_str().unwrap_or_default()
            );
        }
    }
    record_action(
        workspace,
        "github.issue.create",
        json!({
            "repo": repo,
            "title": args.title,
            "labels": args.labels,
            "assignees": args.assignees
        }),
    )
}

fn update_issue(workspace: &Path, backend: &MutationBackend, args: UpdateIssueArgs) -> Result<()> {
    println!("{}", "=== GitHub Issue Update ===".cyan().bold());
    let repo = resolve_repo(workspace, backend, args.repo.as_deref())?;
    let issue_ref = normalize_issue_ref(&args.issue_ref);
    let needs_edit = args.title.is_some()
        || args.body.is_some()
        || !args.add_labels.is_empty()
        || !args.remove_labels.is_empty()
        || args.state.is_some();
    if !needs_edit {
        bail!("github issue update requires at least one field change or --state");
    }

    match backend {
        MutationBackend::CoreLocal => {
            let mut command = gh_command(workspace, &repo, "issue", "edit");
            command.arg(&issue_ref);
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
            if let Some(state) = args.state {
                apply_issue_state_local(workspace, &repo, &issue_ref, state)?;
            }
        }
        MutationBackend::Enterprise { governance } => {
            let mut payload = serde_json::Map::new();
            if let Some(title) = &args.title {
                payload.insert("title".to_string(), json!(title));
            }
            if let Some(body) = &args.body {
                payload.insert("body".to_string(), json!(body));
            }
            if let Some(state) = args.state {
                payload.insert("state".to_string(), json!(issue_state(state)));
            }
            if !payload.is_empty() {
                let _ = crate::github_api::api_patch_json(
                    governance,
                    &format!("repos/{repo}/issues/{issue_ref}"),
                    &serde_json::Value::Object(payload),
                )?;
            }
            add_issue_labels_api(governance, &repo, &issue_ref, &args.add_labels)?;
            remove_issue_labels_api(governance, &repo, &issue_ref, &args.remove_labels)?;
        }
    }

    record_action(
        workspace,
        "github.issue.update",
        json!({
            "repo": repo,
            "issue_ref": issue_ref,
            "state": args.state.map(issue_state),
            "add_labels": args.add_labels,
            "remove_labels": args.remove_labels
        }),
    )?;
    println!("{} updated issue {}", "OK:".green().bold(), issue_ref);
    Ok(())
}

fn comment_issue(workspace: &Path, backend: &MutationBackend, args: CommentIssueArgs) -> Result<()> {
    println!("{}", "=== GitHub Issue Comment ===".cyan().bold());
    let repo = resolve_repo(workspace, backend, args.repo.as_deref())?;
    let issue_ref = normalize_issue_ref(&args.issue_ref);
    match backend {
        MutationBackend::CoreLocal => {
            let output = gh_command(workspace, &repo, "issue", "comment")
                .args([&issue_ref, "--body", &args.body])
                .output()
                .context("running gh issue comment")?;
            ensure_success("gh issue comment", &output)?;
        }
        MutationBackend::Enterprise { governance } => {
            let _ = crate::github_api::api_post_json(
                governance,
                &format!("repos/{repo}/issues/{issue_ref}/comments"),
                &json!({ "body": args.body }),
            )?;
        }
    }
    record_action(
        workspace,
        "github.issue.comment",
        json!({
            "repo": repo,
            "issue_ref": issue_ref
        }),
    )?;
    println!("{} commented on issue {}", "OK:".green().bold(), issue_ref);
    Ok(())
}

fn label_issue(workspace: &Path, backend: &MutationBackend, args: LabelIssueArgs) -> Result<()> {
    println!("{}", "=== GitHub Issue Label ===".cyan().bold());
    if args.add.is_empty() && args.remove.is_empty() {
        bail!("github issue label requires at least one --add or --remove value");
    }
    let repo = resolve_repo(workspace, backend, args.repo.as_deref())?;
    let issue_ref = normalize_issue_ref(&args.issue_ref);
    match backend {
        MutationBackend::CoreLocal => {
            let mut command = gh_command(workspace, &repo, "issue", "edit");
            command.arg(&issue_ref);
            for label in &args.add {
                command.args(["--add-label", label]);
            }
            for label in &args.remove {
                command.args(["--remove-label", label]);
            }
            let output = command.output().context("running gh issue edit for labels")?;
            ensure_success("gh issue edit", &output)?;
        }
        MutationBackend::Enterprise { governance } => {
            add_issue_labels_api(governance, &repo, &issue_ref, &args.add)?;
            remove_issue_labels_api(governance, &repo, &issue_ref, &args.remove)?;
        }
    }
    record_action(
        workspace,
        "github.issue.label",
        json!({
            "repo": repo,
            "issue_ref": issue_ref,
            "add": args.add,
            "remove": args.remove
        }),
    )?;
    println!("{} labels updated for issue {}", "OK:".green().bold(), issue_ref);
    Ok(())
}

fn create_pr(workspace: &Path, backend: &MutationBackend, args: CreatePrArgs) -> Result<()> {
    println!("{}", "=== GitHub PR Create ===".cyan().bold());
    let repo = resolve_repo(workspace, backend, args.repo.as_deref())?;
    match backend {
        MutationBackend::CoreLocal => {
            let mut command = gh_command(workspace, &repo, "pr", "create");
            command.args(["--title", &args.title, "--body", &args.body, "--base", &args.base]);
            if let Some(head) = &args.head {
                command.args(["--head", head]);
            }
            if args.draft {
                command.arg("--draft");
            }
            for label in &args.labels {
                command.args(["--label", label]);
            }
            let output = command.output().context("running gh pr create")?;
            ensure_success("gh pr create", &output)?;
            println!(
                "{} {}",
                "OK:".green().bold(),
                String::from_utf8_lossy(&output.stdout).trim()
            );
        }
        MutationBackend::Enterprise { governance } => {
            let head = args
                .head
                .clone()
                .or_else(|| current_branch_name(workspace))
                .context("github pr create requires --head or an active branch")?;
            let response = crate::github_api::api_post_json(
                governance,
                &format!("repos/{repo}/pulls"),
                &json!({
                    "title": args.title,
                    "body": args.body,
                    "base": args.base,
                    "head": head,
                    "draft": args.draft
                }),
            )?;
            if !args.labels.is_empty() {
                add_issue_labels_api(
                    governance,
                    &repo,
                    &response["number"].as_u64().unwrap_or_default().to_string(),
                    &args.labels,
                )?;
            }
            println!(
                "{} PR #{} {}",
                "OK:".green().bold(),
                response["number"].as_u64().unwrap_or_default(),
                response["html_url"].as_str().unwrap_or_default()
            );
        }
    }
    record_action(
        workspace,
        "github.pr.create",
        json!({
            "repo": repo,
            "title": args.title,
            "base": args.base,
            "head": args.head,
            "draft": args.draft,
            "labels": args.labels
        }),
    )
}

fn update_pr(workspace: &Path, backend: &MutationBackend, args: UpdatePrArgs) -> Result<()> {
    println!("{}", "=== GitHub PR Update ===".cyan().bold());
    if args.ready && args.draft {
        bail!("github pr update cannot use --ready and --draft together");
    }
    let repo = resolve_repo(workspace, backend, args.repo.as_deref())?;
    let pr_ref = normalize_pr_ref(&args.pr_ref);
    let needs_edit = args.title.is_some()
        || args.body.is_some()
        || args.base.is_some()
        || !args.add_labels.is_empty()
        || !args.remove_labels.is_empty();
    if !needs_edit && !args.ready && !args.draft {
        bail!("github pr update requires at least one field change, --ready, or --draft");
    }

    match backend {
        MutationBackend::CoreLocal => {
            if needs_edit {
                let mut command = gh_command(workspace, &repo, "pr", "edit");
                command.arg(&pr_ref);
                if let Some(title) = &args.title {
                    command.args(["--title", title]);
                }
                if let Some(body) = &args.body {
                    command.args(["--body", body]);
                }
                if let Some(base) = &args.base {
                    command.args(["--base", base]);
                }
                for label in &args.add_labels {
                    command.args(["--add-label", label]);
                }
                for label in &args.remove_labels {
                    command.args(["--remove-label", label]);
                }
                let output = command.output().context("running gh pr edit")?;
                ensure_success("gh pr edit", &output)?;
            }

            if args.ready || args.draft {
                let mut command = gh_command(workspace, &repo, "pr", "ready");
                command.arg(&pr_ref);
                if args.draft {
                    command.arg("--undo");
                }
                let output = command.output().context("running gh pr ready")?;
                ensure_success("gh pr ready", &output)?;
            }
        }
        MutationBackend::Enterprise { governance } => {
            let mut payload = serde_json::Map::new();
            if let Some(title) = &args.title {
                payload.insert("title".to_string(), json!(title));
            }
            if let Some(body) = &args.body {
                payload.insert("body".to_string(), json!(body));
            }
            if let Some(base) = &args.base {
                payload.insert("base".to_string(), json!(base));
            }
            if !payload.is_empty() {
                let _ = crate::github_api::api_patch_json(
                    governance,
                    &format!("repos/{repo}/pulls/{pr_ref}"),
                    &serde_json::Value::Object(payload),
                )?;
            }
            add_issue_labels_api(governance, &repo, &pr_ref, &args.add_labels)?;
            remove_issue_labels_api(governance, &repo, &pr_ref, &args.remove_labels)?;
            if args.ready || args.draft {
                toggle_pr_draft_api(governance, &repo, &pr_ref, args.draft)?;
            }
        }
    }

    record_action(
        workspace,
        "github.pr.update",
        json!({
            "repo": repo,
            "pr_ref": pr_ref,
            "base": args.base,
            "add_labels": args.add_labels,
            "remove_labels": args.remove_labels,
            "ready": args.ready,
            "draft": args.draft
        }),
    )?;
    println!("{} updated PR {}", "OK:".green().bold(), pr_ref);
    Ok(())
}

fn comment_pr(workspace: &Path, backend: &MutationBackend, args: CommentPrArgs) -> Result<()> {
    println!("{}", "=== GitHub PR Comment ===".cyan().bold());
    let repo = resolve_repo(workspace, backend, args.repo.as_deref())?;
    let pr_ref = normalize_pr_ref(&args.pr_ref);
    match backend {
        MutationBackend::CoreLocal => {
            let output = gh_command(workspace, &repo, "pr", "comment")
                .args([&pr_ref, "--body", &args.body])
                .output()
                .context("running gh pr comment")?;
            ensure_success("gh pr comment", &output)?;
        }
        MutationBackend::Enterprise { governance } => {
            let _ = crate::github_api::api_post_json(
                governance,
                &format!("repos/{repo}/issues/{pr_ref}/comments"),
                &json!({ "body": args.body }),
            )?;
        }
    }
    record_action(
        workspace,
        "github.pr.comment",
        json!({
            "repo": repo,
            "pr_ref": pr_ref
        }),
    )?;
    println!("{} commented on PR {}", "OK:".green().bold(), pr_ref);
    Ok(())
}

fn label_pr(workspace: &Path, backend: &MutationBackend, args: LabelPrArgs) -> Result<()> {
    println!("{}", "=== GitHub PR Label ===".cyan().bold());
    if args.add.is_empty() && args.remove.is_empty() {
        bail!("github pr label requires at least one --add or --remove value");
    }
    let repo = resolve_repo(workspace, backend, args.repo.as_deref())?;
    let pr_ref = normalize_pr_ref(&args.pr_ref);
    match backend {
        MutationBackend::CoreLocal => {
            let mut command = gh_command(workspace, &repo, "pr", "edit");
            command.arg(&pr_ref);
            for label in &args.add {
                command.args(["--add-label", label]);
            }
            for label in &args.remove {
                command.args(["--remove-label", label]);
            }
            let output = command.output().context("running gh pr edit for labels")?;
            ensure_success("gh pr edit", &output)?;
        }
        MutationBackend::Enterprise { governance } => {
            add_issue_labels_api(governance, &repo, &pr_ref, &args.add)?;
            remove_issue_labels_api(governance, &repo, &pr_ref, &args.remove)?;
        }
    }
    record_action(
        workspace,
        "github.pr.label",
        json!({
            "repo": repo,
            "pr_ref": pr_ref,
            "add": args.add,
            "remove": args.remove
        }),
    )?;
    println!("{} labels updated for PR {}", "OK:".green().bold(), pr_ref);
    Ok(())
}

fn mutation_backend(workspace: &Path) -> Result<MutationBackend> {
    let governance = load_governance(workspace)?;
    match crate::github_api::operating_profile(&governance)? {
        crate::github_api::OperatingProfile::Enterprise => {
            crate::github_api::require_enterprise_api_mode(&governance)?;
            Ok(MutationBackend::Enterprise { governance })
        }
        crate::github_api::OperatingProfile::CoreLocal => {
            require_gh_enabled(workspace, "github mutation")?;
            Ok(MutationBackend::CoreLocal)
        }
    }
}

fn load_governance(workspace: &Path) -> Result<YamlValue> {
    crate::validate::readiness::load_governance(&workspace.join(".github/github-governance.yaml"))
}

fn require_gh_enabled(workspace: &Path, action: &str) -> Result<()> {
    crate::common::capability_contract::require_policy_enabled(workspace, "gh", action)
}

fn gh_command<'a>(workspace: &'a Path, repo: &'a str, subject: &'a str, action: &'a str) -> Command {
    let mut command = Command::new("gh");
    command
        .current_dir(workspace)
        .args([subject, action, "--repo", repo]);
    command
}

fn resolve_repo(workspace: &Path, backend: &MutationBackend, explicit: Option<&str>) -> Result<String> {
    if let Some(repo) = explicit {
        return Ok(repo.trim().to_string());
    }

    match backend {
        MutationBackend::Enterprise { governance } => crate::github_api::repository_full_name(governance),
        MutationBackend::CoreLocal => {
            if let Ok(governance) = load_governance(workspace) {
                if let Ok(repo) = crate::github_api::repository_full_name(&governance) {
                    if !repo.contains("REPLACE_ME") {
                        return Ok(repo);
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
    }
}

fn add_issue_labels_api(
    governance: &YamlValue,
    repo: &str,
    issue_ref: &str,
    labels: &[String],
) -> Result<()> {
    if labels.is_empty() {
        return Ok(());
    }
    let _ = crate::github_api::api_post_json(
        governance,
        &format!("repos/{repo}/issues/{issue_ref}/labels"),
        &json!({ "labels": labels }),
    )?;
    Ok(())
}

fn remove_issue_labels_api(
    governance: &YamlValue,
    repo: &str,
    issue_ref: &str,
    labels: &[String],
) -> Result<()> {
    for label in labels {
        crate::github_api::api_delete(
            governance,
            &format!(
                "repos/{repo}/issues/{issue_ref}/labels/{}",
                urlencoding::encode(label)
            ),
        )?;
    }
    Ok(())
}

fn toggle_pr_draft_api(
    governance: &YamlValue,
    repo: &str,
    pr_ref: &str,
    draft: bool,
) -> Result<()> {
    let pr = crate::github_api::api_get_json(governance, &format!("repos/{repo}/pulls/{pr_ref}"))?;
    let node_id = pr["node_id"]
        .as_str()
        .map(str::to_string)
        .filter(|value| !value.is_empty())
        .context("pull request node_id missing from GitHub API response")?;
    let query = if draft {
        "mutation($id:ID!){ convertPullRequestToDraft(input:{pullRequestId:$id}) { pullRequest { number isDraft } } }"
    } else {
        "mutation($id:ID!){ markPullRequestReadyForReview(input:{pullRequestId:$id}) { pullRequest { number isDraft } } }"
    };
    let _ = crate::github_api::graphql(governance, query, json!({ "id": node_id }))?;
    Ok(())
}

fn apply_issue_state_local(workspace: &Path, repo: &str, issue_ref: &str, state: IssueState) -> Result<()> {
    let subcommand = match state {
        IssueState::Open => "reopen",
        IssueState::Closed => "close",
    };
    let output = gh_command(workspace, repo, "issue", subcommand)
        .arg(issue_ref)
        .output()
        .with_context(|| format!("running gh issue {subcommand}"))?;
    ensure_success(&format!("gh issue {subcommand}"), &output)?;
    Ok(())
}

fn record_action(workspace: &Path, action: &str, payload: serde_json::Value) -> Result<()> {
    crate::audit::events::record_sensitive_action(workspace, action, "runtime-cli", "success", payload)
}

fn normalize_issue_ref(input: &str) -> String {
    let trimmed = input.trim();
    if let Some(rest) = trimmed.strip_prefix("GH-") {
        return rest.to_string();
    }
    trimmed.trim_start_matches('#').to_string()
}

fn normalize_pr_ref(input: &str) -> String {
    input.trim().trim_start_matches('#').to_string()
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

fn current_branch_name(workspace: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["-C", &workspace.to_string_lossy(), "branch", "--show-current"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!branch.is_empty()).then_some(branch)
}
