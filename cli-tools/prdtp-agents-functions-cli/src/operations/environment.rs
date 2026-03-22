use anyhow::Result;
use clap::Args;
use colored::Colorize;
use std::path::Path;

use crate::common::enums::{Environment, EventType, Role, Severity};
use crate::common::yaml_ops;
use crate::common::audit;

#[derive(Args)]
pub struct RecordEventArgs {
    /// Environment name
    #[arg(long, value_enum)]
    pub(crate) env_name: Environment,
    /// Event type
    #[arg(long, value_enum)]
    pub(crate) event_type: EventType,
    /// Role reporting the event
    #[arg(long, value_enum)]
    pub(crate) reported_by: Role,
    /// Optional build version
    #[arg(long)]
    pub(crate) build_version: Option<String>,
    /// Optional severity
    #[arg(long, value_enum)]
    pub(crate) severity: Option<Severity>,
    /// Optional notes
    #[arg(long)]
    pub(crate) notes: Option<String>,
    /// Optional custom ID
    #[arg(long)]
    pub(crate) id: Option<String>,
}

pub fn record(workspace: &Path, args: RecordEventArgs) -> Result<()> {
    tracing::info!(
        workspace = %workspace.display(),
        environment = %args.env_name,
        event_type = %args.event_type,
        reported_by = %args.reported_by,
        severity = ?args.severity,
        "recording environment event"
    );
    println!("{}", "=== Record Environment Event ===".cyan().bold());

    let id = args.id.unwrap_or_else(|| yaml_ops::new_auto_id("ee-"));
    let ts = yaml_ops::now_utc_iso();

    // Try SQLite first
    if let Some(db_path) = audit::sqlite_db_path(workspace) {
        if let Ok(conn) = audit::open_db(&db_path) {
            let severity_str = args.severity.map(|s| s.to_string()).unwrap_or_default();
            let build = args.build_version.as_deref().unwrap_or("");
            let notes = args.notes.as_deref().unwrap_or("");

            let sql = "INSERT INTO environment_events \
                       (id, timestamp_utc, environment, event_type, reported_by, build_version, severity, notes) \
                       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";

            match conn.execute(
                sql,
                rusqlite::params![
                    id,
                    ts,
                    args.env_name.to_string(),
                    args.event_type.to_string(),
                    args.reported_by.to_string(),
                    build,
                    severity_str,
                    notes
                ],
            ) {
                Ok(_) => {
                    tracing::info!(event_id = %id, storage = "sqlite", "environment event recorded");
                    println!("{} Recorded event {id} in SQLite", "OK:".green().bold());
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!(event_id = %id, error = %e, "sqlite insert failed; falling back to degraded spool");
                    eprintln!(
                        "  {} SQLite insert failed: {e} — falling back to spool",
                        "⚠".yellow()
                    );
                }
            }
        }
    }

    // Fallback: JSON spool
    let details = format!(
        "env={}, type={}, build={}, severity={}, notes={}",
        args.env_name,
        args.event_type,
        args.build_version.as_deref().unwrap_or(""),
        args.severity.map(|s| s.to_string()).unwrap_or_default(),
        args.notes.as_deref().unwrap_or("")
    );

    audit::write_degraded_record(
        workspace,
        "record_environment_event",
        "environment_event",
        &id,
        &args.reported_by.to_string(),
        &details,
    )?;

    tracing::warn!(event_id = %id, storage = "degraded-spool", "environment event recorded to spool because sqlite was unavailable");
    println!(
        "{} Recorded event {id} to degraded spool (SQLite unavailable)",
        "OK:".yellow().bold()
    );
    Ok(())
}
