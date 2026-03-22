use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub fn normalize_lf(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn to_relative_posix(full: &Path, base: &Path) -> String {
    let full_str = full.to_string_lossy();
    let base_str = base.to_string_lossy();
    let rel = if full_str.starts_with(base_str.as_ref()) {
        &full_str[base_str.len()..]
    } else {
        &full_str
    };
    rel.trim_start_matches(['\\', '/']).replace('\\', "/")
}

pub fn yaml_scalar(path: &Path, key_path: &str) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)?;
    Ok(yaml_scalar_from_str(&content, key_path))
}

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

pub fn yaml_bool(path: &Path, key_path: &str, default: bool) -> bool {
    yaml_scalar(path, key_path)
        .ok()
        .flatten()
        .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on"))
        .unwrap_or(default)
}

#[derive(Debug)]
pub struct CommandResult {
    pub success: bool,
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl CommandResult {
    pub fn combined_output(&self) -> String {
        match (self.stdout.trim(), self.stderr.trim()) {
            ("", "") => String::new(),
            (stdout, "") => stdout.to_string(),
            ("", stderr) => stderr.to_string(),
            (stdout, stderr) => format!("{stdout}\n{stderr}"),
        }
    }
}

fn command_result(output: std::process::Output) -> CommandResult {
    CommandResult {
        success: output.status.success(),
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    }
}

pub fn command_exists(name: &str) -> bool {
    std::process::Command::new(name)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

pub fn command_capture(name: &str, args: &[&str], cwd: Option<&Path>) -> Result<CommandResult> {
    let mut command = std::process::Command::new(name);
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command.output().with_context(|| format!("running {name}"))?;
    Ok(command_result(output))
}

pub fn executable_capture(
    executable: &Path,
    args: &[&str],
    cwd: Option<&Path>,
) -> Result<CommandResult> {
    let mut command = std::process::Command::new(executable);
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command
        .output()
        .with_context(|| format!("running {}", executable.display()))?;
    Ok(command_result(output))
}

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

fn is_skill_root(path: &Path) -> bool {
    path.join("SKILL.md").is_file()
        && path.join("VERSION").is_file()
        && path.join("templates").join("workspace").is_dir()
}

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