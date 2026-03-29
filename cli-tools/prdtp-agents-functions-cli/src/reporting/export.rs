use anyhow::{bail, Result};
use clap::{Args, ValueEnum};
use colored::Colorize;
use std::fs;
use std::path::Path;

#[derive(Clone, ValueEnum)]
pub enum ExportFormat {
    Csv,
    Xlsx,
}

#[derive(Args)]
pub struct ExportArgs {
    /// Export format (csv, xlsx)
    #[arg(long, value_enum, default_value = "csv")]
    pub format: ExportFormat,
    /// Output directory (defaults to .state/exports/)
    #[arg(long)]
    pub output_dir: Option<String>,
    /// Only export a specific section (findings, handoffs, releases)
    #[arg(long)]
    pub section: Option<String>,
}

pub fn run(workspace: &Path, args: ExportArgs) -> Result<()> {
    crate::common::capability_contract::require_policy_enabled(
        workspace,
        "reporting",
        "report export",
    )?;
    let format_name = match &args.format {
        ExportFormat::Csv => "csv",
        ExportFormat::Xlsx => "xlsx",
    };
    tracing::info!(
        workspace = %workspace.display(),
        format = format_name,
        section = ?args.section,
        output_dir = ?args.output_dir,
        "exporting reporting artifacts"
    );
    let snapshot_path = workspace.join(".state/reporting/report-snapshot.json");
    if !snapshot_path.exists() {
        bail!("No report-snapshot.json found. Run `prdtp-agents-functions-cli report snapshot` first.");
    }

    let snapshot_str = fs::read_to_string(&snapshot_path)?;
    let snapshot: serde_json::Value = serde_json::from_str(&snapshot_str)?;

    let out_dir = match args.output_dir {
        Some(ref d) => workspace.join(d),
        None => workspace.join(".state/exports"),
    };
    fs::create_dir_all(&out_dir)?;

    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");

    let sections = selected_sections(args.section.as_deref());

    match args.format {
        ExportFormat::Csv => export_csv(&snapshot, &out_dir, &ts.to_string(), &sections)?,
        ExportFormat::Xlsx => export_xlsx(&snapshot, &out_dir, &ts.to_string(), &sections)?,
    }

    Ok(())
}

fn export_csv(
    snapshot: &serde_json::Value,
    out_dir: &Path,
    ts: &str,
    sections: &[&str],
) -> Result<()> {
    for section in sections {
        let Some(items) = snapshot_section_items(snapshot, section) else {
            log_skipped_section(section, "not found or empty in snapshot");
            continue;
        };

        if items.is_empty() {
            log_skipped_section(section, "is empty");
            continue;
        }

        // Collect all keys from first item as header
        let headers: Vec<String> = match items[0].as_object() {
            Some(obj) => obj.keys().cloned().collect(),
            None => continue,
        };

        let mut csv = headers.join(",") + "\n";
        for item in items {
            let row: Vec<String> = headers
                .iter()
                .map(|h| {
                    let val = item.get(h).cloned().unwrap_or(serde_json::Value::Null);
                    csv_escape(&json_cell_string(&val))
                })
                .collect();
            csv.push_str(&row.join(","));
            csv.push('\n');
        }

        let file_path = out_dir.join(format!("{section}-{ts}.csv"));
        fs::write(&file_path, &csv)?;
        tracing::info!(section = %section, file_path = %file_path.display(), "csv export written");
        println!("  {} {}", "✓".green().bold(), file_path.display());
    }

    Ok(())
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn export_xlsx(
    snapshot: &serde_json::Value,
    out_dir: &Path,
    ts: &str,
    sections: &[&str],
) -> Result<()> {
    use rust_xlsxwriter::Workbook;

    let file_path = out_dir.join(format!("report-{ts}.xlsx"));
    let mut workbook = Workbook::new();

    for section in sections {
        let Some(items) = snapshot_section_items(snapshot, section) else {
            continue;
        };

        if items.is_empty() {
            continue;
        }

        let sheet = workbook.add_worksheet();
        sheet.set_name(*section)?;

        let headers: Vec<String> = match items[0].as_object() {
            Some(obj) => obj.keys().cloned().collect(),
            None => continue,
        };

        // Write headers
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string(0, col as u16, header)?;
        }

        // Write data
        for (row_idx, item) in items.iter().enumerate() {
            for (col_idx, header) in headers.iter().enumerate() {
                let val = item.get(header).cloned().unwrap_or(serde_json::Value::Null);
                let cell_str = json_cell_string(&val);
                sheet.write_string((row_idx + 1) as u32, col_idx as u16, &cell_str)?;
            }
        }
    }

    workbook.save(&file_path)?;
    tracing::info!(file_path = %file_path.display(), sections = ?sections, "xlsx export written");
    println!("  {} {}", "✓".green().bold(), file_path.display());

    Ok(())
}

fn selected_sections(section: Option<&str>) -> Vec<&str> {
    match section {
        Some(section) => vec![section],
        None => vec!["findings", "handoffs", "releases"],
    }
}

fn snapshot_section_items<'a>(
    snapshot: &'a serde_json::Value,
    section: &str,
) -> Option<&'a [serde_json::Value]> {
    snapshot.get(section)?.as_array().map(Vec::as_slice)
}

fn log_skipped_section(section: &str, reason: &str) {
    tracing::debug!(section = %section, reason, "report export section skipped");
    println!("  {} Section '{}' {}.", "SKIP:".yellow(), section, reason);
}

fn json_cell_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(string) => string.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}
