use anyhow::{bail, Context, Result};
use clap::Args;
use colored::Colorize;
use serde::Deserialize;
use serde_yaml::Value;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::validate::readiness;

const OPERATIONAL_YAML: &[&str] = &[
    "docs/project/handoffs.yaml",
    "docs/project/findings.yaml",
    "docs/project/releases.yaml",
];

#[derive(Args)]
pub struct PrGovernanceArgs {
    /// GitHub event payload path. Defaults to GITHUB_EVENT_PATH.
    #[arg(long)]
    pub event_path: Option<PathBuf>,
}

#[derive(Args)]
pub struct ReleaseGateArgs {
    /// GitHub event payload path. Defaults to GITHUB_EVENT_PATH.
    #[arg(long)]
    pub event_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct PullRequestEvent {
    pull_request: PullRequestPayload,
    repository: RepositoryPayload,
}

#[derive(Debug, Deserialize)]
struct RepositoryPayload {
    full_name: String,
}

#[derive(Debug, Deserialize)]
struct PullRequestPayload {
    number: u64,
    title: String,
    body: Option<String>,
    base: BranchRef,
    head: BranchRef,
    labels: Vec<LabelPayload>,
}

#[derive(Debug, Deserialize)]
struct BranchRef {
    #[serde(rename = "ref")]
    branch: String,
}

#[derive(Debug, Deserialize)]
struct LabelPayload {
    name: String,
}

pub fn run(workspace: &Path, args: PrGovernanceArgs) -> Result<()> {
    println!("{}", "=== Validate PR Governance ===".cyan().bold());
    crate::common::capability_contract::require_policy_enabled(
        workspace,
        "gh",
        "validate pr-governance",
    )?;
    let event = load_pr_event(args.event_path.as_deref())?;
    let governance =
        readiness::load_governance(&workspace.join(".github/github-governance.yaml"))?;

    validate_pr_metadata(&event, &governance)?;
    validate_commit_subjects(workspace, &event.pull_request.base.branch, &event.pull_request.head.branch)?;
    validate_immutable_governance_internal(workspace, &event, &governance)?;

    if event.pull_request.base.branch == "main" {
        validate_release_gate_internal(workspace, &event, &governance)?;
    }

    crate::audit::events::record_sensitive_action(
        workspace,
        "validate.pr-governance",
        "runtime-cli",
        "success",
        serde_json::json!({
            "repo": event.repository.full_name,
            "pr_number": event.pull_request.number,
            "base_ref": event.pull_request.base.branch,
            "head_ref": event.pull_request.head.branch
        }),
    )?;

    println!(
        "{} PR metadata, commit subjects, and release gate passed",
        "PASSED:".green().bold()
    );
    Ok(())
}

pub fn run_release_gate(workspace: &Path, args: ReleaseGateArgs) -> Result<()> {
    println!("{}", "=== Validate Release Gate ===".cyan().bold());
    crate::common::capability_contract::require_policy_enabled(
        workspace,
        "gh",
        "validate release-gate",
    )?;
    let event = load_pr_event(args.event_path.as_deref())?;
    let governance =
        readiness::load_governance(&workspace.join(".github/github-governance.yaml"))?;
    validate_immutable_governance_internal(workspace, &event, &governance)?;
    validate_release_gate_internal(workspace, &event, &governance)?;
    crate::audit::events::record_sensitive_action(
        workspace,
        "validate.release-gate",
        "runtime-cli",
        "success",
        serde_json::json!({
            "repo": event.repository.full_name,
            "pr_number": event.pull_request.number
        }),
    )?;
    println!(
        "{} Release gate requirements satisfied",
        "PASSED:".green().bold()
    );
    Ok(())
}

fn load_pr_event(path: Option<&Path>) -> Result<PullRequestEvent> {
    let resolved = path
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("GITHUB_EVENT_PATH").map(PathBuf::from))
        .context("missing PR event payload; pass --event-path or set GITHUB_EVENT_PATH")?;
    let raw = std::fs::read_to_string(&resolved)
        .with_context(|| format!("reading {}", resolved.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing {}", resolved.display()))
}

fn validate_pr_metadata(event: &PullRequestEvent, governance: &Value) -> Result<()> {
    let pr = &event.pull_request;
    let base = pr.base.branch.as_str();
    let head = pr.head.branch.as_str();
    let body = pr.body.as_deref().unwrap_or("");
    let labels = pr
        .labels
        .iter()
        .map(|label| label.name.as_str())
        .collect::<Vec<_>>();

    let task_branch = regex::Regex::new(
        r"^(backend|frontend|qa|arch|ux|product|ops|techlead)\/((gh-\d+)|(\d+))-[a-z0-9][a-z0-9._-]*$",
    )?;
    let normal_title = regex::Regex::new(
        r"^(feat|fix|chore|docs|refactor|test|ci|build|perf|revert)\((backend|frontend|qa|arch|ux|product|ops|techlead)\): (GH-\d+|#\d+)\b.+",
    )?;
    let release_title = regex::Regex::new(r"^release\(ops\): (GH-\d+|#\d+)\b.+")?;
    let issue_ref = regex::Regex::new(r"(GH-\d+|#\d+)")?;
    let release_promotion = head == "develop" && base == "main";

    let mut errors = Vec::new();

    if !matches!(base, "develop" | "main") {
        errors.push(format!(
            "Base branch '{base}' is not allowed. Use develop for task PRs or main for release promotion."
        ));
    }
    if base == "main" && !release_promotion {
        errors.push(format!(
            "PRs targeting main must come from develop. Received '{head}' -> '{base}'."
        ));
    }
    if !release_promotion && !task_branch.is_match(head) {
        errors.push(format!(
            "Head branch '{head}' must match <role>/<issue-id>-slug."
        ));
    }
    if release_promotion {
        if !release_title.is_match(&pr.title) {
            errors.push(
                "Release promotion PR title must match: release(ops): GH-123 short summary"
                    .to_string(),
            );
        }
    } else if !normal_title.is_match(&pr.title) {
        errors.push(
            "PR title must follow Conventional Commits with role scope and issue reference."
                .to_string(),
        );
    }
    if !issue_ref.is_match(&format!("{}\n{body}", pr.title)) {
        errors.push(
            "PR title or body must reference a linked issue, for example GH-123 or Closes #123."
                .to_string(),
        );
    }

    for section in required_sections() {
        if !body.contains(section) {
            errors.push(format!("PR body is missing required section '{section}'."));
        }
    }

    let role_labels = governance_csv_set(governance, &["github", "labels", "role"])?;
    let kind_labels = governance_csv_set(governance, &["github", "labels", "kind"])?;
    let priority_labels = governance_csv_set(governance, &["github", "labels", "priority"])?;

    if !labels.iter().any(|label| role_labels.contains(*label)) {
        errors.push("PR is missing a role:* label.".to_string());
    }
    if !labels.iter().any(|label| kind_labels.contains(*label)) {
        errors.push("PR is missing a kind:* label.".to_string());
    }
    if !labels.iter().any(|label| priority_labels.contains(*label)) {
        errors.push("PR is missing a priority:* label.".to_string());
    }

    if !errors.is_empty() {
        bail!("{}", errors.join("\n"));
    }

    Ok(())
}

fn required_sections() -> &'static [&'static str] {
    &[
        "## Functional summary",
        "## Linked issue",
        "## Branches",
        "## Canonical docs touched",
        "## Validations run",
        "## Risks",
        "## Rollback",
        "## Handoffs / findings",
    ]
}

fn governance_csv_set(governance: &Value, path: &[&str]) -> Result<BTreeSet<String>> {
    let raw = readiness::yaml_string(governance, path)
        .with_context(|| format!("missing governance label contract at {}", path.join(".")))?;
    let values = readiness::parse_csv(&raw).into_iter().collect::<BTreeSet<_>>();
    if values.is_empty() {
        bail!("governance label contract {} must not be empty", path.join("."));
    }
    Ok(values)
}

fn validate_commit_subjects(workspace: &Path, base_ref: &str, head_ref: &str) -> Result<()> {
    if head_ref == "develop" && base_ref == "main" {
        println!("  Release promotion PR detected -- skipping per-commit task branch validation.");
        return Ok(());
    }

    let _ = Command::new("git")
        .args(["fetch", "origin", base_ref, "--depth=200"])
        .current_dir(workspace)
        .output();

    let range = commit_range(workspace, base_ref)?;
    let output = Command::new("git")
        .args(["log", "--format=%s", &range])
        .current_dir(workspace)
        .output()
        .with_context(|| format!("running `git log --format=%s {range}`"))?;

    if !output.status.success() {
        bail!(
            "failed to inspect commit subjects for range '{range}': {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let subjects = String::from_utf8(output.stdout)
        .context("git log returned non-UTF8 output")?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    if subjects.is_empty() {
        bail!("No commits found between HEAD and {range}");
    }

    let regex = regex::Regex::new(
        r"^(feat|fix|chore|docs|refactor|test|ci|build|perf|revert)\((backend|frontend|qa|arch|ux|product|ops|techlead)\): (GH-[0-9]+|#[0-9]+) .+",
    )?;
    let invalid = subjects
        .iter()
        .filter(|subject| !regex.is_match(subject))
        .cloned()
        .collect::<Vec<_>>();
    if !invalid.is_empty() {
        bail!("Invalid commit subject(s):\n{}", invalid.join("\n"));
    }

    Ok(())
}

fn commit_range(workspace: &Path, base_ref: &str) -> Result<String> {
    for candidate in [format!("origin/{base_ref}"), base_ref.to_string()] {
        let status = Command::new("git")
            .args(["rev-parse", "--verify", &candidate])
            .current_dir(workspace)
            .output()
            .with_context(|| format!("running `git rev-parse --verify {candidate}`"))?;
        if status.status.success() {
            return Ok(format!("{candidate}..HEAD"));
        }
    }
    Ok("HEAD".to_string())
}

fn validate_release_gate_internal(
    workspace: &Path,
    event: &PullRequestEvent,
    governance: &Value,
) -> Result<()> {
    if event.pull_request.base.branch != "main" {
        println!(
            "  Base branch is '{}' - release gate applies only to PRs targeting main.",
            event.pull_request.base.branch
        );
        return Ok(());
    }

    let readiness_status =
        readiness::yaml_string(governance, &["readiness", "status"]).unwrap_or_default();
    if readiness_status != readiness::READY_STATUS {
        bail!(
            "PRs targeting main require readiness.status=production-ready. Found '{}'.",
            if readiness_status.is_empty() {
                "unknown"
            } else {
                &readiness_status
            }
        );
    }

    if !readiness::yaml_bool(governance, &["github", "branch_protection", "enabled"])
        .unwrap_or(false)
    {
        bail!("Workspace is marked production-ready but github.branch_protection.enabled is not true.");
    }

    let reviewer_logins = configured_reviewer_logins(
        governance,
        &["github", "release_gate", "reviewer_logins"],
        "production-ready requires real github.release_gate.reviewer_logins values in .github/github-governance.yaml",
    )?;
    let approval_quorum = configured_approval_quorum(
        governance,
        &["github", "release_gate", "approval_quorum"],
        "github.release_gate.approval_quorum",
        reviewer_logins.len(),
        Some(6),
    )?;

    readiness::validate_remote_governance(workspace, governance)?;

    let latest_by_reviewer = latest_review_states(workspace, event)?;

    let approved_reviewers = approved_reviewers(&reviewer_logins, &latest_by_reviewer);
    if approved_reviewers.len() < approval_quorum as usize {
        bail!(
            "production-ready requires {} release-gate approval(s) from github.release_gate.reviewer_logins. Approved: {}. Allowed: {}",
            approval_quorum,
            format_login_list(&approved_reviewers),
            reviewer_logins.join(", ")
        );
    }

    println!(
        "  Release gate approvals: {} of {} required ({})",
        approved_reviewers.len(),
        approval_quorum,
        approved_reviewers.join(", ")
    );

    Ok(())
}

fn validate_immutable_governance_internal(
    workspace: &Path,
    event: &PullRequestEvent,
    governance: &Value,
) -> Result<()> {
    let immutable_required = readiness::yaml_bool(
        governance,
        &["github", "immutable_governance", "required"],
    )
    .unwrap_or(false);
    if !immutable_required {
        return Ok(());
    }

    let immutable_manifest = load_immutable_manifest(workspace)?;
    if immutable_manifest.is_empty() {
        bail!("immutable governance enforcement requires non-empty .github/immutable-files.txt");
    }

    let changed_files = pr_changed_files(workspace, event)?;
    let immutable_hits = changed_files
        .into_iter()
        .filter(|path| immutable_manifest.contains(path))
        .filter(|path| !OPERATIONAL_YAML.contains(&path.as_str()))
        .collect::<Vec<_>>();

    if immutable_hits.is_empty() {
        return Ok(());
    }

    let reviewer_logins = configured_reviewer_logins(
        governance,
        &["github", "immutable_governance", "reviewer_logins"],
        "immutable governance changes require real github.immutable_governance.reviewer_logins values in .github/github-governance.yaml",
    )?;
    let approval_quorum = configured_approval_quorum(
        governance,
        &["github", "immutable_governance", "approval_quorum"],
        "github.immutable_governance.approval_quorum",
        reviewer_logins.len(),
        None,
    )?;

    let required_labels = readiness::parse_csv(
        &readiness::yaml_string(governance, &["github", "immutable_governance", "required_labels"])
            .unwrap_or_default(),
    );
    let current_labels = event
        .pull_request
        .labels
        .iter()
        .map(|label| label.name.as_str())
        .collect::<BTreeSet<_>>();
    let missing_labels = required_labels
        .iter()
        .filter(|label| !current_labels.contains(label.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !missing_labels.is_empty() {
        bail!(
            "immutable governance changes require labels [{}]. Missing: {}",
            required_labels.join(", "),
            missing_labels.join(", ")
        );
    }

    let latest_by_reviewer = latest_review_states(workspace, event)?;
    let approved_reviewers = approved_reviewers(&reviewer_logins, &latest_by_reviewer);
    if approved_reviewers.len() < approval_quorum as usize {
        bail!(
            "immutable governance changes require {} approval(s) from github.immutable_governance.reviewer_logins. Approved: {}. Allowed: {}",
            approval_quorum,
            format_login_list(&approved_reviewers),
            reviewer_logins.join(", ")
        );
    }

    println!(
        "  Immutable governance approvals: {} of {} required ({}) for {}",
        approved_reviewers.len(),
        approval_quorum,
        approved_reviewers.join(", "),
        immutable_hits.join(", ")
    );
    Ok(())
}

fn load_immutable_manifest(workspace: &Path) -> Result<BTreeSet<String>> {
    let manifest_path = workspace.join(".github/immutable-files.txt");
    if !manifest_path.is_file() {
        bail!(
            "immutable governance enforcement requires {}",
            manifest_path.display()
        );
    }
    let raw = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    Ok(raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(normalize_repo_path)
        .collect())
}

fn pr_changed_files(workspace: &Path, event: &PullRequestEvent) -> Result<Vec<String>> {
    let governance =
        readiness::load_governance(&workspace.join(".github/github-governance.yaml"))?;
    let mut page = 1;
    let mut files = Vec::new();
    loop {
        let response = crate::github_api::api_get_json(
            &governance,
            &format!(
                "repos/{}/pulls/{}/files?per_page=100&page={page}",
                event.repository.full_name, event.pull_request.number
            ),
        )
        .with_context(|| {
            format!(
                "listing changed files for PR #{} via GitHub API",
                event.pull_request.number
            )
        })?;
        let batch = response.as_array().cloned().unwrap_or_default();
        if batch.is_empty() {
            break;
        }
        for entry in &batch {
            let Some(filename) = entry["filename"].as_str() else {
                continue;
            };
            files.push(normalize_repo_path(filename));
        }
        if batch.len() < 100 {
            break;
        }
        page += 1;
    }
    Ok(files)
}

fn latest_review_states(
    workspace: &Path,
    event: &PullRequestEvent,
) -> Result<HashMap<String, String>> {
    let governance =
        readiness::load_governance(&workspace.join(".github/github-governance.yaml"))?;
    let reviews = crate::github_api::api_get_json(
        &governance,
        &format!(
            "repos/{}/pulls/{}/reviews",
            event.repository.full_name, event.pull_request.number
        ),
    )
    .context("listing PR reviews via GitHub API")?;

    let review_array = reviews.as_array().cloned().unwrap_or_default();
    let mut latest_by_reviewer = HashMap::new();
    for review in review_array {
        let login = review["user"]["login"].as_str().unwrap_or("").trim().to_string();
        let state = review["state"].as_str().unwrap_or("").trim().to_string();
        if !login.is_empty() {
            latest_by_reviewer.insert(login, state);
        }
    }
    Ok(latest_by_reviewer)
}

fn normalize_repo_path(path: &str) -> String {
    let mut normalized = path.replace('\\', "/");
    if let Some(stripped) = normalized.strip_prefix("./") {
        normalized = stripped.to_string();
    }
    normalized.trim_start_matches('/').to_string()
}

fn configured_reviewer_logins(governance: &Value, path: &[&str], error_message: &str) -> Result<Vec<String>> {
    let mut unique = BTreeSet::new();
    let reviewer_logins = readiness::parse_csv(
        &readiness::yaml_string(governance, path).unwrap_or_default(),
    )
    .into_iter()
    .filter(|login| unique.insert(login.clone()))
    .collect::<Vec<_>>();

    if reviewer_logins.is_empty()
        || reviewer_logins
            .iter()
            .any(|value| value.contains("REPLACE_ME") || value.contains("team-"))
    {
        bail!("{error_message}");
    }

    Ok(reviewer_logins)
}

fn configured_approval_quorum(
    governance: &Value,
    path: &[&str],
    label: &str,
    reviewer_count: usize,
    max: Option<u64>,
) -> Result<u64> {
    if reviewer_count == 0 {
        bail!("{label} requires at least one configured reviewer login");
    }

    let quorum = match crate::github_api::yaml_value(governance, path) {
        None => 1,
        Some(value) => value
            .as_u64()
            .with_context(|| format!("{label} must be an integer"))?,
    };
    if quorum == 0 {
        bail!("{label} must be at least 1");
    }
    if quorum > reviewer_count as u64 {
        bail!(
            "{label}={quorum} exceeds the {reviewer_count} configured reviewer login(s)"
        );
    }
    if let Some(maximum) = max {
        if quorum > maximum {
            bail!(
                "{label}={quorum} exceeds the GitHub branch-protection maximum of {maximum} approving reviews"
            );
        }
    }

    Ok(quorum)
}

fn approved_reviewers(
    reviewer_logins: &[String],
    latest_by_reviewer: &HashMap<String, String>,
) -> Vec<String> {
    reviewer_logins
        .iter()
        .filter(|login| {
            latest_by_reviewer
                .get(*login)
                .map(|state| state == "APPROVED")
                .unwrap_or(false)
        })
        .cloned()
        .collect()
}

fn format_login_list(logins: &[String]) -> String {
    if logins.is_empty() {
        "none".to_string()
    } else {
        logins.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::{approved_reviewers, configured_approval_quorum, format_login_list};
    use serde_yaml::Value;
    use std::collections::HashMap;

    #[test]
    fn approved_reviewers_respect_threshold_candidates() {
        let reviewers = vec!["devops-login".to_string(), "backup-login".to_string()];
        let latest_by_reviewer = HashMap::from([
            ("devops-login".to_string(), "APPROVED".to_string()),
            ("backup-login".to_string(), "COMMENTED".to_string()),
            ("outsider".to_string(), "APPROVED".to_string()),
        ]);

        let approved = approved_reviewers(&reviewers, &latest_by_reviewer);
        assert_eq!(approved, vec!["devops-login".to_string()]);
        assert_eq!(format_login_list(&approved), "devops-login");
    }

    #[test]
    fn configured_approval_quorum_defaults_to_one_when_missing() {
        let governance: Value = serde_yaml::from_str(
            "github:\n  release_gate:\n    reviewer_logins: devops-login\n",
        )
        .expect("failed to parse governance fixture");

        let quorum = configured_approval_quorum(
            &governance,
            &["github", "release_gate", "approval_quorum"],
            "github.release_gate.approval_quorum",
            1,
            Some(6),
        )
        .expect("missing quorum should default to 1");

        assert_eq!(quorum, 1);
    }

    #[test]
    fn configured_approval_quorum_rejects_values_above_reviewer_count() {
        let governance: Value = serde_yaml::from_str(
            "github:\n  immutable_governance:\n    approval_quorum: 3\n",
        )
        .expect("failed to parse governance fixture");

        let error = configured_approval_quorum(
            &governance,
            &["github", "immutable_governance", "approval_quorum"],
            "github.immutable_governance.approval_quorum",
            2,
            None,
        )
        .expect_err("quorum larger than reviewer count should fail");

        assert!(error
            .to_string()
            .contains("exceeds the 2 configured reviewer login(s)"));
    }
}
