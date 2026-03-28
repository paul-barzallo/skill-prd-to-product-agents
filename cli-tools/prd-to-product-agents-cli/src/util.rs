use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Write a file with UTF-8 no-BOM encoding, creating parent dirs.
pub fn write_utf8(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content).with_context(|| format!("writing {}", path.display()))
}

/// Normalize line endings to LF.
pub fn normalize_lf(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Write with LF normalization.
pub fn write_utf8_lf(path: &Path, content: &str) -> Result<()> {
    write_utf8(path, &normalize_lf(content))
}

/// Append content to a file, creating it if missing.
pub fn append_utf8_lf(path: &Path, content: &str) -> Result<()> {
    let existing = if path.exists() {
        fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?
    } else {
        String::new()
    };
    let sep = if existing.is_empty() || existing.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    write_utf8_lf(path, &format!("{existing}{sep}{}", normalize_lf(content)))
}

/// SHA-256 of a string, returned as hex.
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// SHA-256 of file content.
pub fn file_hash(path: &Path) -> Result<String> {
    let content =
        fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(sha256_hex(&content))
}

/// SHA-256 of raw file bytes.
pub fn file_hash_bytes(path: &Path) -> Result<String> {
    let content = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Get the current UTC timestamp as ISO 8601.
pub fn now_utc() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Convert path to relative posix format from a base.
pub fn to_relative_posix(full: &Path, base: &Path) -> String {
    let full_str = full.to_string_lossy();
    let base_str = base.to_string_lossy();
    let rel = if full_str.starts_with(base_str.as_ref()) {
        &full_str[base_str.len()..]
    } else {
        &full_str
    };
    rel.trim_start_matches(['\\', '/'])
        .replace('\\', "/")
}

/// Read a YAML scalar value by dotted key path using serde_yaml.
pub fn yaml_scalar(path: &Path, key_path: &str) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)?;
    Ok(yaml_scalar_from_str(&content, key_path))
}

/// Read a YAML scalar value from content by dotted key path.
pub fn yaml_scalar_from_str(content: &str, key_path: &str) -> Option<String> {
    let yaml: serde_yaml::Value = serde_yaml::from_str(content).ok()?;
    let mut current = &yaml;
    for key in key_path.split('.') {
        current = current.get(key)?;
    }
    match current {
        serde_yaml::Value::String(s) => Some(s.clone()),
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        serde_yaml::Value::Null => None,
        _ => None,
    }
}

/// Read a YAML boolean from a capabilities file.
pub fn yaml_bool(path: &Path, key_path: &str, default: bool) -> bool {
    yaml_scalar(path, key_path)
        .ok()
        .flatten()
        .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on"))
        .unwrap_or(default)
}

#[derive(Debug)]
pub struct CommandResult {
    pub stdout: String,
}

fn log_command_failure(name: &str, args: &[&str], output: &std::process::Output) {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let command_line = if args.is_empty() {
        name.to_string()
    } else {
        format!("{} {}", name, args.join(" "))
    };

    tracing::warn!(
        command = %command_line,
        exit_code = output.status.code(),
        "command returned non-success status"
    );

    if !stdout.is_empty() {
        tracing::debug!(command = %command_line, stdout = %stdout, "command stdout");
    }
    if !stderr.is_empty() {
        tracing::debug!(command = %command_line, stderr = %stderr, "command stderr");
    }
}

#[allow(dead_code)]
pub fn log_subprocess_failure(command: &str, target: &str, output: &std::process::Output) {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

    tracing::warn!(command, target = %target, exit_code = output.status.code(), "subprocess returned non-success status");

    if !stdout.is_empty() {
        tracing::debug!(command, target = %target, stdout = %stdout, "subprocess stdout");
    }
    if !stderr.is_empty() {
        tracing::debug!(command, target = %target, stderr = %stderr, "subprocess stderr");
        eprintln!("[Preflight Command Failed] {}", stderr);
    }
}

fn command_result(output: std::process::Output) -> CommandResult {
    CommandResult {
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
    }
}

/// Check if a command is available on PATH.
pub fn command_exists(name: &str) -> bool {
    match std::process::Command::new(name).arg("--version").output() {
        Ok(output) => {
            if !output.status.success() {
                log_command_failure(name, &["--version"], &output);
                let err = String::from_utf8_lossy(&output.stderr);
                if !err.trim().is_empty() {
                    eprintln!("[Preflight Warning] '{}' exists but returned error on --version: {}", name, err.trim());
                }
                return false;
            }
            true
        }
        Err(_) => false,
    }
}

/// SQLite runtime support is bundled into the packaged CLI via rusqlite.
pub fn sqlite_runtime_available() -> bool {
    true
}

/// Standalone sqlite3 CLI availability is optional.
pub fn sqlite_cli_available() -> bool {
    command_exists("sqlite3")
}

/// Run a command and capture stdout/stderr and exit status.
pub fn command_capture(name: &str, args: &[&str], cwd: Option<&Path>) -> Result<CommandResult> {
    let mut command = std::process::Command::new(name);
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command
        .output()
        .with_context(|| format!("running {name}"))?;
    if !output.status.success() {
        log_command_failure(name, args, &output);
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let command_line = if args.is_empty() {
            name.to_string()
        } else {
            format!("{} {}", name, args.join(" "))
        };

        if stderr.is_empty() {
            bail!(
                "command '{}' failed with exit code {:?}{}",
                command_line,
                output.status.code(),
                if stdout.is_empty() {
                    String::new()
                } else {
                    format!("; stdout: {}", stdout)
                }
            );
        }

        bail!(
            "command '{}' failed with exit code {:?}: {}",
            command_line,
            output.status.code(),
            stderr
        );
    }
    Ok(command_result(output))
}

/// Run a command and capture stdout.
pub fn command_output(name: &str, args: &[&str]) -> Result<String> {
    Ok(command_capture(name, args, None)?.stdout)
}

/// Run a command, return success/failure.
pub fn command_ok(name: &str, args: &[&str]) -> bool {
    match std::process::Command::new(name).args(args).output() {
        Ok(output) => {
            if output.status.success() {
                true
            } else {
                log_command_failure(name, args, &output);
                let err = String::from_utf8_lossy(&output.stderr);
                if !err.trim().is_empty() {
                    eprintln!("[Preflight Warning] '{} {}' failed: {}", name, args.join(" "), err.trim());
                }
                false
            }
        }
        Err(e) => {
            tracing::warn!(command = %name, error = %e, "failed to execute command");
            eprintln!("[Preflight Warning] Could not execute '{}': {}", name, e);
            false
        }
    }
}

/// Detect current OS.
pub fn detect_os() -> &'static str {
    if cfg!(target_os = "windows") {
        if std::env::var("WSL_DISTRO_NAME").is_ok() || std::env::var("WSL_INTEROP").is_ok() {
            "wsl"
        } else {
            "windows"
        }
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        if std::env::var("WSL_DISTRO_NAME").is_ok() || std::env::var("WSL_INTEROP").is_ok() {
            "wsl"
        } else {
            "linux"
        }
    } else {
        "unknown"
    }
}

/// Detect current host environment.
pub fn detect_host() -> &'static str {
    if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
        || std::env::var("GITHUB_WORKFLOW").is_ok()
    {
        "github"
    } else if std::env::var("VSCODE_PID").is_ok()
        || std::env::var("TERM_PROGRAM").as_deref() == Ok("vscode")
    {
        "vscode"
    } else {
        "local"
    }
}

/// Backup a file with timestamp, keeping N most recent.
pub fn backup_with_retention(path: &Path, keep: usize) -> Result<Option<std::path::PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }
    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let backup = path.with_file_name(format!(
        "{}.backup-{ts}",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    fs::rename(path, &backup)?;

    // Clean old backups
    if let Some(parent) = path.parent() {
        let base_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let pattern = format!("{base_name}.backup-");
        let mut backups: Vec<_> = fs::read_dir(parent)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with(&pattern)
            })
            .collect();
        backups.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
        for old in backups.iter().skip(keep) {
            let _ = fs::remove_file(old.path());
        }
    }
    Ok(Some(backup))
}

/// Read project VERSION when available, returning None outside a repository root.
pub fn read_version_if_present(skill_root: &Path) -> Result<Option<String>> {
    let Some(version_path) = find_project_version_path(skill_root) else {
        return Ok(None);
    };
    let content = fs::read_to_string(&version_path)
        .with_context(|| format!("reading VERSION at {}", version_path.display()))?;
    Ok(Some(content.trim().to_string()))
}

fn find_project_version_path(path: &Path) -> Option<PathBuf> {
    let project_root = resolve_project_root(path);
    let version_path = project_root.join("VERSION");
    version_path.is_file().then_some(version_path)
}

fn resolve_project_root(path: &Path) -> PathBuf {
    let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if is_repo_root(&resolved) {
        return resolved;
    }

    for ancestor in resolved.ancestors() {
        if is_repo_root(ancestor) {
            return ancestor.to_path_buf();
        }
    }

    resolved
}

fn is_skill_root(path: &Path) -> bool {
    path.join("SKILL.md").is_file()
        && path.join("templates").join("workspace").is_dir()
}

fn is_repo_root(path: &Path) -> bool {
    path.join("AGENTS.md").is_file()
        && path.join("VERSION").is_file()
        && path
            .join(".agents")
            .join("skills")
            .join("prd-to-product-agents")
            .join("templates")
            .join("workspace")
            .is_dir()
}

/// Resolve either the skill root itself or the repository root that contains it.
pub fn resolve_skill_root(path: &Path) -> PathBuf {
    let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if is_skill_root(&resolved) {
        return resolved;
    }

    let nested = resolved
        .join(".agents")
        .join("skills")
        .join("prd-to-product-agents");
    if is_skill_root(&nested) {
        return nested.canonicalize().unwrap_or(nested);
    }

    resolved
}
