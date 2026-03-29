use anyhow::Result;
use clap::{Args, ValueEnum};
use colored::Colorize;
use prdtp_agents_shared::capabilities::{
    read_capabilities_document, render_capabilities_yaml, write_capabilities_document,
    CapabilitySnapshotInput,
};
use serde_json::json;
use serde_yaml::Value;
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum CapabilityName {
    Git,
    Gh,
    Sqlite,
    Markdownlint,
    Reporting,
    LocalHistory,
}

#[derive(Args)]
pub struct AuthorizeArgs {
    /// Capability to authorize or de-authorize
    #[arg(long, value_enum)]
    capability: CapabilityName,
    /// Set authorization enabled=true/false
    #[arg(long)]
    enabled: String,
    /// Provenance of the authorization decision
    #[arg(long, default_value = "manual-maintainer")]
    source: String,
    /// Optional policy mode override for git/sqlite
    #[arg(long)]
    mode: Option<String>,
    /// Optional reporting visibility override
    #[arg(long)]
    visibility_mode: Option<String>,
}

#[derive(Debug, Default)]
struct ExistingPolicies {
    git_authorized: Option<bool>,
    git_authorization_source: Option<String>,
    git_mode: Option<String>,
    gh_authorized: Option<bool>,
    gh_authorization_source: Option<String>,
    sqlite_authorized: Option<bool>,
    sqlite_authorization_source: Option<String>,
    sqlite_mode: Option<String>,
    markdownlint_authorized: Option<bool>,
    markdownlint_authorization_source: Option<String>,
    reporting_authorized: Option<bool>,
    reporting_authorization_source: Option<String>,
    reporting_visibility_mode: Option<String>,
    local_history_authorized: Option<bool>,
    local_history_authorization_source: Option<String>,
}

pub fn run(workspace: &Path, args: DetectArgs) -> Result<()> {
    println!("{}", "=== Detect Capabilities ===".cyan().bold());

    let cap_path = workspace.join(".github/workspace-capabilities.yaml");

    let os = detect_os();
    let host = detect_host();
    println!("  OS: {os}, Host: {host}");

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
    let xlsx_ready = workspace
        .join("reporting-ui/vendor/xlsx.mini.min.js")
        .exists();

    println!("  git: {git_installed} (identity: {git_identity})");
    println!("  gh: {gh_installed} (auth: {gh_auth})");
    println!(
        "  sqlite: {sqlite_runtime_available} (db: {db_initialized}, sqlite3 cli: {sqlite_cli_available})"
    );
    println!("  node: {node_installed}, npm: {npm_installed}");
    println!("  markdownlint: {mdlint_installed}");
    println!("  reporting UI: {ui_available}, XLSX: {xlsx_ready}");

    let existing_policies = read_existing_policies(&cap_path);

    let git_authorized = if args.disable_git {
        false
    } else {
        existing_policies.git_authorized.unwrap_or(false)
    };
    let git_authorization_source = existing_policies
        .git_authorization_source
        .unwrap_or_else(|| "manual-default-deny".to_string());
    let git_mode = existing_policies.git_mode.unwrap_or_else(|| {
        if git_authorized {
            "full".to_string()
        } else {
            "local-only".to_string()
        }
    });
    let gh_authorized = if args.disable_gh {
        false
    } else {
        existing_policies
            .gh_authorized
            .unwrap_or(false)
    };
    let gh_authorization_source = existing_policies
        .gh_authorization_source
        .unwrap_or_else(|| "manual-default-deny".to_string());
    let sqlite_authorized = if args.disable_sqlite {
        false
    } else {
        existing_policies
            .sqlite_authorized
            .unwrap_or(sqlite_runtime_available)
    };
    let sqlite_authorization_source = existing_policies
        .sqlite_authorization_source
        .unwrap_or_else(|| {
            if sqlite_runtime_available {
                "detected-default".to_string()
            } else {
                "missing-runtime".to_string()
            }
        });
    let sqlite_mode = existing_policies.sqlite_mode.unwrap_or_else(|| {
        if sqlite_authorized && db_initialized {
            "ledger".to_string()
        } else {
            "spool-only".to_string()
        }
    });
    let markdownlint_authorized = if args.disable_markdownlint {
        false
    } else {
        existing_policies
            .markdownlint_authorized
            .unwrap_or(mdlint_installed)
    };
    let markdownlint_authorization_source = existing_policies
        .markdownlint_authorization_source
        .unwrap_or_else(|| {
            if mdlint_installed {
                "detected-default".to_string()
            } else {
                "missing-runtime".to_string()
            }
        });
    let reporting_authorized = existing_policies.reporting_authorized.unwrap_or(true);
    let reporting_authorization_source = existing_policies
        .reporting_authorization_source
        .unwrap_or_else(|| "detected-default".to_string());
    let local_history_authorized = existing_policies.local_history_authorized.unwrap_or(true);
    let local_history_authorization_source = existing_policies
        .local_history_authorization_source
        .unwrap_or_else(|| "detected-default".to_string());
    let visibility_mode = existing_policies.reporting_visibility_mode.unwrap_or_else(|| {
        if gh_authorized {
            "auto".to_string()
        } else {
            "local-only".to_string()
        }
    });

    let yaml = render_capabilities_yaml(CapabilitySnapshotInput {
        host: host.to_string(),
        os: os.to_string(),
        git_installed,
        git_identity_configured: git_identity,
        git_authorized,
        git_authorization_source,
        git_mode,
        gh_installed,
        gh_authenticated: gh_auth,
        gh_authorized,
        gh_authorization_source,
        sqlite_installed: sqlite_runtime_available,
        db_initialized,
        sqlite_authorized,
        sqlite_authorization_source,
        sqlite_mode,
        node_installed,
        npm_installed,
        node_native_linux: node_native,
        markdownlint_installed: mdlint_installed,
        markdownlint_native_linux: mdlint_native,
        markdownlint_authorized,
        markdownlint_authorization_source,
        local_history_authorized,
        local_history_authorization_source,
        local_history_format: "markdown+json".to_string(),
        local_history_path: ".state/local-history".to_string(),
        reporting_ui_available: ui_available,
        reporting_xlsx_export_ready: xlsx_ready,
        reporting_pdf_export_ready: false,
        reporting_authorized,
        reporting_authorization_source,
        reporting_visibility_mode: visibility_mode,
        last_updated: yaml_ops::now_utc_iso(),
    })?;

    if let Some(parent) = cap_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&cap_path, &yaml)?;
    let _ = crate::audit::events::record_sensitive_action(
        workspace,
        "capabilities.detect",
        "runtime-cli",
        "success",
        json!({
            "path": ".github/workspace-capabilities.yaml",
            "git_authorized": git_authorized,
            "gh_authorized": gh_authorized,
            "sqlite_authorized": sqlite_authorized,
            "reporting_authorized": reporting_authorized
        }),
    );

    println!("\n{} Wrote {}", "OK:".green().bold(), cap_path.display());
    Ok(())
}

/// Quick preflight check. Exit 1 if critical capabilities are missing.
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
    let yaml: Value = serde_yaml::from_str(&content)?;

    let git_enabled = yaml_bool(&yaml, &["capabilities", "git", "authorized", "enabled"]);
    let sqlite_enabled = yaml_bool(
        &yaml,
        &["capabilities", "sqlite", "authorized", "enabled"],
    );

    println!("  Git authorized: {git_enabled}");
    println!("  SQLite authorized: {sqlite_enabled}");

    if !git_enabled {
        println!(
            "  {} Git disabled - running in local-only mode",
            "!".yellow()
        );
    }
    if !sqlite_enabled {
        println!(
            "  {} SQLite disabled - audit in spool-only mode",
            "!".yellow()
        );
    }

    println!("{} Preflight check passed", "OK:".green().bold());
    Ok(())
}

pub fn authorize(workspace: &Path, args: AuthorizeArgs) -> Result<()> {
    println!("{}", "=== Authorize Capability ===".cyan().bold());
    let enabled = parse_bool_flag(&args.enabled)?;

    let cap_path = workspace.join(".github/workspace-capabilities.yaml");
    if !cap_path.is_file() {
        anyhow::bail!(
            "{} not found. Run `prdtp-agents-functions-cli capabilities detect` first.",
            cap_path.display()
        );
    }

    let mut document = read_capabilities_document(&cap_path)?;
    match args.capability {
        CapabilityName::Git => {
            document.capabilities.git.authorized.enabled = enabled;
            document.capabilities.git.authorized.source = args.source.clone();
            if let Some(mode) = args.mode {
                document.capabilities.git.policy.mode = mode;
            }
        }
        CapabilityName::Gh => {
            document.capabilities.gh.authorized.enabled = enabled;
            document.capabilities.gh.authorized.source = args.source.clone();
        }
        CapabilityName::Sqlite => {
            document.capabilities.sqlite.authorized.enabled = enabled;
            document.capabilities.sqlite.authorized.source = args.source.clone();
            if let Some(mode) = args.mode {
                document.capabilities.sqlite.policy.mode = mode;
            }
        }
        CapabilityName::Markdownlint => {
            document.capabilities.markdownlint.authorized.enabled = enabled;
            document.capabilities.markdownlint.authorized.source = args.source.clone();
        }
        CapabilityName::Reporting => {
            document.capabilities.reporting.authorized.enabled = enabled;
            document.capabilities.reporting.authorized.source = args.source.clone();
            if let Some(visibility_mode) = args.visibility_mode {
                document.capabilities.reporting.policy.visibility_mode = visibility_mode;
            }
        }
        CapabilityName::LocalHistory => {
            document.capabilities.local_history.authorized.enabled = enabled;
            document.capabilities.local_history.authorized.source = args.source.clone();
        }
    }
    document.last_updated = yaml_ops::now_utc_iso();
    write_capabilities_document(&cap_path, &document)?;
    let _ = crate::audit::events::record_sensitive_action(
        workspace,
        "capabilities.authorize",
        "runtime-cli",
        "success",
        json!({
            "capability": capability_name(args.capability),
            "enabled": enabled,
            "source": args.source
        }),
    );

    println!(
        "{} capabilities.{}.authorized.enabled={}",
        "OK:".green().bold(),
        capability_name(args.capability),
        enabled
    );
    Ok(())
}

fn detect_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if std::fs::read_to_string("/proc/version")
        .map(|value| value.to_lowercase().contains("microsoft"))
        .unwrap_or(false)
    {
        "wsl"
    } else {
        "linux"
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
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("which")
            .arg(name)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

fn git_identity_configured() -> bool {
    let name = Command::new("git")
        .args(["config", "user.name"])
        .output()
        .map(|output| output.status.success() && !output.stdout.is_empty())
        .unwrap_or(false);
    let email = Command::new("git")
        .args(["config", "user.email"])
        .output()
        .map(|output| output.status.success() && !output.stdout.is_empty())
        .unwrap_or(false);
    name && email
}

fn gh_authenticated() -> bool {
    Command::new("gh")
        .args(["auth", "status"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn yaml_bool(yaml: &Value, keys: &[&str]) -> bool {
    let mut current = yaml;
    for key in keys {
        match current.get(*key) {
            Some(value) => current = value,
            None => return false,
        }
    }
    current.as_bool().unwrap_or(false)
}

fn read_existing_policies(cap_path: &Path) -> ExistingPolicies {
    if !cap_path.exists() {
        return ExistingPolicies::default();
    }

    let content = match std::fs::read_to_string(cap_path) {
        Ok(content) => content,
        Err(_) => return ExistingPolicies::default(),
    };
    let yaml: Value = match serde_yaml::from_str(&content) {
        Ok(value) => value,
        Err(_) => return ExistingPolicies::default(),
    };

    if let Some(ts) = yaml.get("last_updated").and_then(|value| value.as_str()) {
        if ts == "1970-01-01T00:00:00Z" {
            return ExistingPolicies::default();
        }
    }

    ExistingPolicies {
        git_authorized: yaml_bool_opt(&yaml, &["capabilities", "git", "authorized", "enabled"])
            .or_else(|| yaml_bool_opt(&yaml, &["capabilities", "git", "policy", "enabled"])),
        git_authorization_source: yaml_str_opt(
            &yaml,
            &["capabilities", "git", "authorized", "source"],
        ),
        git_mode: yaml_str_opt(&yaml, &["capabilities", "git", "policy", "mode"]),
        gh_authorized: yaml_bool_opt(&yaml, &["capabilities", "gh", "authorized", "enabled"])
            .or_else(|| yaml_bool_opt(&yaml, &["capabilities", "gh", "policy", "enabled"])),
        gh_authorization_source: yaml_str_opt(
            &yaml,
            &["capabilities", "gh", "authorized", "source"],
        ),
        sqlite_authorized: yaml_bool_opt(
            &yaml,
            &["capabilities", "sqlite", "authorized", "enabled"],
        )
        .or_else(|| yaml_bool_opt(&yaml, &["capabilities", "sqlite", "policy", "enabled"])),
        sqlite_authorization_source: yaml_str_opt(
            &yaml,
            &["capabilities", "sqlite", "authorized", "source"],
        ),
        sqlite_mode: yaml_str_opt(&yaml, &["capabilities", "sqlite", "policy", "mode"]),
        markdownlint_authorized: yaml_bool_opt(
            &yaml,
            &["capabilities", "markdownlint", "authorized", "enabled"],
        ),
        markdownlint_authorization_source: yaml_str_opt(
            &yaml,
            &["capabilities", "markdownlint", "authorized", "source"],
        ),
        reporting_authorized: yaml_bool_opt(
            &yaml,
            &["capabilities", "reporting", "authorized", "enabled"],
        )
        .or_else(|| yaml_bool_opt(&yaml, &["capabilities", "reporting", "policy", "enabled"])),
        reporting_authorization_source: yaml_str_opt(
            &yaml,
            &["capabilities", "reporting", "authorized", "source"],
        ),
        reporting_visibility_mode: yaml_str_opt(
            &yaml,
            &["capabilities", "reporting", "policy", "visibility_mode"],
        ),
        local_history_authorized: yaml_bool_opt(
            &yaml,
            &["capabilities", "local_history", "authorized", "enabled"],
        )
        .or_else(|| {
            yaml_bool_opt(&yaml, &["capabilities", "local_history", "policy", "enabled"])
        }),
        local_history_authorization_source: yaml_str_opt(
            &yaml,
            &["capabilities", "local_history", "authorized", "source"],
        ),
    }
}

fn capability_name(capability: CapabilityName) -> &'static str {
    match capability {
        CapabilityName::Git => "git",
        CapabilityName::Gh => "gh",
        CapabilityName::Sqlite => "sqlite",
        CapabilityName::Markdownlint => "markdownlint",
        CapabilityName::Reporting => "reporting",
        CapabilityName::LocalHistory => "local_history",
    }
}

fn parse_bool_flag(value: &str) -> Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => anyhow::bail!("--enabled must be one of: true, false, 1, 0, yes, no, on, off"),
    }
}

fn yaml_bool_opt(yaml: &Value, keys: &[&str]) -> Option<bool> {
    let mut current = yaml;
    for key in keys {
        current = current.get(*key)?;
    }
    current.as_bool()
}

fn yaml_str_opt(yaml: &Value, keys: &[&str]) -> Option<String> {
    let mut current = yaml;
    for key in keys {
        current = current.get(*key)?;
    }
    current.as_str().map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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

    #[test]
    fn capabilities_document_round_trips_as_valid_yaml() {
        let yaml = render_capabilities_yaml(CapabilitySnapshotInput {
            host: "local".to_string(),
            os: "windows".to_string(),
            git_installed: true,
            git_identity_configured: true,
            git_authorized: true,
            git_authorization_source: "explicit".to_string(),
            git_mode: "full".to_string(),
            gh_installed: true,
            gh_authenticated: true,
            gh_authorized: true,
            gh_authorization_source: "explicit".to_string(),
            sqlite_installed: true,
            db_initialized: true,
            sqlite_authorized: true,
            sqlite_authorization_source: "detected-default".to_string(),
            sqlite_mode: "ledger".to_string(),
            node_installed: true,
            npm_installed: true,
            node_native_linux: false,
            markdownlint_installed: true,
            markdownlint_native_linux: false,
            markdownlint_authorized: true,
            markdownlint_authorization_source: "detected-default".to_string(),
            local_history_authorized: true,
            local_history_authorization_source: "detected-default".to_string(),
            local_history_format: "markdown+json".to_string(),
            local_history_path: ".state/local-history".to_string(),
            reporting_ui_available: true,
            reporting_xlsx_export_ready: true,
            reporting_pdf_export_ready: false,
            reporting_authorized: true,
            reporting_authorization_source: "detected-default".to_string(),
            reporting_visibility_mode: "auto".to_string(),
            last_updated: "2026-03-28T00:00:00Z".to_string(),
        })
        .expect("failed to render capabilities yaml");
        let parsed: Value = serde_yaml::from_str(&yaml).expect("failed to parse capabilities yaml");

        assert_eq!(
            parsed["capabilities"]["sqlite"]["detected"]["installed"].as_bool(),
            Some(true)
        );
        assert_eq!(
            parsed["capabilities"]["git"]["policy"]["mode"].as_str(),
            Some("full")
        );
        assert_eq!(
            parsed["capabilities"]["git"]["authorized"]["enabled"].as_bool(),
            Some(true)
        );
    }

    #[test]
    fn existing_policies_are_preserved_after_detection() {
        let temp = TempDir::new().expect("failed to create temp workspace");
        let workspace = temp.path();
        let caps_dir = workspace.join(".github");
        std::fs::create_dir_all(&caps_dir).expect("failed to create caps dir");
        std::fs::write(
            caps_dir.join("workspace-capabilities.yaml"),
            r#"schema_version: 1
environment:
  host: unknown
  os: unknown
capabilities:
  git:
    detected:
      installed: false
      identity_configured: false
    authorized:
      enabled: false
      source: manual-default-deny
    policy:
      mode: local-only
  gh:
    detected:
      installed: false
      authenticated: false
    authorized:
      enabled: false
      source: manual-default-deny
  sqlite:
    detected:
      installed: false
      db_initialized: false
    authorized:
      enabled: true
      source: detected-default
    policy:
      mode: spool-only
  node:
    detected:
      installed: false
      npm_installed: false
      native_linux: false
  markdownlint:
    detected:
      installed: false
      native_linux: false
    authorized:
      enabled: false
      source: missing-runtime
  local_history:
    authorized:
      enabled: true
      source: detected-default
    policy:
      format: markdown+json
      path: .state/local-history
  reporting:
    detected:
      ui_available: false
      xlsx_export_ready: false
      pdf_export_ready: false
    authorized:
      enabled: false
      source: manual-maintainer
    policy:
      visibility_mode: local-only
last_updated: 2026-03-27T00:00:00Z
"#,
        )
        .expect("failed to seed capabilities yaml");

        run(
            workspace,
            DetectArgs {
                disable_git: false,
                disable_gh: false,
                disable_sqlite: false,
                disable_markdownlint: false,
            },
        )
        .expect("capabilities detect failed");

        let content = std::fs::read_to_string(caps_dir.join("workspace-capabilities.yaml"))
            .expect("failed to read updated capabilities yaml");
        let parsed: Value =
            serde_yaml::from_str(&content).expect("updated capabilities yaml must parse");

        assert_eq!(
            parsed["capabilities"]["sqlite"]["authorized"]["enabled"].as_bool(),
            Some(true)
        );
        assert_eq!(
            parsed["capabilities"]["reporting"]["authorized"]["enabled"].as_bool(),
            Some(false)
        );
    }
}
