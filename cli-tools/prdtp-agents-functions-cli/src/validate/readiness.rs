use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::common::capability_contract;
use crate::encoding::{self, EncodingArgs};
use crate::validate::finalize_validation;

const READY_STATUSES: &[&str] = &["configured", "production-ready"];

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
        eprintln!("  {} .github/workspace-capabilities.yaml missing", "✗".red());
        errors += 1;
    } else {
        println!("  {} workspace-capabilities.yaml present", "✓".green());
        for capability in ["git", "gh", "sqlite", "reporting"] {
            match capability_contract::policy_enabled(workspace, capability) {
                Ok(Some(enabled)) => {
                    let icon = if enabled { "✓".green() } else { "⚠".yellow() };
                    let status = if enabled { "enabled" } else { "disabled" };
                    println!("  {} capabilities.{capability}.policy.enabled = {status}", icon);
                }
                Ok(None) => {
                    eprintln!(
                        "  {} capabilities.{capability}.policy.enabled missing",
                        "✗".red()
                    );
                    errors += 1;
                }
                Err(error) => {
                    eprintln!(
                        "  {} capabilities.{capability}.policy.enabled unreadable: {error}",
                        "✗".red()
                    );
                    errors += 1;
                }
            }
        }
    }

    println!("\n{}", "Checking readiness state...".bold());
    let gov_path = workspace.join(".github/github-governance.yaml");
    if gov_path.exists() {
        let content = std::fs::read_to_string(&gov_path)?;
        let parsed: serde_yaml::Value = serde_yaml::from_str(&content)?;
        let readiness = parsed
            .get("readiness")
            .and_then(|v| v.get("status"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if READY_STATUSES.contains(&readiness) {
            println!("  {} readiness.status = {}", "✓".green(), readiness);
        } else {
            eprintln!(
                "  {} readiness.status = '{}' is not ready; expected one of: {}",
                "✗".red(),
                readiness,
                READY_STATUSES.join(", ")
            );
            errors += 1;
        }
    } else {
        eprintln!("  {} .github/github-governance.yaml missing", "✗".red());
        errors += 1;
    }

    finalize_validation(
        "readiness",
        errors,
        warnings,
        None,
        "Workspace is operationally ready",
    )
}
