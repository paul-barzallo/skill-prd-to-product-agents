use anyhow::{Context, Result};
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::filter::EnvFilter;

const LOG_FILE_NAME: &str = "cli-diagnostic.log";
const DEFAULT_LOG_LEVEL: &str = "info";
const TEMP_LOG_DIR_NAME: &str = "prd-to-product-agents-cli";

pub fn init(skill_root: &Path, use_temp_dir: bool) -> Result<WorkerGuard> {
    let log_dir = diagnostic_log_dir(skill_root, use_temp_dir);
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
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_LEVEL));

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(non_blocking)
        .with_ansi(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .context("failed to initialize tracing subscriber")?;

    tracing::info!(
        log_path = %log_path.display(),
        temp_log_dir = use_temp_dir,
        "structured logging initialized"
    );

    Ok(guard)
}

fn diagnostic_log_dir(_skill_root: &Path, _use_temp_dir: bool) -> PathBuf {
    // The skill CLI must never mutate the distributed package during bootstrap or validation.
    // Keep diagnostics out of the skill root so copied packages remain portable and clean.
    std::env::temp_dir().join(TEMP_LOG_DIR_NAME).join("logs")
}
