use anyhow::{bail, Context, Result};
use colored::Colorize;
use regex::Regex;
use serde_json::Value as JsonValue;
use serde_yaml::Value;
use std::path::Path;
use std::process::Command;

use crate::common::capability_contract;
use crate::encoding::{self, EncodingArgs};
use crate::validate::finalize_validation;

pub(crate) const READY_STATUS: &str = "production-ready";

struct RemoteBranchProtectionRule {
    pattern: String,
    requires_approving_reviews: bool,
    required_approving_review_count: u64,
    requires_code_owner_reviews: bool,
    requires_conversation_resolution: bool,
}

pub fn run(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "validating workspace readiness");
    println!("{}", "=== Validate Readiness ===".cyan().bold());

    let mut errors = 0u32;
    let warnings = 0u32;

    println!("\n{}", "Checking structural workspace contract...".bold());
    match crate::validate::workspace::run(workspace) {
        Ok(()) => println!("  {} workspace structure", "✓".green()),
        Err(error) => {
            eprintln!("  {} workspace structure: {error}", "✗".red());
            errors += 1;
        }
    }

    println!("\n{}", "Checking governance readiness...".bold());
    match crate::validate::governance::run(workspace) {
        Ok(()) => println!("  {} governance contract", "✓".green()),
        Err(error) => {
            eprintln!("  {} governance contract: {error}", "✗".red());
            errors += 1;
        }
    }

    println!("\n{}", "Checking assembled agent sync...".bold());
    match crate::agents::verify_assembly(workspace) {
        Ok(()) => println!("  {} agent assembly", "✓".green()),
        Err(error) => {
            eprintln!("  {} agent assembly: {error}", "✗".red());
            errors += 1;
        }
    }

    println!("\n{}", "Checking encoding hygiene...".bold());
    match encoding::run(workspace, EncodingArgs { target: None }) {
        Ok(()) => println!("  {} encoding", "✓".green()),
        Err(error) => {
            eprintln!("  {} encoding: {error}", "✗".red());
            errors += 1;
        }
    }

    println!("\n{}", "Checking capability contract...".bold());
    if !capability_contract::capabilities_file_exists(workspace) {
        eprintln!(
            "  {} .github/workspace-capabilities.yaml missing",
            "✗".red()
        );
        errors += 1;
    } else {
        println!("  {} workspace-capabilities.yaml present", "✓".green());
        for capability in ["git", "gh", "sqlite", "reporting"] {
            match capability_contract::policy_enabled(workspace, capability) {
                Ok(Some(enabled)) => {
                    let icon = if enabled { "✓".green() } else { "!".yellow() };
                    let status = if enabled { "enabled" } else { "disabled" };
                    println!(
                        "  {} capabilities.{capability}.authorized.enabled = {status}",
                        icon
                    );
                }
                Ok(None) => {
                    eprintln!(
                        "  {} capabilities.{capability}.authorized.enabled missing",
                        "✗".red()
                    );
                    errors += 1;
                }
                Err(error) => {
                    eprintln!(
                        "  {} capabilities.{capability}.authorized.enabled unreadable: {error}",
                        "✗".red()
                    );
                    errors += 1;
                }
            }
        }
    }

    let gov_path = workspace.join(".github/github-governance.yaml");
    println!("\n{}", "Checking readiness state...".bold());
    let governance = match load_governance(&gov_path) {
        Ok(governance) => governance,
        Err(error) => {
            eprintln!("  {} governance load failed: {error}", "✗".red());
            return finalize_validation(
                "readiness",
                errors + 1,
                warnings,
                None,
                "Workspace is operationally ready",
            );
        }
    };

    let readiness = yaml_string(&governance, &["readiness", "status"]).unwrap_or_default();
    if readiness == READY_STATUS {
        println!("  {} readiness.status = {readiness}", "✓".green());
    } else {
        eprintln!(
            "  {} readiness.status = '{}' is not release-eligible; expected '{}'",
            "✗".red(),
            readiness,
            READY_STATUS
        );
        errors += 1;
    }

    println!("\n{}", "Checking remote governance...".bold());
    match validate_remote_governance(workspace, &governance) {
        Ok(()) => println!("  {} remote governance", "✓".green()),
        Err(error) => {
            eprintln!("  {} remote governance: {error}", "✗".red());
            errors += 1;
        }
    }

    finalize_validation(
        "readiness",
        errors,
        warnings,
        None,
        "Workspace is operationally ready",
    )
}

pub(crate) fn load_governance(path: &Path) -> Result<Value> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let parsed = serde_yaml::from_str::<Value>(&content)
        .with_context(|| format!("parsing {}", path.display()))?;
    Ok(parsed)
}

pub(crate) fn validate_remote_governance(workspace: &Path, governance: &Value) -> Result<()> {
    let project_enabled = yaml_bool(governance, &["github", "project", "enabled"]).unwrap_or(false);
    if project_enabled {
        bail!(
            "github.project.enabled=true is out of the current supported operational contract; keep github.project.enabled=false until GitHub Project support is implemented as a future extension"
        );
    }

    ensure_gh_policy_enabled(workspace)?;

    let owner = yaml_string(governance, &["github", "repository", "owner"])
        .context("github.repository.owner missing")?;
    let repo = yaml_string(governance, &["github", "repository", "name"])
        .context("github.repository.name missing")?;
    let repo_full_name = format!("{owner}/{repo}");

    let repo_view = run_gh_json(
        workspace,
        &["repo", "view", &repo_full_name, "--json", "nameWithOwner"],
    )
    .context("unable to resolve configured repository via gh")?;
    let remote_name = repo_view["nameWithOwner"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    if remote_name != repo_full_name {
        bail!(
            "configured repository mismatch: expected '{repo_full_name}', gh resolved '{remote_name}'"
        );
    }

    let branch_protection_enabled =
        yaml_bool(governance, &["github", "branch_protection", "enabled"]).unwrap_or(false);
    if !branch_protection_enabled {
        bail!("github.branch_protection.enabled must be true for production-ready");
    }

    let protected_branches = parse_csv(
        &yaml_string(
            governance,
            &["github", "branch_protection", "protected_branches"],
        )
        .unwrap_or_default(),
    );
    if protected_branches.is_empty() {
        bail!("github.branch_protection.protected_branches must list at least one branch");
    }

    let branch_rules = load_branch_protection_rules(workspace, &owner, &repo)?;
    let missing_branches = protected_branches
        .iter()
        .filter(|branch| {
            !branch_rules
                .iter()
                .any(|rule| rule_matches_branch(&rule.pattern, branch))
        })
        .cloned()
        .collect::<Vec<_>>();
    if !missing_branches.is_empty() {
        bail!(
            "remote branch protection rules do not cover: {}",
            missing_branches.join(", ")
        );
    }

    let require_code_owner_reviews = yaml_bool(
        governance,
        &["github", "branch_protection", "require_code_owner_review"],
    )
    .unwrap_or(false);
    let require_resolved_conversations = yaml_bool(
        governance,
        &["github", "branch_protection", "require_resolved_conversations"],
    )
    .unwrap_or(false);
    let require_release_gate_approval = yaml_bool(
        governance,
        &["github", "branch_protection", "require_release_gate_approval"],
    )
    .unwrap_or(false);

    let mut branch_rule_errors = Vec::new();
    for branch in &protected_branches {
        let Some(rule) = branch_rules
            .iter()
            .find(|rule| rule_matches_branch(&rule.pattern, branch))
        else {
            continue;
        };
        if require_code_owner_reviews && !rule.requires_code_owner_reviews {
            branch_rule_errors.push(format!(
                "{branch}: remote rule '{}' does not require CODEOWNERS review",
                rule.pattern
            ));
        }
        if require_resolved_conversations && !rule.requires_conversation_resolution {
            branch_rule_errors.push(format!(
                "{branch}: remote rule '{}' does not require resolved conversations",
                rule.pattern
            ));
        }
        if require_release_gate_approval {
            if !rule.requires_approving_reviews {
                branch_rule_errors.push(format!(
                    "{branch}: remote rule '{}' does not require approving reviews",
                    rule.pattern
                ));
            } else if rule.required_approving_review_count < 1 {
                branch_rule_errors.push(format!(
                    "{branch}: remote rule '{}' requires fewer than 1 approving review",
                    rule.pattern
                ));
            }
        }
    }
    if !branch_rule_errors.is_empty() {
        bail!(
            "remote branch protection is weaker than .github/github-governance.yaml requires:\n  {}",
            branch_rule_errors.join("\n  ")
        );
    }

    let reviewer_logins = parse_csv(
        &yaml_string(governance, &["github", "release_gate", "reviewer_logins"])
            .unwrap_or_default(),
    );
    if reviewer_logins.is_empty() {
        bail!("github.release_gate.reviewer_logins must contain at least one login");
    }
    for login in &reviewer_logins {
        run_gh_json(workspace, &["api", &format!("users/{login}")]).with_context(|| {
            format!("release gate login '{login}' is not readable via GitHub API")
        })?;
    }

    let immutable_required = yaml_bool(governance, &["github", "immutable_governance", "required"])
        .unwrap_or(false);
    if immutable_required {
        let immutable_logins = parse_csv(
            &yaml_string(governance, &["github", "immutable_governance", "reviewer_logins"])
                .unwrap_or_default(),
        );
        if immutable_logins.is_empty() {
            bail!("github.immutable_governance.reviewer_logins must contain at least one login");
        }
        for login in &immutable_logins {
            run_gh_json(workspace, &["api", &format!("users/{login}")]).with_context(|| {
                format!("immutable governance login '{login}' is not readable via GitHub API")
            })?;
        }
    }

    Ok(())
}

pub(crate) fn ensure_gh_policy_enabled(workspace: &Path) -> Result<()> {
    match capability_contract::policy_enabled(workspace, "gh")? {
        Some(true) => {}
        Some(false) => {
            bail!("capabilities.gh.authorized.enabled=false blocks production-ready readiness")
        }
        None => bail!("capabilities.gh.authorized.enabled missing from workspace capability contract"),
    }

    let output = Command::new("gh")
        .args(["auth", "status"])
        .current_dir(workspace)
        .output()
        .context("running `gh auth status`")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            bail!("gh auth status failed; production-ready readiness requires an authenticated gh session");
        }
        bail!(
            "gh auth status failed; production-ready readiness requires an authenticated gh session: {stderr}"
        );
    }

    Ok(())
}

fn load_branch_protection_rules(
    workspace: &Path,
    owner: &str,
    repo: &str,
) -> Result<Vec<RemoteBranchProtectionRule>> {
    let response = run_gh_json(
        workspace,
        &[
            "api",
            "graphql",
            "-f",
            "query=query($owner:String!,$name:String!){repository(owner:$owner,name:$name){branchProtectionRules(first:100){nodes{pattern requiresApprovingReviews requiredApprovingReviewCount requiresCodeOwnerReviews requiresConversationResolution}}}}",
            "-F",
            &format!("owner={owner}"),
            "-F",
            &format!("name={repo}"),
        ],
    )?;

    Ok(
        response["data"]["repository"]["branchProtectionRules"]["nodes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|node| {
                Some(RemoteBranchProtectionRule {
                    pattern: node["pattern"].as_str()?.to_string(),
                    requires_approving_reviews: node["requiresApprovingReviews"]
                        .as_bool()
                        .unwrap_or(false),
                    required_approving_review_count: node["requiredApprovingReviewCount"]
                        .as_u64()
                        .unwrap_or(0),
                    requires_code_owner_reviews: node["requiresCodeOwnerReviews"]
                        .as_bool()
                        .unwrap_or(false),
                    requires_conversation_resolution: node["requiresConversationResolution"]
                        .as_bool()
                        .unwrap_or(false),
                })
            })
            .collect(),
    )
}

pub(crate) fn run_gh_json(workspace: &Path, args: &[&str]) -> Result<JsonValue> {
    let output = Command::new("gh")
        .args(args)
        .current_dir(workspace)
        .output()
        .with_context(|| format!("running `gh {}`", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            bail!(
                "`gh {}` failed with exit code {:?}",
                args.join(" "),
                output.status.code()
            );
        }
        bail!("`gh {}` failed: {stderr}", args.join(" "));
    }

    let stdout = String::from_utf8(output.stdout).context("gh command returned non-UTF8 output")?;
    let parsed = serde_json::from_str::<JsonValue>(&stdout)
        .with_context(|| format!("parsing JSON output from `gh {}`", args.join(" ")))?;
    Ok(parsed)
}

pub(crate) fn parse_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn rule_matches_branch(pattern: &str, branch: &str) -> bool {
    if pattern == branch {
        return true;
    }
    let regex = format!("^{}$", regex::escape(pattern).replace("\\*", ".*"));
    Regex::new(&regex)
        .map(|compiled| compiled.is_match(branch))
        .unwrap_or(false)
}

pub(crate) fn yaml_string(root: &Value, path: &[&str]) -> Option<String> {
    let mut current = root;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(str::to_string)
}

pub(crate) fn yaml_bool(root: &Value, path: &[&str]) -> Option<bool> {
    let mut current = root;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_bool()
}
