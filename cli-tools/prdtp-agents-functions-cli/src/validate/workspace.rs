use anyhow::Result;
use colored::Colorize;
use std::collections::BTreeSet;
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
    let (se, sw) = validate_structured_project_yaml(workspace);
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

/// Validate structured project YAML files against the shared schema and contract rules.
/// Returns (errors, warnings).
pub(crate) fn validate_structured_project_yaml(workspace: &Path) -> (u32, u32) {
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

    let acceptance_anchors = acceptance_criteria_anchors(workspace, &mut warnings);
    let mut backlog_story_ids = BTreeSet::new();
    let mut backlog_epic_ids = BTreeSet::new();
    let mut backlog_stories_defined = false;

    // ── backlog.yaml ─────────────────────────────────────────────
    let backlog_path = workspace.join("docs/project/backlog.yaml");
    if backlog_path.exists() {
        match std::fs::read_to_string(&backlog_path)
            .ok()
            .and_then(|content| serde_yaml::from_str::<serde_yaml::Value>(&content).ok())
        {
            Some(parsed) => {
                let epics = require_sequence(
                    &parsed,
                    "epics",
                    "backlog.yaml",
                    &mut errors,
                );
                let stories = require_sequence(
                    &parsed,
                    "stories",
                    "backlog.yaml",
                    &mut errors,
                );
                let epics_defined = epics.is_some();
                backlog_stories_defined = stories.is_some();

                if let Some(epics) = epics {
                    for (i, entry) in epics.iter().enumerate() {
                        let label = format!("backlog.epics[{i}]");
                        check_required_field(entry, "id", &label, &mut errors);
                        check_required_field(entry, "title", &label, &mut errors);
                        check_required_field(entry, "priority", &label, &mut errors);
                        check_required_field(entry, "status", &label, &mut errors);
                        check_role_field(entry, "owner_role", &label, valid_roles, &mut errors);
                        if let Some(id) = entry.get("id").and_then(|value| value.as_str()) {
                            backlog_epic_ids.insert(id.to_string());
                        }
                    }
                }

                if let Some(stories) = stories {
                    for (i, entry) in stories.iter().enumerate() {
                        let label = format!("backlog.stories[{i}]");
                        check_required_field(entry, "id", &label, &mut errors);
                        check_required_field(entry, "title", &label, &mut errors);
                        check_required_field(entry, "status", &label, &mut errors);
                        check_required_field(entry, "priority", &label, &mut errors);
                        check_required_field(entry, "epic_id", &label, &mut errors);
                        check_required_field(entry, "acceptance_ref", &label, &mut errors);
                        check_role_field(entry, "owner_role", &label, valid_roles, &mut errors);
                        check_acceptance_ref(
                            entry,
                            "acceptance_ref",
                            &label,
                            &acceptance_anchors,
                            &mut errors,
                        );

                        if let Some(id) = entry.get("id").and_then(|value| value.as_str()) {
                            backlog_story_ids.insert(id.to_string());
                        }

                        if let Some(epic_id) = entry.get("epic_id").and_then(|value| value.as_str())
                        {
                            if epics_defined && !backlog_epic_ids.contains(epic_id) {
                                tracing::error!(entry = %label, epic_id, "backlog story references missing epic");
                                eprintln!(
                                    "  {} {label}.epic_id = '{epic_id}' — missing matching epic in backlog.yaml",
                                    "✗".red()
                                );
                                errors += 1;
                            }
                        }
                    }
                    println!(
                        "  {} backlog.yaml — {epic_count} epic(s), {story_count} story(ies) validated",
                        "✓".green(),
                        epic_count = epics.map_or(0, |entries| entries.len()),
                        story_count = stories.len()
                    );
                }
            }
            None => {
                tracing::error!(path = %backlog_path.display(), "failed to parse backlog yaml during structural validation");
                eprintln!("  {} backlog.yaml — parse error", "✗".red());
                errors += 1;
            }
        }
    }

    // ── refined-stories.yaml ────────────────────────────────────
    let refined_path = workspace.join("docs/project/refined-stories.yaml");
    if refined_path.exists() {
        match std::fs::read_to_string(&refined_path)
            .ok()
            .and_then(|content| serde_yaml::from_str::<serde_yaml::Value>(&content).ok())
        {
            Some(parsed) => {
                if let Some(stories) = require_sequence(
                    &parsed,
                    "stories",
                    "refined-stories.yaml",
                    &mut errors,
                ) {
                    for (i, entry) in stories.iter().enumerate() {
                        let label = format!("refined-stories.stories[{i}]");
                        check_required_field(entry, "id", &label, &mut errors);
                        check_required_field(entry, "title", &label, &mut errors);
                        check_required_field(entry, "status", &label, &mut errors);
                        check_required_field(entry, "priority", &label, &mut errors);
                        check_required_field(entry, "owner_role", &label, &mut errors);
                        check_required_field(entry, "acceptance_ref", &label, &mut errors);
                        check_role_field(entry, "owner_role", &label, valid_roles, &mut errors);
                        check_acceptance_ref(
                            entry,
                            "acceptance_ref",
                            &label,
                            &acceptance_anchors,
                            &mut errors,
                        );

                        if let Some(status) = entry.get("status").and_then(|value| value.as_str()) {
                            if !status.eq_ignore_ascii_case("draft")
                                && !has_nonempty_sequence(entry, "implementation_map")
                            {
                                tracing::error!(entry = %label, status, "refined story is missing implementation_map outside draft state");
                                eprintln!(
                                    "  {} {label}.implementation_map — required when status is not 'draft'",
                                    "✗".red()
                                );
                                errors += 1;
                            }
                        }

                        if let Some(story_id) = entry.get("id").and_then(|value| value.as_str()) {
                            if backlog_stories_defined && !backlog_story_ids.contains(story_id) {
                                tracing::error!(entry = %label, story_id, "refined story references missing backlog story");
                                eprintln!(
                                    "  {} {label}.id = '{story_id}' — missing matching story in backlog.yaml",
                                    "✗".red()
                                );
                                errors += 1;
                            }
                        }
                    }
                    println!(
                        "  {} refined-stories.yaml — {count} story(ies) validated",
                        "✓".green(),
                        count = stories.len()
                    );
                }
            }
            None => {
                tracing::error!(path = %refined_path.display(), "failed to parse refined stories yaml during structural validation");
                eprintln!("  {} refined-stories.yaml — parse error", "✗".red());
                errors += 1;
            }
        }
    }

    // ── quality-gates.yaml ──────────────────────────────────────
    let gates_path = workspace.join("docs/project/quality-gates.yaml");
    if gates_path.exists() {
        match std::fs::read_to_string(&gates_path)
            .ok()
            .and_then(|content| serde_yaml::from_str::<serde_yaml::Value>(&content).ok())
        {
            Some(parsed) => {
                if let Some(gates) = require_sequence(
                    &parsed,
                    "gates",
                    "quality-gates.yaml",
                    &mut errors,
                ) {
                    for (i, entry) in gates.iter().enumerate() {
                        let label = format!("quality-gates.gates[{i}]");
                        check_required_field(entry, "id", &label, &mut errors);
                        check_required_field(entry, "name", &label, &mut errors);
                        check_required_field(entry, "status", &label, &mut errors);
                        check_required_field(entry, "owner_roles", &label, &mut errors);
                        check_role_list_field(entry, "owner_roles", &label, valid_roles, &mut errors);
                    }
                    println!(
                        "  {} quality-gates.yaml — {count} gate(s) validated",
                        "✓".green(),
                        count = gates.len()
                    );
                }
            }
            None => {
                tracing::error!(path = %gates_path.display(), "failed to parse quality gates yaml during structural validation");
                eprintln!("  {} quality-gates.yaml — parse error", "✗".red());
                errors += 1;
            }
        }
    }

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

fn acceptance_criteria_anchors(workspace: &Path, warnings: &mut u32) -> BTreeSet<String> {
    let path = workspace.join("docs/project/acceptance-criteria.md");
    let Ok(content) = std::fs::read_to_string(&path) else {
        tracing::warn!(path = %path.display(), "acceptance criteria doc missing during structured yaml validation");
        *warnings += 1;
        return BTreeSet::new();
    };

    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix('#')
                .map(|heading| markdown_anchor_slug(heading.trim_start_matches('#').trim()))
        })
        .filter(|slug| !slug.is_empty())
        .collect()
}

fn markdown_anchor_slug(heading: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for ch in heading.chars().flat_map(|ch| ch.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_dash = false;
        } else if (ch.is_ascii_whitespace() || ch == '-') && !last_was_dash && !slug.is_empty() {
            slug.push('-');
            last_was_dash = true;
        }
    }

    slug.trim_matches('-').to_string()
}

fn require_sequence<'a>(
    parsed: &'a serde_yaml::Value,
    key: &str,
    file_label: &str,
    errors: &mut u32,
) -> Option<&'a Vec<serde_yaml::Value>> {
    match parsed.get(key).and_then(|value| value.as_sequence()) {
        Some(sequence) => Some(sequence),
        None => {
            tracing::error!(file = %file_label, key, "expected top-level YAML sequence missing");
            eprintln!(
                "  {} {file_label} — top-level key '{key}' must be a YAML sequence",
                "✗".red()
            );
            *errors += 1;
            None
        }
    }
}

fn check_role_field(
    entry: &serde_yaml::Value,
    field: &str,
    label: &str,
    valid_roles: &[&str],
    counter: &mut u32,
) {
    check_enum_field(entry, field, valid_roles, label, counter);
    if entry.get(field).is_none() {
        return;
    }
    if entry.get(field).and_then(|value| value.as_str()).is_none() {
        tracing::error!(entry = %label, field = %field, "role field must be a string");
        eprintln!(
            "  {} {label}.{field} — expected a role string",
            "✗".red()
        );
        *counter += 1;
    }
}

fn check_role_list_field(
    entry: &serde_yaml::Value,
    field: &str,
    label: &str,
    valid_roles: &[&str],
    counter: &mut u32,
) {
    let Some(values) = entry.get(field) else {
        return;
    };

    let Some(values) = values.as_sequence() else {
        tracing::error!(entry = %label, field = %field, "role list field must be a sequence");
        eprintln!(
            "  {} {label}.{field} — expected a YAML sequence of roles",
            "✗".red()
        );
        *counter += 1;
        return;
    };

    if values.is_empty() {
        tracing::error!(entry = %label, field = %field, "role list field must not be empty");
        eprintln!("  {} {label}.{field} — must not be empty", "✗".red());
        *counter += 1;
        return;
    }

    for value in values {
        match value.as_str() {
            Some(role) if valid_roles.contains(&role) => {}
            Some(role) => {
                tracing::error!(entry = %label, field = %field, role, "invalid role in role list field");
                eprintln!(
                    "  {} {label}.{field} contains invalid role '{role}'",
                    "✗".red()
                );
                *counter += 1;
            }
            None => {
                tracing::error!(entry = %label, field = %field, "role list field contains non-string value");
                eprintln!(
                    "  {} {label}.{field} contains a non-string role value",
                    "✗".red()
                );
                *counter += 1;
            }
        }
    }
}

fn check_acceptance_ref(
    entry: &serde_yaml::Value,
    field: &str,
    label: &str,
    anchors: &BTreeSet<String>,
    counter: &mut u32,
) {
    let Some(reference) = entry.get(field).and_then(|value| value.as_str()) else {
        return;
    };

    let Some((path, anchor)) = reference.split_once('#') else {
        tracing::error!(entry = %label, field = %field, reference, "acceptance ref is missing markdown anchor");
        eprintln!(
            "  {} {label}.{field} = '{reference}' — expected docs/project/acceptance-criteria.md#<anchor>",
            "✗".red()
        );
        *counter += 1;
        return;
    };

    if path != "docs/project/acceptance-criteria.md" {
        tracing::error!(entry = %label, field = %field, reference, "acceptance ref points outside acceptance criteria doc");
        eprintln!(
            "  {} {label}.{field} = '{reference}' — expected docs/project/acceptance-criteria.md#<anchor>",
            "✗".red()
        );
        *counter += 1;
        return;
    }

    let normalized_anchor = markdown_anchor_slug(anchor);
    if normalized_anchor.is_empty() || !anchors.contains(&normalized_anchor) {
        tracing::error!(entry = %label, field = %field, reference, "acceptance ref points to missing heading");
        eprintln!(
            "  {} {label}.{field} = '{reference}' — missing matching heading in acceptance-criteria.md",
            "✗".red()
        );
        *counter += 1;
    }
}

fn has_nonempty_sequence(entry: &serde_yaml::Value, field: &str) -> bool {
    entry.get(field)
        .and_then(|value| value.as_sequence())
        .map(|value| !value.is_empty())
        .unwrap_or(false)
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
