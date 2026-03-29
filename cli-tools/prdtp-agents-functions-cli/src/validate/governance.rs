use anyhow::Result;
use colored::Colorize;
use serde_yaml::Value;
use std::fs;
use std::path::Path;

use crate::validate::finalize_validation;

const VALID_READINESS: &[&str] = &["template", "bootstrapped", "configured", "production-ready"];
const CONFIGURED_READINESS: &[&str] = &["configured", "production-ready"];

pub fn run(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "validating configured governance contract");
    println!("{}", "=== Validate Governance ===".cyan().bold());

    let gov_path = workspace.join(".github/github-governance.yaml");
    let mut errors = 0u32;
    let warnings = 0u32;

    if !gov_path.exists() {
        eprintln!("  {} .github/github-governance.yaml not found", "✗".red());
        return finalize_validation("governance", 1, warnings, None, "Governance checks passed");
    }

    let content = fs::read_to_string(&gov_path)?;
    let parsed: Value = match serde_yaml::from_str(&content) {
        Ok(value) => {
            println!("  {} YAML parse OK", "✓".green());
            value
        }
        Err(error) => {
            eprintln!("  {} YAML parse error: {error}", "✗".red());
            return finalize_validation(
                "governance",
                1,
                warnings,
                None,
                "Governance checks passed",
            );
        }
    };

    println!("\n{}", "Checking readiness state...".bold());
    let readiness = yaml_string(&parsed, &["readiness", "status"]);
    match readiness.as_deref() {
        Some(status) if VALID_READINESS.contains(&status) => {
            println!("  {} readiness.status = {status}", "✓".green());
            if !CONFIGURED_READINESS.contains(&status) {
                eprintln!(
                    "  {} readiness.status is '{status}' but governance validation requires 'configured' or 'production-ready'",
                    "✗".red()
                );
                errors += 1;
            }
        }
        Some(status) => {
            eprintln!(
                "  {} readiness.status '{}' is invalid (expected: {})",
                "✗".red(),
                status,
                VALID_READINESS.join(", ")
            );
            errors += 1;
        }
        None => {
            eprintln!("  {} readiness.status missing", "✗".red());
            errors += 1;
        }
    }

    println!("\n{}", "Checking repository identifiers...".bold());
    errors += check_scalar(
        &parsed,
        &["github", "repository", "owner"],
        "github.repository.owner",
    );
    errors += check_scalar(
        &parsed,
        &["github", "repository", "name"],
        "github.repository.name",
    );

    println!("\n{}", "Checking reviewers...".bold());
    errors += check_scalar(
        &parsed,
        &["github", "reviewers", "product"],
        "github.reviewers.product",
    );
    errors += check_scalar(
        &parsed,
        &["github", "reviewers", "architecture"],
        "github.reviewers.architecture",
    );
    errors += check_scalar(
        &parsed,
        &["github", "reviewers", "tech_lead"],
        "github.reviewers.tech_lead",
    );
    errors += check_scalar(
        &parsed,
        &["github", "reviewers", "qa"],
        "github.reviewers.qa",
    );
    errors += check_scalar(
        &parsed,
        &["github", "reviewers", "devops"],
        "github.reviewers.devops",
    );
    errors += check_scalar(
        &parsed,
        &["github", "reviewers", "infra"],
        "github.reviewers.infra",
    );
    errors += check_scalar(
        &parsed,
        &["github", "release_gate", "reviewer_handles"],
        "github.release_gate.reviewer_handles",
    );
    errors += check_scalar(
        &parsed,
        &["github", "release_gate", "reviewer_logins"],
        "github.release_gate.reviewer_logins",
    );
    errors += check_bool(
        &parsed,
        &["github", "immutable_governance", "required"],
        "github.immutable_governance.required",
    );
    errors += check_scalar(
        &parsed,
        &["github", "immutable_governance", "reviewer_handles"],
        "github.immutable_governance.reviewer_handles",
    );
    errors += check_scalar(
        &parsed,
        &["github", "immutable_governance", "reviewer_logins"],
        "github.immutable_governance.reviewer_logins",
    );
    errors += check_scalar(
        &parsed,
        &["github", "immutable_governance", "required_labels"],
        "github.immutable_governance.required_labels",
    );

    println!("\n{}", "Scanning placeholder markers...".bold());
    if contains_placeholder_marker(&content) {
        eprintln!(
            "  {} github-governance.yaml still contains placeholder markers",
            "✗".red()
        );
        errors += 1;
    } else {
        println!(
            "  {} github-governance.yaml is placeholder-free",
            "✓".green()
        );
    }

    println!("\n{}", "Checking CODEOWNERS...".bold());
    let codeowners_path = workspace.join(".github/CODEOWNERS");
    if !codeowners_path.exists() {
        eprintln!("  {} .github/CODEOWNERS missing", "✗".red());
        errors += 1;
    } else {
        let codeowners = fs::read_to_string(&codeowners_path)?;
        if contains_placeholder_marker(&codeowners) {
            eprintln!(
                "  {} CODEOWNERS still contains placeholders or template handles",
                "✗".red()
            );
            errors += 1;
        } else {
            println!("  {} CODEOWNERS present and placeholder-free", "✓".green());
        }
    }

    finalize_validation(
        "governance",
        errors,
        warnings,
        None,
        "Governance checks passed",
    )
}

fn check_scalar(parsed: &Value, path: &[&str], label: &str) -> u32 {
    match yaml_string(parsed, path) {
        Some(value) if !contains_placeholder_marker(&value) => {
            println!("  {} {} = {}", "✓".green(), label, value);
            0
        }
        Some(_) => {
            eprintln!("  {} {} contains a placeholder value", "✗".red(), label);
            1
        }
        None => {
            eprintln!("  {} {} missing", "✗".red(), label);
            1
        }
    }
}

fn check_bool(parsed: &Value, path: &[&str], label: &str) -> u32 {
    match yaml_bool(parsed, path) {
        Some(value) => {
            println!("  {} {} = {}", "✓".green(), label, value);
            0
        }
        None => {
            eprintln!("  {} {} missing or not boolean", "✗".red(), label);
            1
        }
    }
}

fn yaml_string(root: &Value, path: &[&str]) -> Option<String> {
    let mut current = root;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(str::to_string)
}

fn yaml_bool(root: &Value, path: &[&str]) -> Option<bool> {
    let mut current = root;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_bool()
}

fn contains_placeholder_marker(content: &str) -> bool {
    content.contains("REPLACE_ME")
        || content.contains("@team-")
        || content.contains("TODO")
        || content.contains("PLACEHOLDER")
        || content.contains("your-org")
        || content.contains("your-repo")
        || content.contains("github-governance-provisioned")
}
