use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use walkdir::WalkDir;

use crate::validate::{finalize_validation, validation_failure};

/// Required sections in every prompt file.
const REQUIRED_SECTIONS: &[&str] = &["## Context scope"];

/// Sections where at least ONE write-style section should exist.
const WRITE_SECTIONS: &[&str] = &["## Write", "## Exit"];

pub fn run(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "validating prompt files");
    println!("{}", "=== Validate Prompts ===".cyan().bold());

    let prompts_dir = workspace.join(".github/prompts");
    if !prompts_dir.exists() {
        tracing::error!(path = ".github/prompts", "prompt directory not found");
        eprintln!("{} .github/prompts/ not found", "ERROR:".red().bold());
        return Err(validation_failure("prompt directory not found: .github/prompts"));
    }

    let mut errors = 0u32;
    let mut warnings = 0u32;
    let mut scanned = 0u32;
    let mut fully_valid = 0u32;

    for entry in WalkDir::new(&prompts_dir)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
    {
        let path = entry.path();
        let rel = path.strip_prefix(workspace).unwrap_or(path);
        let content = std::fs::read_to_string(path)?;
        scanned += 1;
        let errors_before = errors;
        let warnings_before = warnings;

        // Check for YAML frontmatter
        if !content.starts_with("---") {
            tracing::error!(path = %rel.display(), "prompt file missing yaml frontmatter");
            eprintln!("  {} {} — no YAML frontmatter", "✗".red(), rel.display());
            errors += 1;
            continue;
        }

        // Check required sections
        for section in REQUIRED_SECTIONS {
            if !content.contains(section) {
                tracing::error!(path = %rel.display(), section = %section, "prompt file missing required section");
                eprintln!(
                    "  {} {} — missing section: {section}",
                    "✗".red(),
                    rel.display()
                );
                errors += 1;
            }
        }

        // Check at least one write/exit section exists
        let has_write = WRITE_SECTIONS.iter().any(|s| content.contains(s));
        if !has_write {
            tracing::warn!(path = %rel.display(), "prompt file missing write/exit section");
            eprintln!(
                "  {} {} — missing ## Write or ## Exit section",
                "⚠".yellow(),
                rel.display()
            );
            warnings += 1;
        }

        // Check that if the prompt references YAML files, it mentions state-ops
        let refs_yaml = content.contains("handoffs.yaml")
            || content.contains("findings.yaml")
            || content.contains("releases.yaml");
        let refs_state_ops = content.contains("state-ops")
            || content.contains("prdtp-agents-functions-cli state");
        if refs_yaml && !refs_state_ops {
            tracing::warn!(path = %rel.display(), "prompt references yaml state files without state-ops guidance");
            eprintln!(
                "  {} {} — references YAML state files but doesn't mention state-ops or prdtp-agents-functions-cli",
                "⚠".yellow(),
                rel.display()
            );
            warnings += 1;
        }

        if errors == errors_before && warnings == warnings_before {
            println!("  {} {}", "✓".green(), rel.display());
            fully_valid += 1;
        }
    }

    println!("\n{}", "────────────────────────────".dimmed());
    println!("Scanned {scanned} prompt file(s)");
    println!("Fully valid {fully_valid} prompt file(s)");
    finalize_validation("prompts", errors, warnings, Some(scanned), "All prompts valid")
}
