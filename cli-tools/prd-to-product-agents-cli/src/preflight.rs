use anyhow::Result;
use clap::Args;
use colored::Colorize;
use std::path::Path;

use crate::util;

#[derive(Args)]
pub struct DetectArgs {
    /// Target workspace directory
    #[arg(long)]
    pub target: Option<std::path::PathBuf>,
}

#[derive(Args)]
pub struct DepsArgs {
    /// Target workspace directory
    #[arg(long)]
    pub target: Option<std::path::PathBuf>,

    /// Attempt to install missing dependencies via system package managers
    #[arg(long)]
    pub install: bool,

    /// Configure git identity (global or local scope)
    #[arg(long, value_parser = ["global", "local"])]
    pub configure_git_identity: Option<String>,

    /// Git user name (requires --configure-git-identity)
    #[arg(long, requires = "configure_git_identity")]
    pub git_user_name: Option<String>,

    /// Git user email (requires --configure-git-identity)
    #[arg(long, requires = "configure_git_identity")]
    pub git_user_email: Option<String>,

    /// Launch interactive gh auth login
    #[arg(long)]
    pub start_gh_auth: bool,
}

/// Detect environment capabilities and write workspace-capabilities.yaml.
pub fn detect(_skill_root: &Path, args: DetectArgs) -> Result<()> {
    let target = args
        .target
        .as_deref()
        .unwrap_or(Path::new("."));
    let target = target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf());

    println!("{}", "--- Detecting environment capabilities ---".cyan());

    let os = util::detect_os();
    let host = util::detect_host();

    let git_installed = util::command_exists("git");
    let git_identity = if git_installed {
        has_git_identity(&target)
    } else {
        false
    };
    let gh_installed = util::command_exists("gh");
    let gh_authenticated = if gh_installed {
        util::command_ok("gh", &["auth", "status"])
    } else {
        false
    };
    let sqlite_runtime_available = util::sqlite_runtime_available();
    let sqlite_cli_available = util::sqlite_cli_available();
    let db_initialized = target
        .join(".state")
        .join("project_memory.db")
        .exists();
    let node_installed = util::command_exists("node");
    let npm_installed = util::command_exists("npm");
    let markdownlint_installed = util::command_exists("markdownlint");

    println!("  OS:            {os}");
    println!("  Host:          {host}");
    println!(
        "  Git:           {} {}",
        if git_installed { "installed".green() } else { "missing".red() },
        if git_identity { "(identity OK)" } else { "(no identity)" }
    );
    println!(
        "  gh:            {} {}",
        if gh_installed { "installed".green() } else { "missing".yellow() },
        if gh_authenticated { "(authenticated)" } else { "(not authenticated)" }
    );
    println!(
        "  SQLite:        {}",
        if sqlite_runtime_available {
            "embedded runtime available".green()
        } else {
            "runtime unavailable".red()
        }
    );
    println!(
        "  sqlite3 CLI:   {}",
        if sqlite_cli_available {
            "installed (optional)".green()
        } else {
            "missing (optional)".yellow()
        }
    );
    println!(
        "  DB:            {}",
        if db_initialized { "initialized".green() } else { "not initialized".yellow() }
    );
    println!(
        "  Node:          {}",
        if node_installed { "installed".green() } else { "missing (optional)".yellow() }
    );
    println!(
        "  npm:           {}",
        if npm_installed { "installed".green() } else { "missing (optional)".yellow() }
    );
    println!(
        "  markdownlint:  {}",
        if markdownlint_installed { "installed".green() } else { "not installed (optional)".yellow() }
    );

    // Write capabilities YAML
    let caps_path = target
        .join(".github")
        .join("workspace-capabilities.yaml");

    let existing_updated = util::yaml_scalar(&caps_path, "last_updated").ok().flatten();
    let preserve = existing_updated
        .as_deref()
        .map_or(false, |v| !v.is_empty() && v != "1970-01-01T00:00:00Z");

    let git_enabled = if preserve {
        util::yaml_bool(&caps_path, "capabilities.git.policy.enabled", git_installed && git_identity)
    } else {
        git_installed && git_identity
    };
    let gh_enabled = if preserve {
        util::yaml_bool(&caps_path, "capabilities.gh.policy.enabled", gh_installed && gh_authenticated) && git_enabled
    } else {
        gh_installed && gh_authenticated && git_enabled
    };
    let sqlite_enabled = if preserve {
        util::yaml_bool(
            &caps_path,
            "capabilities.sqlite.policy.enabled",
            sqlite_runtime_available,
        )
    } else {
        sqlite_runtime_available
    };
    let mdlint_enabled = if preserve {
        util::yaml_bool(&caps_path, "capabilities.markdownlint.policy.enabled", markdownlint_installed)
    } else {
        markdownlint_installed
    };
    let local_history = if preserve {
        util::yaml_bool(&caps_path, "capabilities.local_history.policy.enabled", true)
    } else {
        true
    };

    let ui_available = target.join("reporting-ui/index.html").exists();
    let xlsx_ready = target.join("reporting-ui/vendor/xlsx.mini.min.js").exists();

    let b = |v: bool| if v { "true" } else { "false" };
    let yaml = format!(
        r#"schema_version: 1
environment:
  host: {host}
  os: {os}
capabilities:
  git:
    detected:
      installed: {git_installed}
      identity_configured: {git_identity}
    policy:
      enabled: {git_enabled}
      mode: {git_mode}
  gh:
    detected:
      installed: {gh_installed}
      authenticated: {gh_auth}
    policy:
      enabled: {gh_enabled}
  sqlite:
    detected:
      installed: {sqlite_installed}
      db_initialized: {db_init}
    policy:
      enabled: {sqlite_enabled}
      mode: {sqlite_mode}
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
      enabled: {mdlint_enabled}
  local_history:
    policy:
      enabled: {local_history}
      format: markdown+json
      path: .state/local-history
  reporting:
    detected:
      ui_available: {ui_available}
      xlsx_export_ready: {xlsx_ready}
      pdf_export_ready: false
    policy:
      enabled: true
      visibility_mode: {vis_mode}
last_updated: {ts}
"#,
        git_installed = b(git_installed),
        git_identity = b(git_identity),
        git_enabled = b(git_enabled),
        git_mode = if git_enabled { "full" } else { "local-only" },
        gh_installed = b(gh_installed),
        gh_auth = b(gh_authenticated),
        gh_enabled = b(gh_enabled),
        sqlite_installed = b(sqlite_runtime_available),
        db_init = b(db_initialized),
        sqlite_enabled = b(sqlite_enabled),
        sqlite_mode = if sqlite_enabled && db_initialized { "ledger" } else { "spool-only" },
        node_installed = b(node_installed),
        npm_installed = b(npm_installed),
        node_native = b(node_installed),
        mdlint_installed = b(markdownlint_installed),
        mdlint_native = b(markdownlint_installed),
        mdlint_enabled = b(mdlint_enabled),
        local_history = b(local_history),
        ui_available = b(ui_available),
        xlsx_ready = b(xlsx_ready),
        vis_mode = if gh_enabled { "auto" } else { "local-only" },
        ts = util::now_utc(),
    );

    util::write_utf8_lf(&caps_path, &yaml)?;
    println!(
        "\n  {} {}",
        "Written:".green(),
        caps_path.display()
    );

    Ok(())
}

/// Quick preflight capability check.
pub fn check(_skill_root: &Path) -> Result<()> {
    println!("{}", "--- Preflight check ---".cyan());

    let mut ok = true;

    // Required
    print!("  git:           ");
    if util::command_exists("git") {
        println!("{}", "OK".green());
    } else {
        println!("{}", "MISSING (required)".red());
        ok = false;
    }

    // Optional but recommended
    print!("  sqlite3:       ");
    if util::command_exists("sqlite3") {
        println!("{}", "OK".green());
    } else {
        println!("{}", "missing (optional — DB init will be deferred)".yellow());
    }

    print!("  gh:            ");
    if util::command_exists("gh") {
        println!("{}", "OK".green());
    } else {
        println!("{}", "missing (optional — GitHub automation disabled)".yellow());
    }

    print!("  markdownlint:  ");
    if util::command_exists("markdownlint") {
        println!("{}", "OK".green());
    } else {
        println!(
            "{}",
            "missing (optional — install: npm install -g markdownlint-cli)"
                .yellow()
        );
    }

    print!("  gitleaks:      ");
    if util::command_exists("gitleaks") {
        println!("{}", "OK".green());
    } else {
        println!("{}", "missing (optional — secrets scanning disabled)".yellow());
    }

    if ok {
        println!("\n  {}", "Preflight passed.".green());
    } else {
        println!("\n  {}", "Preflight has missing required dependencies.".red());
    }

    Ok(())
}

/// Check workspace dependency availability with install hints.
pub fn deps(_skill_root: &Path, args: DepsArgs) -> Result<()> {
    let target = args
        .target
        .as_deref()
        .unwrap_or(Path::new("."));
    let target = target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf());

    println!(
        "{}",
        format!("--- Dependency check for {} ---", target.display()).cyan()
    );

    struct DepInfo {
        name: &'static str,
        required: bool,
        install_hint: &'static str,
    }

    let deps_list = [
        DepInfo {
            name: "git",
            required: true,
            install_hint: "https://git-scm.com/downloads",
        },
        DepInfo {
            name: "gh",
            required: false,
            install_hint: "https://cli.github.com/",
        },
        DepInfo {
            name: "sqlite3",
            required: false,
            install_hint: "winget install SQLite.SQLite / brew install sqlite3 / apt install sqlite3",
        },
        DepInfo {
            name: "markdownlint",
            required: false,
            install_hint: "npm install -g markdownlint-cli",
        },
        DepInfo {
            name: "gitleaks",
            required: false,
            install_hint: "https://github.com/gitleaks/gitleaks#installing",
        },
    ];

    let mut missing_required = false;
    let mut missing_tools: Vec<&str> = Vec::new();

    for dep in &deps_list {
        let available = util::command_exists(dep.name);
        let tag = if dep.required { "required" } else { "optional" };
        if available {
            println!(
                "  {} {} ({})",
                dep.name,
                "available".green(),
                tag
            );
        } else {
            let color_msg = if dep.required {
                missing_required = true;
                format!("{} ({})", "MISSING".red(), tag)
            } else {
                format!("{} ({})", "missing".yellow(), tag)
            };
            println!("  {} {}", dep.name, color_msg);
            if !args.install {
                println!("    Install: {}", dep.install_hint);
            }
            missing_tools.push(dep.name);
        }
    }

    // Auto-install missing dependencies
    if args.install && !missing_tools.is_empty() {
        println!("\n{}", "Attempting to install missing tools...".cyan());
        for tool_name in &missing_tools {
            print!("  Installing {}... ", tool_name);
            if try_install(tool_name) {
                println!("{}", "ok".green());
                if *tool_name == "git" {
                    missing_required = false;
                }
            } else {
                println!("{}", "failed".red());
                if let Some(dep) = deps_list.iter().find(|d| d.name == *tool_name) {
                    println!("    Manual install: {}", dep.install_hint);
                }
            }
        }
    }

    // Configure git identity
    if let Some(ref scope) = args.configure_git_identity {
        println!("\n{}", "Configuring git identity...".cyan());
        let name = args.git_user_name.as_deref().unwrap_or("");
        let email = args.git_user_email.as_deref().unwrap_or("");
        if name.is_empty() || email.is_empty() {
            println!(
                "  {} --git-user-name and --git-user-email are required",
                "ERROR:".red()
            );
        } else if !util::command_exists("git") {
            println!("  {} git is not installed", "ERROR:".red());
        } else {
            let ok = configure_git_identity(&target, scope, name, email);
            if ok {
                println!(
                    "  {} git identity configured ({})",
                    "OK".green(),
                    scope
                );
            } else {
                println!(
                    "  {} failed to configure git identity",
                    "ERROR:".red()
                );
            }
        }
    }

    // Launch gh auth login
    if args.start_gh_auth {
        println!("\n{}", "Launching gh auth login...".cyan());
        if !util::command_exists("gh") {
            println!(
                "  {} gh is not installed — install it first",
                "ERROR:".red()
            );
        } else {
            let status = std::process::Command::new("gh")
                .args(["auth", "login"])
                .status();
            match status {
                Ok(s) if s.success() => {
                    println!("  {} gh auth login completed", "OK".green());
                }
                _ => {
                    println!(
                        "  {} gh auth login did not complete successfully",
                        "WARN:".yellow()
                    );
                }
            }
        }
    }

    println!();

    if missing_required {
        println!(
            "  {}",
            "Some required dependencies are missing.".red()
        );
    } else {
        println!("  {}", "All required dependencies available.".green());
    }

    Ok(())
}

fn configure_git_identity(target: &Path, scope: &str, name: &str, email: &str) -> bool {
    let scope_args: &[&str] = match scope {
        "global" => &["config", "--global"],
        "local" => &["config"],
        _ => return false,
    };

    let mut name_args = scope_args.to_vec();
    name_args.extend_from_slice(&["user.name", name]);

    let mut email_args = scope_args.to_vec();
    email_args.extend_from_slice(&["user.email", email]);

    let name_ok = std::process::Command::new("git")
        .args(&name_args)
        .current_dir(target)

        .output()
        .map(|o| { if !o.status.success() { let err = String::from_utf8_lossy(&o.stderr); if !err.trim().is_empty() { eprintln!("[Preflight Command Failed] {}", err.trim()); } } o.status.success() })
        .unwrap_or(false);

    let email_ok = std::process::Command::new("git")
        .args(&email_args)
        .current_dir(target)

        .output()
        .map(|o| { if !o.status.success() { let err = String::from_utf8_lossy(&o.stderr); if !err.trim().is_empty() { eprintln!("[Preflight Command Failed] {}", err.trim()); } } o.status.success() })
        .unwrap_or(false);

    name_ok && email_ok
}

fn try_install(tool_name: &str) -> bool {
    let os = util::detect_os();
    match tool_name {
        "git" => match os {
            "windows" => win_install("Git.Git", "git", "git"),
            "linux" | "wsl" => apt_install(&["git"]),
            "macos" => brew_install(&["git"]),
            _ => false,
        },
        "gh" => match os {
            "windows" => win_install("GitHub.cli", "gh", "gh"),
            "linux" | "wsl" => apt_install(&["gh"]),
            "macos" => brew_install(&["gh"]),
            _ => false,
        },
        "sqlite3" => match os {
            "windows" => win_install("SQLite.SQLite", "sqlite", "sqlite"),
            "linux" | "wsl" => apt_install(&["sqlite3"]),
            "macos" => brew_install(&["sqlite"]),
            _ => false,
        },
        "markdownlint" => {
            if util::command_exists("npm") {
                std::process::Command::new("npm")
                    .args(["install", "-g", "markdownlint-cli"])

                    .output()
                    .map(|o| { if !o.status.success() { let err = String::from_utf8_lossy(&o.stderr); if !err.trim().is_empty() { eprintln!("[Preflight Command Failed] {}", err.trim()); } } o.status.success() })
                    .unwrap_or(false)
            } else {
                false
            }
        }
        "gitleaks" => match os {
            "macos" => brew_install(&["gitleaks"]),
            _ => false,
        },
        _ => false,
    }
}

fn win_install(winget_id: &str, choco_pkg: &str, scoop_pkg: &str) -> bool {
    if util::command_exists("winget") {
        let ok = std::process::Command::new("winget")
            .args([
                "install", "--id", winget_id, "-e", "--silent",
                "--accept-package-agreements", "--accept-source-agreements",
            ])

            .output()
            .map(|o| { if !o.status.success() { let err = String::from_utf8_lossy(&o.stderr); if !err.trim().is_empty() { eprintln!("[Preflight Command Failed] {}", err.trim()); } } o.status.success() })
            .unwrap_or(false);
        if ok {
            return true;
        }
    }
    if util::command_exists("choco") {
        let ok = std::process::Command::new("choco")
            .args(["install", choco_pkg, "-y"])

            .output()
            .map(|o| { if !o.status.success() { let err = String::from_utf8_lossy(&o.stderr); if !err.trim().is_empty() { eprintln!("[Preflight Command Failed] {}", err.trim()); } } o.status.success() })
            .unwrap_or(false);
        if ok {
            return true;
        }
    }
    if util::command_exists("scoop") {
        let ok = std::process::Command::new("scoop")
            .args(["install", scoop_pkg])

            .output()
            .map(|o| { if !o.status.success() { let err = String::from_utf8_lossy(&o.stderr); if !err.trim().is_empty() { eprintln!("[Preflight Command Failed] {}", err.trim()); } } o.status.success() })
            .unwrap_or(false);
        if ok {
            return true;
        }
    }
    false
}

fn apt_install(packages: &[&str]) -> bool {
    let mut args = vec!["apt-get", "install", "-y"];
    args.extend_from_slice(packages);
    std::process::Command::new("sudo")
        .args(&args)

        .output()
        .map(|o| { if !o.status.success() { let err = String::from_utf8_lossy(&o.stderr); if !err.trim().is_empty() { eprintln!("[Preflight Command Failed] {}", err.trim()); } } o.status.success() })
        .unwrap_or(false)
}

fn brew_install(packages: &[&str]) -> bool {
    let mut args = vec!["install"];
    args.extend(packages.iter().copied());
    std::process::Command::new("brew")
        .args(&args)

        .output()
        .map(|o| { if !o.status.success() { let err = String::from_utf8_lossy(&o.stderr); if !err.trim().is_empty() { eprintln!("[Preflight Command Failed] {}", err.trim()); } } o.status.success() })
        .unwrap_or(false)
}

fn has_git_identity(target: &Path) -> bool {
    let name = std::process::Command::new("git")
        .args(["config", "--get", "user.name"])
        .current_dir(target)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);
    let email = std::process::Command::new("git")
        .args(["config", "--get", "user.email"])
        .current_dir(target)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);
    name && email
}
