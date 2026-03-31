use anyhow::{Context, Result};
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::filter::EnvFilter;

const LOG_FILE_NAME: &str = "cli-diagnostic.log";
const DEFAULT_LOG_LEVEL: &str = "info";
const TEMP_LOG_DIR_NAME: &str = "prdtp-agents-functions-cli";

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
        "structured logging initialized"
    );

    Ok(guard)
}

fn diagnostic_log_dir(workspace: &Path) -> PathBuf {
    if is_packaged_template_workspace(workspace) {
        std::env::temp_dir().join(TEMP_LOG_DIR_NAME).join("logs")
    } else {
        workspace.join(".state").join("logs")
    }
}

fn is_packaged_template_workspace(workspace: &Path) -> bool {
    let Some(parent) = workspace.parent() else {
        return false;
    };
    let Some(grandparent) = parent.parent() else {
        return false;
    };

    workspace.file_name().and_then(|name| name.to_str()) == Some("workspace")
        && parent.file_name().and_then(|name| name.to_str()) == Some("templates")
        && grandparent.join("SKILL.md").is_file()
}

#[cfg(test)]
mod tests {
    use super::{diagnostic_log_dir, is_packaged_template_workspace, TEMP_LOG_DIR_NAME};
    use std::path::PathBuf;

    #[test]
    fn packaged_template_workspace_logs_to_temp() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("failed to resolve repository root")
            .to_path_buf();
        let template_workspace = repo_root
            .join(".agents")
            .join("skills")
            .join("prd-to-product-agents")
            .join("templates")
            .join("workspace");

        assert!(is_packaged_template_workspace(&template_workspace));
        assert_eq!(
            diagnostic_log_dir(&template_workspace),
            std::env::temp_dir().join(TEMP_LOG_DIR_NAME).join("logs")
        );
    }

    #[test]
    fn normal_workspace_logs_inside_workspace_state() {
        let workspace = tempfile::tempdir().expect("failed to create temp workspace");
        let expected = workspace.path().join(".state").join("logs");

        assert!(!is_packaged_template_workspace(workspace.path()));
        assert_eq!(diagnostic_log_dir(workspace.path()), expected);
    }
}
