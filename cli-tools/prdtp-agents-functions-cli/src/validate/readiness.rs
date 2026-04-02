use anyhow::{bail, Context, Result};
use colored::Colorize;
use serde_yaml::Value;
use std::path::Path;

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
    capability_contract::require_policy_enabled(workspace, "gh", "validate readiness")?;

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

pub(crate) fn validate_remote_governance(_workspace: &Path, governance: &Value) -> Result<()> {
    let project_enabled = yaml_bool(governance, &["github", "project", "enabled"]).unwrap_or(false);
    if project_enabled {
        bail!(
            "github.project.enabled=true is out of the current supported operational contract; keep github.project.enabled=false until GitHub Project support is implemented as a future extension"
        );
    }

    let profile = crate::github_api::operating_profile(governance)?;
    if profile != crate::github_api::OperatingProfile::Enterprise {
        bail!("production-ready readiness requires operating_profile=enterprise");
    }
    if crate::github_api::audit_mode(governance)? != crate::github_api::AuditMode::Remote {
        bail!("production-ready readiness requires audit.mode=remote");
    }
    let _ = crate::github_api::audit_remote_config(governance)?
        .context("production-ready readiness requires audit.remote.* to be configured")?;
    crate::github_api::require_enterprise_api_mode(governance)?;
    let _ = crate::github_api::github_identity_login(governance)
        .context("enterprise readiness requires a working non-interactive GitHub API identity")?;

    let owner = yaml_string(governance, &["github", "repository", "owner"])
        .context("github.repository.owner missing")?;
    let repo = yaml_string(governance, &["github", "repository", "name"])
        .context("github.repository.name missing")?;
    let repo_full_name = format!("{owner}/{repo}");

    let repo_view = crate::github_api::api_get_json(governance, &format!("repos/{repo_full_name}"))
        .context("unable to resolve configured repository via GitHub API")?;
    let remote_name = repo_view["full_name"].as_str().unwrap_or_default().to_string();
    if remote_name != repo_full_name {
        bail!(
            "configured repository mismatch: expected '{repo_full_name}', GitHub API resolved '{remote_name}'"
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
    let release_gate_reviewer_count = parse_csv(
        &yaml_string(governance, &["github", "release_gate", "reviewer_logins"])
            .unwrap_or_default(),
    )
    .into_iter()
    .collect::<std::collections::BTreeSet<_>>()
    .len();
    let release_gate_quorum = configured_approval_quorum(
        governance,
        &["github", "release_gate", "approval_quorum"],
        "github.release_gate.approval_quorum",
        release_gate_reviewer_count,
        Some(6),
    )?;

    let mut branch_rule_errors = Vec::new();
    for branch in &protected_branches {
        let rule = load_branch_protection_rule(governance, &repo_full_name, branch)?;
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
            } else if rule.required_approving_review_count < release_gate_quorum {
                branch_rule_errors.push(format!(
                    "{branch}: remote rule '{}' requires fewer than {} approving review(s)",
                    rule.pattern, release_gate_quorum
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
        crate::github_api::api_get_json(governance, &format!("users/{login}")).with_context(|| {
            format!("release gate login '{login}' is not readable via GitHub API")
        })?;
    }

    let immutable_required = yaml_bool(governance, &["github", "immutable_governance", "required"])
        .unwrap_or(false);
    if immutable_required {
        let immutable_logins = parse_csv(
            &yaml_string(governance, &["github", "immutable_governance", "reviewer_logins"])
                .unwrap_or_default(),
        )
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
        if immutable_logins.is_empty() {
            bail!("github.immutable_governance.reviewer_logins must contain at least one login");
        }
        let _ = configured_approval_quorum(
            governance,
            &["github", "immutable_governance", "approval_quorum"],
            "github.immutable_governance.approval_quorum",
            immutable_logins.len(),
            None,
        )?;
        for login in &immutable_logins {
            crate::github_api::api_get_json(governance, &format!("users/{login}")).with_context(|| {
                format!("immutable governance login '{login}' is not readable via GitHub API")
            })?;
        }
    }

    Ok(())
}

fn load_branch_protection_rule(
    governance: &Value,
    repo_full_name: &str,
    branch: &str,
) -> Result<RemoteBranchProtectionRule> {
    let response = crate::github_api::api_get_json(
        governance,
        &format!(
            "repos/{repo_full_name}/branches/{}/protection",
            urlencoding::encode(branch)
        ),
    )
    .with_context(|| format!("loading branch protection for '{branch}'"))?;

    Ok(RemoteBranchProtectionRule {
        pattern: branch.to_string(),
        requires_approving_reviews: response["required_pull_request_reviews"]
            .is_object(),
        required_approving_review_count: response["required_pull_request_reviews"]
            ["required_approving_review_count"]
            .as_u64()
            .unwrap_or(0),
        requires_code_owner_reviews: response["required_pull_request_reviews"]
            ["require_code_owner_reviews"]
            .as_bool()
            .unwrap_or(false),
        requires_conversation_resolution: response["required_conversation_resolution"]
            ["enabled"]
            .as_bool()
            .unwrap_or(false),
    })
}

pub(crate) fn parse_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

pub(crate) fn yaml_string(root: &Value, path: &[&str]) -> Option<String> {
    crate::github_api::yaml_string(root, path)
}

pub(crate) fn yaml_bool(root: &Value, path: &[&str]) -> Option<bool> {
    crate::github_api::yaml_bool(root, path)
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
