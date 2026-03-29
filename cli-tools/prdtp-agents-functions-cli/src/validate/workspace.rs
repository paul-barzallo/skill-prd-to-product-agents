use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use walkdir::WalkDir;

use crate::common::workspace_paths;
use crate::validate::finalize_validation;

pub fn run(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "validating workspace structure");
    println!("{}", "=== Validate Workspace ===".cyan().bold());
    let mut errors = 0u32;
    let mut warnings = 0u32;

    // ── Check required files ─────────────────────────────────────
    println!("\n{}", "Checking required files...".bold());
    for rel in workspace_paths::REQUIRED_FILES
        .iter()
        .chain(workspace_paths::EXTENDED_REQUIRED_FILES.iter())
    {
        let full = workspace.join(rel);
        if full.exists() {
            println!("  {} {rel}", "✓".green());
        } else {
            tracing::error!(path = %rel, "required workspace file missing");
            eprintln!("  {} {rel} — missing", "✗".red());
            errors += 1;
        }
    }

    // ── Check agent files ────────────────────────────────────────
    println!("\n{}", "Checking agent files...".bold());
    let agents_dir = workspace.join(".github/agents");
    for name in workspace_paths::AGENT_NAMES {
        let agent_file = agents_dir.join(format!("{name}.agent.md"));
        if agent_file.exists() {
            println!("  {} {name}.agent.md", "✓".green());
        } else {
            tracing::error!(agent = %name, path = %format!(".github/agents/{name}.agent.md"), "agent file missing");
            eprintln!("  {} {name}.agent.md — missing", "✗".red());
            errors += 1;
        }

        // Identity source
        let identity = agents_dir.join(format!("identity/{name}.md"));
        if identity.exists() {
            println!("  {} identity/{name}.md", "✓".green());
        } else {
            tracing::error!(agent = %name, path = %format!(".github/agents/identity/{name}.md"), "agent identity source missing");
            eprintln!("  {} identity/{name}.md — missing", "✗".red());
            errors += 1;
        }

        // Context source
        let context = agents_dir.join(format!("context/{name}.md"));
        if context.exists() {
            println!("  {} context/{name}.md", "✓".green());
        } else {
            tracing::warn!(agent = %name, path = %format!(".github/agents/context/{name}.md"), "agent context source missing");
            eprintln!("  {} context/{name}.md — missing", "⚠".yellow());
            warnings += 1;
        }
    }

    // Check shared context
    let shared = agents_dir.join("context/shared-context.md");
    if shared.exists() {
        println!("  {} context/shared-context.md", "✓".green());
    } else {
        tracing::error!(
            path = ".github/agents/context/shared-context.md",
            "shared agent context missing"
        );
        eprintln!("  {} context/shared-context.md — missing", "✗".red());
        errors += 1;
    }

    // ── Validate YAML parsability ────────────────────────────────
    println!("\n{}", "Validating YAML files...".bold());
    for rel in workspace_paths::YAML_FILES {
        let full = workspace.join(rel);
        if !full.exists() {
            continue;
        }
        match std::fs::read_to_string(&full) {
            Ok(content) => {
                let parsed: Result<serde_yaml::Value, _> = serde_yaml::from_str(&content);
                match parsed {
                    Ok(_) => println!("  {} {rel}", "✓".green()),
                    Err(e) => {
                        tracing::error!(path = %rel, error = %e, "yaml parse error during workspace validation");
                        eprintln!("  {} {rel} — parse error: {e}", "✗".red());
                        errors += 1;
                    }
                }
            }
            Err(e) => {
                tracing::error!(path = %rel, error = %e, "yaml read error during workspace validation");
                eprintln!("  {} {rel} — read error: {e}", "✗".red());
                errors += 1;
            }
        }
    }

    // ── Structural validation against schemas ────────────────────
    println!(
        "\n{}",
        "Validating YAML structure against schemas...".bold()
    );
    let (se, sw) = validate_operational_yaml_structure(workspace);
    errors += se;
    warnings += sw;

    // ── Validate agent frontmatter has model: field ──────────────
    println!("\n{}", "Checking agent frontmatter...".bold());
    for name in workspace_paths::AGENT_NAMES {
        let agent_file = agents_dir.join(format!("{name}.agent.md"));
        if !agent_file.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&agent_file)?;
        if content.starts_with("---") {
            if let Some(end) = content[3..].find("---") {
                let fm = &content[3..end + 3];
                if !fm.contains("model:") {
                    tracing::warn!(agent = %name, "agent frontmatter missing model field");
                    eprintln!(
                        "  {} {name}.agent.md — missing model: in frontmatter",
                        "⚠".yellow()
                    );
                    warnings += 1;
                } else {
                    println!("  {} {name}.agent.md frontmatter OK", "✓".green());
                }
            }
        } else {
            tracing::warn!(agent = %name, "agent file missing yaml frontmatter");
            eprintln!(
                "  {} {name}.agent.md — no YAML frontmatter found",
                "⚠".yellow()
            );
            warnings += 1;
        }
    }

    // ── Check .state directory ───────────────────────────────────
    println!("\n{}", "Checking .state directory...".bold());
    let state_dir = workspace.join(".state");
    if state_dir.exists() {
        println!("  {} .state/ exists", "✓".green());
        let db = workspace.join(".state/project_memory.db");
        if db.exists() {
            println!("  {} .state/project_memory.db exists", "✓".green());
        } else {
            tracing::warn!(
                path = ".state/project_memory.db",
                "sqlite database missing during workspace validation"
            );
            eprintln!(
                "  {} .state/project_memory.db — missing (SQLite may be disabled)",
                "⚠".yellow()
            );
            warnings += 1;
        }
    } else {
        tracing::warn!(
            path = ".state",
            "workspace state directory missing during validation"
        );
        eprintln!("  {} .state/ — missing", "⚠".yellow());
        warnings += 1;
    }

    // ── Check prompt files ───────────────────────────────────────
    println!("\n{}", "Checking prompt files...".bold());
    let prompts_dir = workspace.join(".github/prompts");
    if prompts_dir.exists() {
        let count = WalkDir::new(&prompts_dir)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
            .count();
        println!("  {} Found {count} prompt files", "✓".green());
    } else {
        tracing::warn!(
            path = ".github/prompts",
            "prompt directory missing during workspace validation"
        );
        eprintln!("  {} .github/prompts/ — missing", "⚠".yellow());
        warnings += 1;
    }

    // ── Summary ──────────────────────────────────────────────────
    println!("\n{}", "────────────────────────────".dimmed());
    finalize_validation("workspace", errors, warnings, None, "All checks passed")
}

/// Validate operational YAML files against the structural rules from schemas/.
/// Returns (errors, warnings).
fn validate_operational_yaml_structure(workspace: &Path) -> (u32, u32) {
    let mut errors = 0u32;
    let mut warnings = 0u32;

    let valid_roles: &[&str] = &[
        "pm-orchestrator",
        "product-owner",
        "ux-designer",
        "software-architect",
        "tech-lead",
        "backend-developer",
        "frontend-developer",
        "qa-lead",
        "devops-release-engineer",
    ];

    // ── handoffs.yaml ────────────────────────────────────────────
    let handoffs_path = workspace.join("docs/project/handoffs.yaml");
    if handoffs_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&handoffs_path) {
            if let Ok(parsed) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                if let Some(list) = parsed.get("handoffs").and_then(|v| v.as_sequence()) {
                    for (i, entry) in list.iter().enumerate() {
                        let label = format!("handoffs[{i}]");
                        check_enum_field(
                            entry,
                            "status",
                            &["pending", "claimed", "done", "cancelled"],
                            &label,
                            &mut errors,
                        );
                        check_enum_field(
                            entry,
                            "type",
                            &["normal", "escalation", "rework", "approval"],
                            &label,
                            &mut errors,
                        );
                        check_enum_field(entry, "from", valid_roles, &label, &mut warnings);
                        check_enum_field(entry, "to", valid_roles, &label, &mut warnings);
                        check_required_field(entry, "id", &label, &mut errors);
                    }
                    println!(
                        "  {} handoffs.yaml — {count} entries validated",
                        "✓".green(),
                        count = list.len()
                    );
                }
            }
        }
    }

    // ── findings.yaml ────────────────────────────────────────────
    let findings_path = workspace.join("docs/project/findings.yaml");
    if findings_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&findings_path) {
            if let Ok(parsed) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                if let Some(list) = parsed.get("findings").and_then(|v| v.as_sequence()) {
                    for (i, entry) in list.iter().enumerate() {
                        let label = format!("findings[{i}]");
                        check_enum_field(
                            entry,
                            "status",
                            &["open", "triaged", "in_progress", "resolved", "wont_fix"],
                            &label,
                            &mut errors,
                        );
                        check_enum_field(
                            entry,
                            "severity",
                            &["low", "medium", "high", "critical"],
                            &label,
                            &mut errors,
                        );
                        check_enum_field(
                            entry,
                            "type",
                            &["bug", "risk", "ambiguity", "security", "ux", "architecture"],
                            &label,
                            &mut errors,
                        );
                        check_required_field(entry, "id", &label, &mut errors);
                        check_required_field(entry, "title", &label, &mut errors);
                    }
                    println!(
                        "  {} findings.yaml — {count} entries validated",
                        "✓".green(),
                        count = list.len()
                    );
                }
            }
        }
    }

    // ── releases.yaml ────────────────────────────────────────────
    let releases_path = workspace.join("docs/project/releases.yaml");
    if releases_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&releases_path) {
            if let Ok(parsed) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                if let Some(list) = parsed.get("releases").and_then(|v| v.as_sequence()) {
                    for (i, entry) in list.iter().enumerate() {
                        let label = format!("releases[{i}]");
                        check_enum_field(
                            entry,
                            "status",
                            &["planning", "ready", "approved", "deployed", "rolled_back"],
                            &label,
                            &mut errors,
                        );
                        check_required_field(entry, "id", &label, &mut errors);
                        check_required_field(entry, "name", &label, &mut errors);
                    }
                    println!(
                        "  {} releases.yaml — {count} entries validated",
                        "✓".green(),
                        count = list.len()
                    );
                }
            }
        }
    }

    (errors, warnings)
}

fn check_enum_field(
    entry: &serde_yaml::Value,
    field: &str,
    valid: &[&str],
    label: &str,
    counter: &mut u32,
) {
    if let Some(val) = entry.get(field).and_then(|v| v.as_str()) {
        if !valid.contains(&val) {
            tracing::error!(entry = %label, field = %field, value = %val, "invalid enum value in operational yaml");
            eprintln!(
                "  {} {label}.{field} = '{val}' — not a valid value (expected one of: {})",
                "✗".red(),
                valid.join(", ")
            );
            *counter += 1;
        }
    }
}

fn check_required_field(entry: &serde_yaml::Value, field: &str, label: &str, counter: &mut u32) {
    if entry.get(field).is_none() {
        tracing::error!(entry = %label, field = %field, "required field missing in operational yaml");
        eprintln!("  {} {label} — missing required field '{field}'", "✗".red());
        *counter += 1;
    }
}
