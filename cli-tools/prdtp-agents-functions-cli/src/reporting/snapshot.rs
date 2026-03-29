use anyhow::Result;
use colored::Colorize;
use serde::Serialize;
use std::fs;
use std::path::Path;

use crate::common::yaml_ops;

#[derive(Serialize)]
struct ReportSnapshot {
    generated_at: String,
    workspace: String,
    product: ProductSummary,
    handoffs: Vec<serde_json::Value>,
    findings: Vec<serde_json::Value>,
    releases: Vec<serde_json::Value>,
    stories: Vec<serde_json::Value>,
    metrics: Metrics,
}

#[derive(Serialize)]
struct ProductSummary {
    vision: String,
    scope_status: String,
}

#[derive(Serialize)]
struct Metrics {
    total_stories: usize,
    total_handoffs: usize,
    total_findings: usize,
    open_findings: usize,
    total_releases: usize,
    pending_handoffs: usize,
}

pub fn run(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "building report snapshot");
    println!("{}", "=== Build Report Snapshot ===".cyan().bold());
    crate::common::capability_contract::require_policy_enabled(
        workspace,
        "reporting",
        "report snapshot",
    )?;

    // Read canonical files
    let handoffs = read_yaml_list(workspace, "docs/project/handoffs.yaml", "handoffs")?;
    let findings = read_yaml_list(workspace, "docs/project/findings.yaml", "findings")?;
    let releases = read_yaml_list(workspace, "docs/project/releases.yaml", "releases")?;
    let stories = read_yaml_list(workspace, "docs/project/refined-stories.yaml", "stories")?;

    // Read vision summary
    let vision_path = workspace.join("docs/project/vision.md");
    let vision_text = if vision_path.exists() {
        let content = fs::read_to_string(&vision_path)?;
        // Extract first non-empty, non-heading line
        content
            .lines()
            .find(|l| !l.is_empty() && !l.starts_with('#'))
            .unwrap_or("No vision defined")
            .to_string()
    } else {
        "No vision file".to_string()
    };

    let open_findings = findings
        .iter()
        .filter(|finding| matches_status(finding, &["open", "triaged", "in_progress"]))
        .count();

    let pending_handoffs = handoffs
        .iter()
        .filter(|handoff| matches_status(handoff, &["pending"]))
        .count();

    let snapshot = ReportSnapshot {
        generated_at: yaml_ops::now_utc_iso(),
        workspace: workspace.to_string_lossy().to_string(),
        product: ProductSummary {
            vision: vision_text,
            scope_status: "active".to_string(),
        },
        metrics: Metrics {
            total_stories: stories.len(),
            total_handoffs: handoffs.len(),
            total_findings: findings.len(),
            open_findings,
            total_releases: releases.len(),
            pending_handoffs,
        },
        handoffs,
        findings,
        releases,
        stories,
    };

    // Write snapshot
    let output_dir = workspace.join(".state/reporting");
    fs::create_dir_all(&output_dir)?;
    let output_path = output_dir.join("report-snapshot.json");
    let json = serde_json::to_string_pretty(&snapshot)?;
    fs::write(&output_path, &json)?;

    tracing::info!(
        output_path = %output_path.display(),
        stories = snapshot.metrics.total_stories,
        handoffs = snapshot.metrics.total_handoffs,
        findings = snapshot.metrics.total_findings,
        releases = snapshot.metrics.total_releases,
        "report snapshot written"
    );

    println!("  Stories: {}", snapshot.metrics.total_stories);
    println!(
        "  Handoffs: {} ({} pending)",
        snapshot.metrics.total_handoffs, snapshot.metrics.pending_handoffs
    );
    println!(
        "  Findings: {} ({} open)",
        snapshot.metrics.total_findings, snapshot.metrics.open_findings
    );
    println!("  Releases: {}", snapshot.metrics.total_releases);
    println!("{} Wrote {}", "OK:".green().bold(), output_path.display());
    Ok(())
}

fn read_yaml_list(workspace: &Path, rel_path: &str, key: &str) -> Result<Vec<serde_json::Value>> {
    let path = workspace.join(rel_path);
    if !path.exists() {
        tracing::warn!(path = %rel_path, key = %key, "report snapshot source file missing; using empty list");
        return Ok(vec![]);
    }

    let content = fs::read_to_string(&path)?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&content)?;

    let Some(raw_items) = yaml.get(key) else {
        tracing::warn!(path = %rel_path, key = %key, "report snapshot source missing expected top-level key; using empty list");
        return Ok(vec![]);
    };

    let Some(sequence) = raw_items.as_sequence() else {
        tracing::warn!(path = %rel_path, key = %key, "report snapshot source key is not a YAML sequence; using empty list");
        return Ok(vec![]);
    };

    let items = sequence
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            match serde_json::to_value(item) {
                Ok(value) => Some(value),
                Err(error) => {
                    tracing::warn!(path = %rel_path, key = %key, index, error = %error, "failed to convert YAML item to JSON for report snapshot; item skipped");
                    None
                }
            }
        })
        .collect();

    Ok(items)
}

fn matches_status(item: &serde_json::Value, accepted: &[&str]) -> bool {
    item.get("status")
        .and_then(|status| status.as_str())
        .map(|status| status.to_ascii_lowercase())
        .map_or(false, |status| accepted.contains(&status.as_str()))
}
