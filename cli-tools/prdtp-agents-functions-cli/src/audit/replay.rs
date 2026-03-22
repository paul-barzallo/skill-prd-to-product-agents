use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::common::audit;

pub fn run(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "replaying audit spool");
    println!("{}", "=== Replay Audit Spool ===".cyan().bold());
    crate::common::capability_contract::require_policy_enabled(
        workspace,
        "sqlite",
        "audit replay-spool",
    )?;

    let spool_dir = workspace.join(".state/audit-spool");
    if !spool_dir.exists() || !spool_dir.is_dir() {
        tracing::info!(path = %spool_dir.display(), "audit spool directory not found; nothing to replay");
        println!("  No audit spool directory found — nothing to replay");
        return Ok(());
    }

    let entries: Vec<_> = fs::read_dir(&spool_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map_or(false, |ext| ext == "json")
        })
        .collect();

    if entries.is_empty() {
        tracing::info!(path = %spool_dir.display(), "audit spool directory empty; nothing to replay");
        println!("  Spool directory is empty — nothing to replay");
        return Ok(());
    }

    let db_path = match audit::sqlite_db_path(workspace) {
        Some(p) => p,
        None => {
            tracing::error!(workspace = %workspace.display(), "sqlite db unavailable; cannot replay audit spool");
            eprintln!(
                "{} SQLite DB not available — cannot replay spool",
                "ERROR:".red().bold()
            );
            std::process::exit(1);
        }
    };

    let conn = audit::open_db(&db_path)?;

    // Ensure table exists
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS agent_activity_log (
            id TEXT PRIMARY KEY,
            timestamp_utc TEXT,
            agent_role TEXT,
            activity_type TEXT,
            entity_type TEXT,
            entity_id TEXT,
            details TEXT
        );"
    )?;

    let mut replayed = 0u32;
    let mut failed = 0u32;
    let total = entries.len();

    for entry in entries {
        let path = entry.path();
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => {
                tracing::warn!(path = %path.display(), "failed to read audit spool entry");
                failed += 1;
                continue;
            }
        };

        let payload: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => {
                tracing::warn!(path = %path.display(), "failed to parse audit spool entry json");
                failed += 1;
                continue;
            }
        };

        let id = payload["id"].as_str().unwrap_or("");
        let ts = payload["timestamp_utc"].as_str().unwrap_or("");
        let role = payload["agent_role"].as_str().unwrap_or("");
        let activity = payload["activity_type"].as_str().unwrap_or("");
        let etype = payload["entity_type"].as_str().unwrap_or("");
        let eid = payload["entity_id"].as_str().unwrap_or("");
        let details = payload["details"].as_str().unwrap_or("");

        let sql = "INSERT OR IGNORE INTO agent_activity_log \
                   (id, timestamp_utc, agent_role, activity_type, entity_type, entity_id, details) \
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";

        match conn.execute(sql, rusqlite::params![id, ts, role, activity, etype, eid, details]) {
            Ok(_) => {
                let _ = fs::remove_file(&path);
                replayed += 1;
            }
            Err(_) => {
                tracing::warn!(path = %path.display(), activity_id = %id, "failed to replay audit spool entry into sqlite");
                failed += 1;
            }
        }
    }

    println!("\n{}", "────────────────────────────".dimmed());
    println!("  Total: {total}");
    println!("  Replayed: {replayed}");
    println!("  Failed: {failed}");
    tracing::info!(total, replayed, failed, "audit spool replay completed");
    println!("{} Spool replay complete", "OK:".green().bold());
    Ok(())
}
