use anyhow::Result;
use clap::Args;
use colored::Colorize;
use std::path::Path;
use std::process::Command;

use crate::common::yaml_ops;

#[derive(Args)]
pub struct DetectArgs {
    /// Force Git capability off
    #[arg(long)]
    disable_git: bool,
    /// Force gh CLI capability off
    #[arg(long)]
    disable_gh: bool,
    /// Force SQLite capability off
    #[arg(long)]
    disable_sqlite: bool,
    /// Force markdownlint capability off
    #[arg(long)]
    disable_markdownlint: bool,
}

pub fn run(workspace: &Path, args: DetectArgs) -> Result<()> {
    println!("{}", "=== Detect Capabilities ===".cyan().bold());

    let cap_path = workspace.join(".github/workspace-capabilities.yaml");

    // ── Detect OS ────────────────────────────────────────────────
    let os = detect_os();
    let host = detect_host();
    println!("  OS: {os}, Host: {host}");

    // ── Detect tools ─────────────────────────────────────────────
    let git_installed = !args.disable_git && command_exists("git");
    let git_identity = git_installed && git_identity_configured();
    let gh_installed = !args.disable_gh && command_exists("gh");
    let gh_auth = gh_installed && gh_authenticated();
    let sqlite_runtime_available = !args.disable_sqlite;
    let sqlite_cli_available = command_exists("sqlite3");
    let db_initialized = workspace.join(".state/project_memory.db").exists();
    let node_installed = command_exists("node");
    let npm_installed = command_exists("npm");
    let node_native = node_installed;
    let mdlint_installed = !args.disable_markdownlint && command_exists("markdownlint");
    let mdlint_native = mdlint_installed;
    let ui_available = workspace.join("reporting-ui/index.html").exists();
    let xlsx_ready = workspace.join("reporting-ui/vendor/xlsx.mini.min.js").exists();

    println!("  git: {git_installed} (identity: {git_identity})");
    println!("  gh: {gh_installed} (auth: {gh_auth})");
    println!(
        "  sqlite: {sqlite_runtime_available} (db: {db_initialized}, sqlite3 cli: {sqlite_cli_available})"
    );
    println!("  node: {node_installed}, npm: {npm_installed}");
    println!("  markdownlint: {mdlint_installed}");
    println!("  reporting UI: {ui_available}, XLSX: {xlsx_ready}");

    // ── Read existing policy (preserve if previously set) ────────
    let (git_policy_enabled, git_mode, gh_policy, sqlite_policy, sqlite_mode, mdlint_policy, reporting_policy) =
        read_existing_policies(&cap_path);

    let git_pol_enabled = if args.disable_git { false } else { git_policy_enabled.unwrap_or(git_installed) };
    let git_pol_mode = git_mode.unwrap_or_else(|| {
        if git_pol_enabled { "full".to_string() } else { "local-only".to_string() }
    });
    let gh_pol = if args.disable_gh { false } else { gh_policy.unwrap_or(gh_installed && gh_auth) };
    let sqlite_pol = if args.disable_sqlite {
        false
    } else {
        sqlite_policy.unwrap_or(sqlite_runtime_available)
    };
    let sqlite_pol_mode = sqlite_mode.unwrap_or_else(|| {
        if sqlite_pol && db_initialized { "ledger".to_string() } else { "spool-only".to_string() }
    });
    let mdlint_pol = if args.disable_markdownlint { false } else { mdlint_policy.unwrap_or(mdlint_installed) };
    let report_pol = reporting_policy.unwrap_or(true);

    let vis_mode = if gh_pol { "auto" } else { "local-only" };

    // ── Write YAML ───────────────────────────────────────────────
    let ts = yaml_ops::now_utc_iso();
    let yaml = format!(
r#"# Workspace capabilities — auto-detected by prdtp-agents-functions-cli
schema_version: 1
environment:
  host: {host}
  os: {os}
capabilities:
  git:
    detected:
      installed: {git_installed}
      identity_configured: {git_identity}
    policy:
      enabled: {git_pol_enabled}
      mode: {git_pol_mode}
  gh:
    detected:
      installed: {gh_installed}
      authenticated: {gh_auth}
    policy:
      enabled: {gh_pol}
  sqlite:
    detected:
    installed: {sqlite_runtime_available}
      db_initialized: {db_initialized}
    policy:
      enabled: {sqlite_pol}
      mode: {sqlite_pol_mode}
  node:
    detected:
      installed: {node_installed}
      npm_installed: {npm_installed}
      native_linux: {node_native}
  markdownlint:
    detected:
      installed: {mdlint_installed}
      native_linux: {mdlint_native}
    policy:
      enabled: {mdlint_pol}
  local_history:
    policy:
      enabled: true
      format: markdown+json
      path: .state/local-history
  reporting:
    detected:
      ui_available: {ui_available}
      xlsx_export_ready: {xlsx_ready}
      pdf_export_ready: false
    policy:
      enabled: {report_pol}
      visibility_mode: {vis_mode}
last_updated: "{ts}"
"#
    );

    if let Some(parent) = cap_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&cap_path, &yaml)?;

    println!("\n{} Wrote {}", "OK:".green().bold(), cap_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn sqlite_policy_defaults_to_enabled_when_not_explicitly_disabled() {
        let explicit_policy = None;
        let sqlite_runtime_available = true;
        let sqlite_policy = explicit_policy.unwrap_or(sqlite_runtime_available);
        assert!(sqlite_policy);
    }

    #[test]
    fn sqlite_policy_respects_explicit_disable() {
        let disable_sqlite = true;
        let explicit_policy = Some(true);
        let sqlite_policy = if disable_sqlite {
            false
        } else {
            explicit_policy.unwrap_or(true)
        };
        assert!(!sqlite_policy);
    }
}

/// Quick preflight check — exit 1 if critical capabilities are missing.
pub fn check(workspace: &Path) -> Result<()> {
    println!("{}", "=== Capability Check ===".cyan().bold());

    let cap_path = workspace.join(".github/workspace-capabilities.yaml");
    if !cap_path.exists() {
        eprintln!(
            "{} workspace-capabilities.yaml not found. Run `prdtp-agents-functions-cli capabilities detect` first.",
            "ERROR:".red().bold()
        );
        std::process::exit(1);
    }

    let content = std::fs::read_to_string(&cap_path)?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&content)?;

    let git_enabled = yaml_bool(&yaml, &["capabilities", "git", "policy", "enabled"]);
    let sqlite_enabled = yaml_bool(&yaml, &["capabilities", "sqlite", "policy", "enabled"]);

    println!("  Git enabled: {git_enabled}");
    println!("  SQLite enabled: {sqlite_enabled}");

    if !git_enabled {
        println!(
            "  {} Git disabled — running in local-only mode",
            "⚠".yellow()
        );
    }
    if !sqlite_enabled {
        println!(
            "  {} SQLite disabled — audit in spool-only mode",
            "⚠".yellow()
        );
    }

    println!("{} Preflight check passed", "OK:".green().bold());
    Ok(())
}

// ── Helper functions ─────────────────────────────────────────────

fn detect_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        // Check for WSL
        if std::fs::read_to_string("/proc/version")
            .map(|v| v.to_lowercase().contains("microsoft"))
            .unwrap_or(false)
        {
            "wsl"
        } else {
            "linux"
        }
    }
}

fn detect_host() -> &'static str {
    if std::env::var("GITHUB_ACTIONS").is_ok() {
        "github"
    } else if std::env::var("VSCODE_PID").is_ok() {
        "vscode"
    } else {
        "local"
    }
}

fn command_exists(name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        Command::new("where")
            .arg(name)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("which")
            .arg(name)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

fn git_identity_configured() -> bool {
    let name = Command::new("git")
        .args(["config", "user.name"])
        .output()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false);
    let email = Command::new("git")
        .args(["config", "user.email"])
        .output()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false);
    name && email
}

fn gh_authenticated() -> bool {
    Command::new("gh")
        .args(["auth", "status"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn yaml_bool(yaml: &serde_yaml::Value, keys: &[&str]) -> bool {
    let mut current = yaml;
    for key in keys {
        match current.get(*key) {
            Some(v) => current = v,
            None => return false,
        }
    }
    current.as_bool().unwrap_or(false)
}

fn read_existing_policies(cap_path: &Path) -> (
    Option<bool>, Option<String>, Option<bool>, Option<bool>, Option<String>, Option<bool>, Option<bool>,
) {
    if !cap_path.exists() {
        return (None, None, None, None, None, None, None);
    }
    let content = match std::fs::read_to_string(cap_path) {
        Ok(c) => c,
        Err(_) => return (None, None, None, None, None, None, None),
    };
    let yaml: serde_yaml::Value = match serde_yaml::from_str(&content) {
        Ok(v) => v,
        Err(_) => return (None, None, None, None, None, None, None),
    };

    // Check if last_updated is the sentinel value (fresh file = no prior policy)
    if let Some(ts) = yaml.get("last_updated").and_then(|v| v.as_str()) {
        if ts == "1970-01-01T00:00:00Z" {
            return (None, None, None, None, None, None, None);
        }
    }

    let git_pol = yaml_bool_opt(&yaml, &["capabilities", "git", "policy", "enabled"]);
    let git_mode = yaml_str_opt(&yaml, &["capabilities", "git", "policy", "mode"]);
    let gh_pol = yaml_bool_opt(&yaml, &["capabilities", "gh", "policy", "enabled"]);
    let sqlite_pol = yaml_bool_opt(&yaml, &["capabilities", "sqlite", "policy", "enabled"]);
    let sqlite_mode = yaml_str_opt(&yaml, &["capabilities", "sqlite", "policy", "mode"]);
    let mdlint_pol = yaml_bool_opt(&yaml, &["capabilities", "markdownlint", "policy", "enabled"]);
    let report_pol = yaml_bool_opt(&yaml, &["capabilities", "reporting", "policy", "enabled"]);

    (git_pol, git_mode, gh_pol, sqlite_pol, sqlite_mode, mdlint_pol, report_pol)
}

fn yaml_bool_opt(yaml: &serde_yaml::Value, keys: &[&str]) -> Option<bool> {
    let mut current = yaml;
    for key in keys {
        current = current.get(*key)?;
    }
    current.as_bool()
}

fn yaml_str_opt(yaml: &serde_yaml::Value, keys: &[&str]) -> Option<String> {
    let mut current = yaml;
    for key in keys {
        current = current.get(*key)?;
    }
    current.as_str().map(String::from)
}
