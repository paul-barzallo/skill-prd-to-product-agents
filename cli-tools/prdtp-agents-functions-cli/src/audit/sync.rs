use anyhow::{bail, Result};
use colored::Colorize;
use std::path::Path;
use walkdir::WalkDir;

use crate::common::audit;
use crate::common::yaml_ops;

pub fn run(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "running audit sync");
    println!("{}", "=== Audit Sync ===".cyan().bold());
    crate::common::capability_contract::require_policy_enabled(
        workspace,
        "sqlite",
        "audit sync",
    )?;

    let db_path = match audit::sqlite_db_path(workspace) {
        Some(p) => p,
        None => {
            tracing::warn!(workspace = %workspace.display(), "sqlite db unavailable; writing degraded audit sync log");
            eprintln!(
                "  {} SQLite DB not found — writing degraded log",
                "⚠".yellow()
            );
            let log_path = workspace.join(".state/state-sync-degraded.log");
            if let Some(parent) = log_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(
                &log_path,
                format!("[{}] state-sync skipped: SQLite unavailable\n", yaml_ops::now_utc_iso()),
            )?;
            tracing::info!(log_path = %log_path.display(), "degraded audit sync log written");
            println!("{} Degraded sync log written", "OK:".yellow().bold());
            return Ok(());
        }
    };

    let conn = audit::open_db(&db_path)?;

    // Tables already exist from init_sqlite_db — no CREATE needed for existing DB.
    // If tables are missing (fresh DB), the fallback CREATE uses the real schema.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS artifacts (
            id TEXT PRIMARY KEY,
            artifact_type TEXT NOT NULL,
            title TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            status TEXT NOT NULL DEFAULT 'draft',
            owner_roles TEXT NOT NULL,
            checksum TEXT,
            last_synced_by_role TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS agent_activity_log (
            id TEXT PRIMARY KEY,
            agent_role TEXT NOT NULL,
            activity_type TEXT NOT NULL,
            entity_type TEXT,
            entity_ref TEXT,
            summary TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS sync_runs (
            id TEXT PRIMARY KEY,
            triggered_by_role TEXT NOT NULL DEFAULT 'pm-orchestrator',
            started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            finished_at TEXT,
            result TEXT NOT NULL DEFAULT 'started',
            processed_artifacts INTEGER DEFAULT 0,
            changed_artifacts INTEGER DEFAULT 0,
            failures_count INTEGER DEFAULT 0,
            notes TEXT
        );"
    )?;

    let docs_path = workspace.join("docs/project");
    if !docs_path.exists() {
        bail!("docs/project/ not found");
    }

    // Keep YAML-derived sync consistent with the same advisory locks used by state ops.
    let _yaml_locks = [
        workspace.join("docs/project/handoffs.yaml"),
        workspace.join("docs/project/findings.yaml"),
        workspace.join("docs/project/releases.yaml"),
    ]
    .into_iter()
    .filter(|path| path.exists())
    .map(|path| yaml_ops::YamlLock::acquire(&path))
    .collect::<Result<Vec<_>>>()?;

    let sync_id = yaml_ops::new_auto_id("sr-");
    let ts = yaml_ops::now_utc_iso();
    let mut processed = 0u32;
    let mut changed = 0u32;
    let mut skipped = 0u32;
    let mut failures = 0u32;

    for entry in WalkDir::new(&docs_path)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !["md", "yaml", "yml"].contains(&ext) {
            continue;
        }

        processed += 1;
        let rel = path
            .strip_prefix(workspace)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        let checksum = match yaml_ops::file_content_hash(path) {
            Ok(h) => h,
            Err(_) => {
                tracing::warn!(path = %rel, "failed to compute checksum during audit sync");
                failures += 1;
                continue;
            }
        };

        let artifact_type = determine_artifact_type(&rel);
        let owner_role = determine_owner_role(&artifact_type);

        // Check existing
        let existing_checksum: Option<String> = conn
            .query_row(
                "SELECT checksum FROM artifacts WHERE path = ?1",
                rusqlite::params![rel],
                |row| row.get(0),
            )
            .ok();

        if existing_checksum.as_deref() == Some(checksum.as_str()) {
            skipped += 1;
            continue;
        }

        changed += 1;
        let art_id = yaml_ops::new_auto_id("art-");
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        conn.execute(
            "INSERT INTO artifacts (id, path, artifact_type, title, owner_roles, checksum, status, last_synced_by_role, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'approved', 'pm-orchestrator', ?7) \
             ON CONFLICT(path) DO UPDATE SET checksum=excluded.checksum, updated_at=excluded.updated_at, last_synced_by_role=excluded.last_synced_by_role",
            rusqlite::params![art_id, rel, artifact_type, title, owner_role, checksum, ts],
        )?;

        // Log activity
        let log_id = yaml_ops::new_auto_id("al-");
        let _ = conn.execute(
            "INSERT OR IGNORE INTO agent_activity_log (id, agent_role, activity_type, entity_type, entity_ref, summary) \
             VALUES (?1, 'pm-orchestrator', 'artifact_synced', 'artifact', ?2, ?3)",
            rusqlite::params![log_id, rel, format!("checksum={checksum}")],
        );
    }

    // Orphan detection
    let mut stmt = conn.prepare("SELECT path FROM artifacts WHERE status != 'removed'")?;
    let db_paths: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    let mut orphans = 0u32;
    for db_rel in &db_paths {
        let full = workspace.join(db_rel);
        if !full.exists() {
            orphans += 1;
            let _ = conn.execute(
                "UPDATE artifacts SET status = 'removed', updated_at = ?1 WHERE path = ?2",
                rusqlite::params![ts, db_rel],
            );
        }
    }

    // Record sync run
    let result = if failures > 0 { "partial" } else { "completed" };
    let notes = format!("processed={processed} changed={changed} skipped={skipped} orphans={orphans}");
    conn.execute(
        "INSERT INTO sync_runs (id, triggered_by_role, started_at, finished_at, result, processed_artifacts, changed_artifacts, failures_count, notes) \
         VALUES (?1, 'pm-orchestrator', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![sync_id, ts, yaml_ops::now_utc_iso(), result, processed, changed, failures, notes],
    )?;

    println!("\n{}", "────────────────────────────".dimmed());
    println!("  Processed: {processed}");
    println!("  Changed (drift): {changed}");
    println!("  Skipped (same): {skipped}");
    println!("  Failures: {failures}");
    println!("  Orphans: {orphans}");
    tracing::info!(sync_id = %sync_id, processed, changed, skipped, failures, orphans, result, "audit sync completed");
    println!("{} Sync {sync_id} — {result}", "OK:".green().bold());
    Ok(())
}

fn determine_artifact_type(rel_path: &str) -> String {
    let lower = rel_path.to_lowercase();
    if lower.contains("vision") { return "vision".into(); }
    if lower.contains("scope") { return "scope".into(); }
    if lower.contains("backlog") { return "backlog".into(); }
    if lower.contains("refined-stories") { return "refined-stories".into(); }
    if lower.contains("acceptance-criteria") { return "acceptance-criteria".into(); }
    if lower.contains("handoffs") { return "handoff".into(); }
    if lower.contains("findings") { return "finding".into(); }
    if lower.contains("releases") { return "release".into(); }
    if lower.contains("quality-gates") { return "gate".into(); }
    if lower.contains("context-summary") { return "context-summary".into(); }
    if lower.contains("architecture/") { return "architecture".into(); }
    if lower.contains("decisions/") { return "decision".into(); }
    if lower.contains("ux/") { return "ux".into(); }
    if lower.contains("qa/") { return "report".into(); }
    if lower.contains("release/") { return "release-tracker".into(); }
    if lower.contains("board") { return "report".into(); }
    if lower.contains("dashboard") { return "report".into(); }
    if lower.contains("risks") { return "risk".into(); }
    "other".into()
}

fn determine_owner_role(artifact_type: &str) -> String {
    match artifact_type {
        "vision" | "scope" | "backlog" | "refined-stories" | "acceptance-criteria" => {
            "product-owner".into()
        }
        "architecture" | "decision" => "software-architect".into(),
        "ux" => "ux-designer".into(),
        "finding" => "qa-lead".into(),
        "release" | "release-tracker" => "devops-release-engineer".into(),
        "handoff" | "report" => "pm-orchestrator".into(),
        "gate" => "qa-lead".into(),
        _ => "pm-orchestrator".into(),
    }
}
