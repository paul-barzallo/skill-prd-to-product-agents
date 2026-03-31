use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;
use serde_json::{json, Value as JsonValue};
use serde_yaml::Value as YamlValue;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Args)]
pub struct ExportArgs {
    /// Output JSONL path
    #[arg(long)]
    pub output: Option<PathBuf>,
}

pub fn run(workspace: &Path, args: ExportArgs) -> Result<()> {
    println!("{}", "=== Export Audit JSONL ===".cyan().bold());
    let output_path = args
        .output
        .unwrap_or_else(|| workspace.join(".state/audit/export.jsonl"));
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut lines = Vec::new();
    append_yaml_snapshot(
        workspace,
        ".github/workspace-capabilities.yaml",
        "capability_contract",
        &mut lines,
    )?;
    append_yaml_snapshot(
        workspace,
        ".github/github-governance.yaml",
        "governance_contract",
        &mut lines,
    )?;
    append_jsonl_file(
        workspace,
        ".state/audit/sensitive-actions.jsonl",
        "sensitive_action",
        &mut lines,
    )?;
    append_json_directory(workspace, ".state/audit-spool", "audit_spool", &mut lines)?;
    append_json_directory(workspace, ".state/work-units", "work_unit", &mut lines)?;
    append_operational_yaml(workspace, "docs/project/handoffs.yaml", "handoffs", &mut lines)?;
    append_operational_yaml(workspace, "docs/project/findings.yaml", "findings", &mut lines)?;
    append_operational_yaml(workspace, "docs/project/releases.yaml", "releases", &mut lines)?;

    fs::write(&output_path, lines.join("\n"))?;
    println!(
        "{} wrote {} event(s) to {}",
        "OK:".green().bold(),
        lines.len(),
        output_path.display()
    );
    Ok(())
}

fn append_yaml_snapshot(
    workspace: &Path,
    relative_path: &str,
    event_type: &str,
    lines: &mut Vec<String>,
) -> Result<()> {
    let path = workspace.join(relative_path);
    if !path.is_file() {
        return Ok(());
    }
    let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let parsed: YamlValue =
        serde_yaml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
    lines.push(
        json!({
            "event_type": event_type,
            "entity_path": relative_path,
            "exported_at": chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "payload": parsed
        })
        .to_string(),
    );
    Ok(())
}

fn append_json_directory(
    workspace: &Path,
    relative_dir: &str,
    event_type: &str,
    lines: &mut Vec<String>,
) -> Result<()> {
    let dir = workspace.join(relative_dir);
    if !dir.is_dir() {
        return Ok(());
    }
    let mut entries = fs::read_dir(&dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().map(|ext| ext == "json").unwrap_or(false))
        .collect::<Vec<_>>();
    entries.sort();

    for path in entries {
        let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let payload: JsonValue =
            serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        lines.push(
            json!({
                "event_type": event_type,
                "entity_path": path.strip_prefix(workspace).unwrap_or(&path).display().to_string(),
                "payload": payload
            })
            .to_string(),
        );
    }
    Ok(())
}

fn append_jsonl_file(
    workspace: &Path,
    relative_path: &str,
    event_type: &str,
    lines: &mut Vec<String>,
) -> Result<()> {
    let path = workspace.join(relative_path);
    if !path.is_file() {
        return Ok(());
    }
    let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        let payload: JsonValue =
            serde_json::from_str(line).with_context(|| format!("parsing {}", path.display()))?;
        lines.push(
            json!({
                "event_type": event_type,
                "entity_path": relative_path,
                "payload": payload
            })
            .to_string(),
        );
    }
    Ok(())
}

fn append_operational_yaml(
    workspace: &Path,
    relative_path: &str,
    top_level_key: &str,
    lines: &mut Vec<String>,
) -> Result<()> {
    let path = workspace.join(relative_path);
    if !path.is_file() {
        return Ok(());
    }
    let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let parsed: YamlValue =
        serde_yaml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
    if let Some(entries) = parsed.get(top_level_key).and_then(|value| value.as_sequence()) {
        for entry in entries {
            lines.push(
                json!({
                    "event_type": top_level_key.trim_end_matches('s'),
                    "entity_path": relative_path,
                    "payload": entry
                })
                .to_string(),
            );
        }
    }
    Ok(())
}
