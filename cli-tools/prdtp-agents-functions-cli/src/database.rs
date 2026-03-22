use anyhow::{bail, Result};
use clap::Args;
use colored::Colorize;
use rusqlite::Connection;
use std::fs;
use std::path::Path;

const PENDING_MARKER: &str = "sqlite-bootstrap.pending.md";

const REQUIRED_TABLES: &[&str] = &[
    "artifacts",
    "agent_activity_log",
    "agent_runs",
    "release_checks",
    "gate_checks",
    "security_checks",
    "client_reviews",
    "environment_events",
    "milestone_reports",
    "sync_runs",
    "sync_failures",
    "metrics",
    "schema_version",
];

#[derive(Args)]
pub struct InitArgs {
    /// Force recreate (backs up existing DB first)
    #[arg(long)]
    pub force: bool,
}

fn pending_marker_path(workspace: &Path) -> std::path::PathBuf {
    workspace.join(".state").join(PENDING_MARKER)
}

fn pending_marker_body() -> String {
    format!(
        "# SQLite Bootstrap Pending\n\n- Mode: DEGRADED\n- Reason: SQLite initialization was deferred\n- Recovery: Install sqlite3 if required and rerun `prdtp-agents-functions-cli database init`\n"
    )
}

fn write_pending_marker(workspace: &Path) -> Result<()> {
    let pending_path = pending_marker_path(workspace);
    if let Some(parent) = pending_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&pending_path, pending_marker_body())?;
    Ok(())
}

pub fn init(workspace: &Path, args: InitArgs) -> Result<()> {
    crate::common::capability_contract::require_policy_enabled(
        workspace,
        "sqlite",
        "database init",
    )?;
    init_with_mode(workspace, args, false)
}

pub(crate) fn init_with_mode(workspace: &Path, args: InitArgs, degraded_mode: bool) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), force = args.force, degraded_mode, "initializing sqlite audit ledger");
    let state_dir = workspace.join(".state");
    let db_path = state_dir.join("project_memory.db");
    let sql_path = state_dir.join("memory-schema.sql");
    let pending_path = pending_marker_path(workspace);

    if !sql_path.exists() {
        bail!("Missing schema file: {}", sql_path.display());
    }

    if degraded_mode {
        if db_path.exists() && args.force {
            fs::remove_file(&db_path)?;
        }
        write_pending_marker(workspace)?;
        tracing::warn!(pending_path = %pending_path.display(), "sqlite initialization deferred; pending marker written");
        println!(
            "{} SQLite initialization deferred; pending marker written",
            "WARN:".yellow().bold()
        );
        return Ok(());
    }

    // Non-destructive rerun: preserve existing DB
    if db_path.exists() && !args.force {
        let conn = Connection::open(&db_path)?;
        let mut missing = 0u32;
        for table in REQUIRED_TABLES {
            let exists: bool = conn.query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |row| row.get(0),
            )?;
            if !exists {
                missing += 1;
            }
        }
        if missing == 0 {
            let table_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )?;
            let view_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='view'",
                [],
                |row| row.get(0),
            )?;
            let index_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )?;
            println!("  Tables: {table_count} | Views: {view_count} | Indices: {index_count}");
            println!("  DB preserved (use --force to rebuild).");
            tracing::info!(table_count, view_count, index_count, "existing sqlite database preserved");
            // Apply pending migrations
            migrate(workspace)?;
            return Ok(());
        } else {
            tracing::warn!(missing, "existing sqlite database missing required tables; rebuilding");
            eprintln!("{} {missing} table(s) missing — rebuilding with backup.", "WARN:".yellow().bold());
        }
    }

    // Backup before destructive rebuild
    if db_path.exists() {
        let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
        let backup_path = db_path.with_extension(format!("db.backup.{ts}"));
        fs::copy(&db_path, &backup_path)?;
        tracing::info!(backup_path = %backup_path.display(), "sqlite database backup created before rebuild");
        println!("  Backup: {}", backup_path.display());
        fs::remove_file(&db_path)?;
    }

    // Create fresh DB from schema
    let schema_sql = fs::read_to_string(&sql_path)?;
    let conn = Connection::open(&db_path)?;
    conn.execute_batch(&schema_sql)?;

    // Verify tables
    for table in REQUIRED_TABLES {
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name=?1",
            [table],
            |row| row.get(0),
        )?;
        if !exists {
            bail!("Expected table '{table}' not found after schema init.");
        }
    }

    let table_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
        [],
        |row| row.get(0),
    )?;
    let view_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='view'",
        [],
        |row| row.get(0),
    )?;
    let index_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name NOT LIKE 'sqlite_%'",
        [],
        |row| row.get(0),
    )?;

    println!("{} SQLite audit ledger initialized", "✓".green().bold());
    println!("  Tables: {table_count} | Views: {view_count} | Indices: {index_count}");
    tracing::info!(db_path = %db_path.display(), table_count, view_count, index_count, "sqlite audit ledger initialized");

    // Write bootstrap report
    let report_path = state_dir.join("sqlite-bootstrap.report.md");
    let report = format!(
        "# SQLite Bootstrap Report\n\n- Timestamp: {}\n- Database: .state/project_memory.db\n- Schema: .state/memory-schema.sql\n- Verified tables: {}\n- Total tables: {table_count}\n- Total views: {view_count}\n- Total indices: {index_count}\n",
        chrono::Utc::now().to_rfc3339(),
        REQUIRED_TABLES.join(", "),
    );
    fs::write(report_path, report)?;

    // Remove pending marker if exists
    let _ = fs::remove_file(pending_path);

    // Apply migrations
    migrate(workspace)?;

    Ok(())
}

pub fn migrate(workspace: &Path) -> Result<()> {
    crate::common::capability_contract::require_policy_enabled(
        workspace,
        "sqlite",
        "database migrate",
    )?;
    tracing::info!(workspace = %workspace.display(), "running sqlite schema migrations");
    let db_path = workspace.join(".state/project_memory.db");
    let migrations_dir = workspace.join(".state/migrations");

    if !db_path.exists() {
        bail!("Database not found at {}. Run prdtp-agents-functions-cli database init first.", db_path.display());
    }

    let conn = Connection::open(&db_path)?;

    let current_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    tracing::info!(current_version, "loaded current sqlite schema version");
    println!("Current schema version: {current_version}");

    if current_version < 5 {
        bail!("schema_version {current_version} is unsupported. Only clean bootstrap or existing v5+ databases are supported.");
    }

    if !migrations_dir.exists() {
        tracing::info!(path = %migrations_dir.display(), "migrations directory not found; schema already up to date");
        println!("No migrations directory found. Schema is up to date.");
        return Ok(());
    }

    let mut migration_files: Vec<_> = fs::read_dir(&migrations_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.ends_with(".sql") && name.chars().next().map_or(false, |c| c.is_ascii_digit())
        })
        .collect();
    migration_files.sort_by_key(|e| e.file_name().to_string_lossy().to_string());

    let mut applied = 0u32;

    for mf in &migration_files {
        let name = mf.file_name().to_string_lossy().to_string();
        let version_str: String = name.chars().take_while(|c| c.is_ascii_digit()).collect();
        let file_version: i64 = version_str.parse().unwrap_or(0);

        if file_version <= current_version {
            continue;
        }

        tracing::info!(migration = %name, file_version, "applying sqlite migration");
        println!("Applying migration {name} (v{file_version})...");

        // Backup before migration
        let backup_path = db_path.with_extension(format!("db.pre-migration-{file_version}.backup"));
        fs::copy(&db_path, &backup_path)?;

        let sql = fs::read_to_string(mf.path())?;
        match conn.execute_batch(&sql) {
            Ok(()) => {
                tracing::info!(migration = %name, file_version, "sqlite migration applied successfully");
                println!("  {} Migration {name} applied.", "OK:".green());
                applied += 1;
                let _ = fs::remove_file(&backup_path);
            }
            Err(e) => {
                tracing::error!(migration = %name, file_version, error = %e, backup_path = %backup_path.display(), "sqlite migration failed");
                eprintln!("  {} Migration {name} failed: {e}", "ERROR:".red().bold());
                eprintln!("  Backup preserved at: {}", backup_path.display());
                bail!("Migration failed. Applied {applied} migration(s) before failure.");
            }
        }
    }

    let new_version: i64 = conn
        .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |row| row.get(0))
        .unwrap_or(current_version);

    println!("\nMigration summary:");
    println!("  Previous version: {current_version}");
    println!("  Current version:  {new_version}");
    println!("  Applied: {applied}");
    tracing::info!(current_version, new_version, applied, "sqlite migration summary complete");

    Ok(())
}
