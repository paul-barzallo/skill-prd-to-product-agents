use anyhow::{Context, Result};
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::filter::EnvFilter;

const LOG_FILE_NAME: &str = "cli-diagnostic.log";
const DEFAULT_LOG_LEVEL: &str = "info";

pub fn init(workspace: &Path) -> Result<WorkerGuard> {
    let log_dir = diagnostic_log_dir(workspace);
    let log_path = log_dir.join(LOG_FILE_NAME);
    fs::create_dir_all(&log_dir)
        .with_context(|| format!("failed to create log directory {}", log_dir.display()))?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("failed to create log file {}", log_path.display()))?;

    let file_appender = tracing_appender::rolling::never(&log_dir, LOG_FILE_NAME);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_LEVEL));

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(non_blocking)
        .with_ansi(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .context("failed to initialize tracing subscriber")?;

    tracing::info!(
        log_path = %log_path.display(),
        "structured logging initialized"
    );

    Ok(guard)
}

fn diagnostic_log_dir(workspace: &Path) -> PathBuf {
    workspace.join(".state").join("logs")
}