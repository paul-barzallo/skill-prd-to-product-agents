use anyhow::{Context, Result};
use rusqlite::Connection;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

use crate::yaml_ops;

/// Determines if SQLite is available and returns the DB path if so.
pub fn sqlite_db_path(workspace: &Path) -> Option<PathBuf> {
    let db = workspace.join(".state/project_memory.db");
    if db.exists() {
        Some(db)
    } else {
        None
    }
}

/// Open a connection to the audit ledger.
pub fn open_db(db_path: &Path) -> Result<Connection> {
    Connection::open(db_path).with_context(|| format!("Opening DB {}", db_path.display()))
}

/// Best-effort audit log: try SQLite, fall back to JSON spool.
pub fn try_audit_activity(
    workspace: &Path,
    agent_role: &str,
    activity_type: &str,
    entity_type: &str,
    entity_id: &str,
    details: &str,
) -> Result<()> {
    let ts = yaml_ops::now_utc_iso();
    let id = yaml_ops::new_auto_id("al-");

    if let Some(db_path) = sqlite_db_path(workspace) {
        if let Ok(conn) = open_db(&db_path) {
            let sql = "INSERT OR IGNORE INTO agent_activity_log \
                       (id, timestamp_utc, agent_role, activity_type, entity_type, entity_id, details) \
                       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";
            if conn
                .execute(sql, rusqlite::params![id, ts, agent_role, activity_type, entity_type, entity_id, details])
                .is_ok()
            {
                return Ok(());
            }
        }
    }

    // Fallback: write JSON spool
    write_spool(workspace, &id, agent_role, activity_type, entity_type, entity_id, details, &ts)
}

/// Write a degraded-mode JSON spool entry.
pub fn write_spool(
    workspace: &Path,
    id: &str,
    agent_role: &str,
    activity_type: &str,
    entity_type: &str,
    entity_id: &str,
    details: &str,
    timestamp: &str,
) -> Result<()> {
    let spool_dir = workspace.join(".state/audit-spool");
    fs::create_dir_all(&spool_dir)?;

    let payload = json!({
        "id": id,
        "timestamp_utc": timestamp,
        "agent_role": agent_role,
        "activity_type": activity_type,
        "entity_type": entity_type,
        "entity_id": entity_id,
        "details": details
    });
    let filename = format!("{id}.json");
    let path = spool_dir.join(filename);
    fs::write(&path, serde_json::to_string_pretty(&payload)?)?;
    Ok(())
}

/// Write a degraded state record (for state-ops when SQLite is unavailable).
pub fn write_degraded_record(
    workspace: &Path,
    operation: &str,
    entity_type: &str,
    entity_id: &str,
    agent_role: &str,
    details: &str,
) -> Result<()> {
    let dir = workspace.join(".state/degraded-ops");
    fs::create_dir_all(&dir)?;

    let ts = yaml_ops::now_utc_iso();
    let payload = json!({
        "operation": operation,
        "entity_type": entity_type,
        "entity_id": entity_id,
        "agent_role": agent_role,
        "timestamp_utc": ts,
        "details": details
    });
    let filename = format!(
        "{}-{}-{}.json",
        entity_type,
        entity_id,
        chrono::Utc::now().format("%Y%m%dT%H%M%S")
    );
    fs::write(dir.join(filename), serde_json::to_string_pretty(&payload)?)?;
    Ok(())
}
