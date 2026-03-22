use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use super::{dashboard, export, snapshot};

/// Run snapshot → dashboard → export (CSV + XLSX) in sequence.
pub fn run(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "running report pack workflow");
    println!("{}", "=== Report Pack ===".cyan().bold());
    println!();
    crate::common::capability_contract::require_policy_enabled(
        workspace,
        "reporting",
        "report pack",
    )?;

    // 1. Generate snapshot
    snapshot::run(workspace)?;
    println!();

    // 2. Refresh dashboard
    dashboard::run(workspace)?;
    println!();

    // 3. Export CSV
    run_export_stage(workspace, "Export CSV", export::ExportFormat::Csv)?;
    println!();

    // 4. Export XLSX
    run_export_stage(workspace, "Export XLSX", export::ExportFormat::Xlsx)?;

    println!();
    tracing::info!("report pack workflow completed");
    println!("{} Report pack complete.", "✓".green().bold());
    Ok(())
}

fn run_export_stage(workspace: &Path, label: &str, format: export::ExportFormat) -> Result<()> {
    println!("{}", format!("── {label} ──").cyan());
    export::run(
        workspace,
        export::ExportArgs {
            format,
            output_dir: None,
            section: None,
        },
    )
}
