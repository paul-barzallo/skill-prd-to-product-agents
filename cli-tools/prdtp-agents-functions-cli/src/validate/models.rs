use anyhow::Result;
use colored::Colorize;
use std::path::Path;

const ALL_AGENTS: &[&str] = &[
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

pub fn run(workspace: &Path) -> Result<()> {
    println!("{}", "=== Validate Models ===".cyan().bold());

    let policy_path = workspace.join(".github/agent-model-policy.yaml");
    if !policy_path.exists() {
        eprintln!(
            "  {} .github/agent-model-policy.yaml not found — skipping model validation",
            "⚠".yellow()
        );
        return Ok(());
    }

    let policy_content = std::fs::read_to_string(&policy_path)?;
    let policy: serde_yaml::Value = serde_yaml::from_str(&policy_content)?;

    // Extract allowed model names from policy (key: official_ga_models)
    let allowed_models: Vec<String> = if let Some(models) = policy.get("official_ga_models") {
        models
            .as_sequence()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    } else {
        vec![]
    };

    let mut errors = 0u32;
    let mut warnings = 0u32;
    let agents_dir = workspace.join(".github/agents");

    for name in ALL_AGENTS {
        let path = agents_dir.join(format!("{name}.agent.md"));
        if !path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&path)?;
        if !content.starts_with("---") {
            continue;
        }

        let rest = &content[3..];
        let end = match rest.find("---") {
            Some(e) => e,
            None => continue,
        };
        let fm = &rest[..end];

        // Extract model: field using proper YAML parsing
        let fm_parsed: serde_yaml::Value = match serde_yaml::from_str(fm) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let models_in_fm: Vec<String> = match fm_parsed.get("model") {
            Some(serde_yaml::Value::String(s)) => vec![s.clone()],
            Some(serde_yaml::Value::Sequence(seq)) => seq
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
            _ => vec![],
        };

        if models_in_fm.is_empty() {
            eprintln!(
                "  {} {name}.agent.md — no model: in frontmatter",
                "⚠".yellow()
            );
            warnings += 1;
            continue;
        }

        for model in &models_in_fm {
            if !allowed_models.is_empty() && !allowed_models.contains(model) {
                eprintln!(
                    "  {} {name}.agent.md — model '{model}' not in allowed_models policy",
                    "✗".red()
                );
                errors += 1;
            }
        }

        if errors == 0 {
            println!("  {} {name}.agent.md model(s): {:?}", "✓".green(), models_in_fm);
        }
    }

    // ── Summary ──────────────────────────────────────────────────
    println!("\n{}", "────────────────────────────".dimmed());
    if errors > 0 {
        eprintln!(
            "{} {errors} error(s), {warnings} warning(s)",
            "FAILED:".red().bold()
        );
        std::process::exit(1);
    } else if warnings > 0 {
        println!(
            "{} 0 errors, {warnings} warning(s)",
            "PASSED (with warnings):".yellow().bold()
        );
    } else {
        println!("{} Model checks passed", "PASSED:".green().bold());
    }
    Ok(())
}
