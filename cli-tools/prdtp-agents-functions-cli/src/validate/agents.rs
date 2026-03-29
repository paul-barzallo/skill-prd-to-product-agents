use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::common::workspace_paths;
use crate::validate::{extract_frontmatter, finalize_validation};

pub fn run(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "validating assembled agent files");
    println!("{}", "=== Validate Agents ===".cyan().bold());

    let agents_dir = workspace.join(".github/agents");
    let mut errors = 0u32;
    let mut warnings = 0u32;

    // ── Check each agent file ────────────────────────────────────
    for name in workspace_paths::AGENT_NAMES {
        let path = agents_dir.join(format!("{name}.agent.md"));
        if !path.exists() {
            tracing::error!(agent = %name, path = %format!(".github/agents/{name}.agent.md"), "assembled agent file missing");
            eprintln!("  {} {name}.agent.md — missing", "✗".red());
            errors += 1;
            continue;
        }
        let content = std::fs::read_to_string(&path)?;

        // Check CONTEXT ZONE divider
        if !content.contains("CONTEXT ZONE") {
            tracing::error!(agent = %name, "assembled agent file missing context zone divider");
            eprintln!(
                "  {} {name}.agent.md — missing CONTEXT ZONE divider",
                "✗".red()
            );
            errors += 1;
        }

        // L0 (pm-orchestrator) must NOT list L2 agents in its agents: frontmatter
        if *name == "pm-orchestrator" {
            if let Some(fm) = extract_frontmatter(&content) {
                for l2 in workspace_paths::L2_AGENTS {
                    if fm.contains(l2) {
                        tracing::error!(agent = "pm-orchestrator", child_agent = %l2, "skip-level coordinator violation detected");
                        eprintln!(
                            "  {} pm-orchestrator frontmatter lists L2 agent '{l2}' — skip-level violation",
                            "✗".red()
                        );
                        errors += 1;
                    }
                }
            }
        }

        // L2 agents must report back to tech-lead only
        if workspace_paths::L2_AGENTS.contains(name) {
            let mentions_report_back = content.contains("tech-lead")
                && (content.contains("report-back") || content.contains("Report Back"));
            if !mentions_report_back {
                tracing::warn!(agent = %name, "l2 agent missing report-back to tech-lead guidance");
                eprintln!(
                    "  {} {name}.agent.md — L2 agent should mention report-back to tech-lead",
                    "⚠".yellow()
                );
                warnings += 1;
            }
        }

        // Only coordinators should have the `agent` tool and `agents:` property
        let has_agents_section = content.contains("agents:");
        let is_coordinator = workspace_paths::COORDINATOR_AGENTS.contains(name);
        if has_agents_section && !is_coordinator {
            tracing::warn!(agent = %name, "non-coordinator agent has agents property");
            eprintln!(
                "  {} {name}.agent.md — non-coordinator has agents: property",
                "⚠".yellow()
            );
            warnings += 1;
        }

        println!("  {} {name}.agent.md", "✓".green());
    }

    // ── Check identity/context source file presence ──────────────
    println!("\n{}", "Checking assembly sources...".bold());
    let shared_ctx = agents_dir.join("context/shared-context.md");
    if !shared_ctx.exists() {
        tracing::error!(
            path = ".github/agents/context/shared-context.md",
            "shared context source missing"
        );
        eprintln!("  {} context/shared-context.md — missing", "✗".red());
        errors += 1;
    } else {
        println!("  {} context/shared-context.md", "✓".green());
    }

    let divider = agents_dir.join("CONTEXT_ZONE_DIVIDER.txt");
    if !divider.exists() {
        tracing::error!(
            path = ".github/agents/CONTEXT_ZONE_DIVIDER.txt",
            "context zone divider source missing"
        );
        eprintln!("  {} CONTEXT_ZONE_DIVIDER.txt — missing", "✗".red());
        errors += 1;
    } else {
        println!("  {} CONTEXT_ZONE_DIVIDER.txt", "✓".green());
    }

    // ── Summary ──────────────────────────────────────────────────
    println!("\n{}", "────────────────────────────".dimmed());
    finalize_validation("agents", errors, warnings, None, "All agent checks passed")
}
