use anyhow::Result;
use clap::Args;
use colored::Colorize;
use std::process::Command;

#[derive(Args)]
pub struct CheckArgs {
    /// Attempt to install missing dependencies via system package managers
    #[arg(long)]
    pub install: bool,
}

struct ToolSpec {
    name: &'static str,
    check_cmd: &'static str,
    check_args: &'static [&'static str],
    required: bool,
    install_hint: &'static str,
}

const TOOLS: &[ToolSpec] = &[
    ToolSpec {
        name: "git",
        check_cmd: "git",
        check_args: &["--version"],
        required: true,
        install_hint: "https://git-scm.com/downloads",
    },
    ToolSpec {
        name: "gh",
        check_cmd: "gh",
        check_args: &["--version"],
        required: false,
        install_hint: "https://cli.github.com/",
    },
    ToolSpec {
        name: "sqlite3",
        check_cmd: "sqlite3",
        check_args: &["--version"],
        required: false,
        install_hint: "bundled via rusqlite — standalone CLI optional",
    },
    ToolSpec {
        name: "markdownlint",
        check_cmd: "markdownlint",
        check_args: &["--version"],
        required: false,
        install_hint: "npm install -g markdownlint-cli (optional, prdtp-agents-functions-cli validates internally)",
    },
    ToolSpec {
        name: "gitleaks",
        check_cmd: "gitleaks",
        check_args: &["version"],
        required: false,
        install_hint: "https://github.com/gitleaks/gitleaks#installing",
    },
];

/// Detect OS family for package manager selection.
enum OsFamily {
    Windows,
    Linux,
    MacOs,
}

fn detect_os() -> OsFamily {
    if cfg!(target_os = "windows") {
        OsFamily::Windows
    } else if cfg!(target_os = "macos") {
        OsFamily::MacOs
    } else {
        OsFamily::Linux
    }
}

fn cmd_exists(name: &str) -> bool {
    let status = Command::new(name)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    status
        .map(|s| {
            if !s.success() {
                tracing::debug!(command = %name, exit_code = s.code(), "dependency probe returned non-success status");
            }
            s.success()
        })
        .unwrap_or(false)
}

/// Try to install a tool using the first available package manager.
fn try_install(tool_name: &str) -> bool {
    let os = detect_os();
    match tool_name {
        "git" => match os {
            OsFamily::Windows => win_install("Git.Git", "git", "git"),
            OsFamily::Linux => apt_install(&["git"]),
            OsFamily::MacOs => brew_install(&["git"]),
        },
        "gh" => match os {
            OsFamily::Windows => win_install("GitHub.cli", "gh", "gh"),
            OsFamily::Linux => apt_install(&["gh"]),
            OsFamily::MacOs => brew_install(&["gh"]),
        },
        "sqlite3" => match os {
            OsFamily::Windows => win_install("SQLite.SQLite", "sqlite", "sqlite"),
            OsFamily::Linux => apt_install(&["sqlite3"]),
            OsFamily::MacOs => brew_install(&["sqlite"]),
        },
        "markdownlint" => {
            if cmd_exists("npm") {
                Command::new("npm")
                    .args(["install", "-g", "markdownlint-cli"])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            } else {
                false
            }
        }
        "gitleaks" => match os {
            OsFamily::MacOs => brew_install(&["gitleaks"]),
            _ => false, // no standard package manager install on Windows/Linux
        },
        _ => false,
    }
}

fn win_install(winget_id: &str, choco_pkg: &str, scoop_pkg: &str) -> bool {
    if cmd_exists("winget") {
        let ok = Command::new("winget")
            .args([
                "install", "--id", winget_id, "-e", "--silent",
                "--accept-package-agreements", "--accept-source-agreements",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok { return true; }
    }
    if cmd_exists("choco") {
        let ok = Command::new("choco")
            .args(["install", choco_pkg, "-y"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok { return true; }
    }
    if cmd_exists("scoop") {
        let ok = Command::new("scoop")
            .args(["install", scoop_pkg])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok { return true; }
    }
    false
}

fn apt_install(packages: &[&str]) -> bool {
    // Try apt-get with sudo
    let mut args = vec!["apt-get", "install", "-y"];
    args.extend_from_slice(packages);
    Command::new("sudo")
        .args(&args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn brew_install(packages: &[&str]) -> bool {
    let mut args = vec!["install"];
    args.extend(packages.iter().copied());
    Command::new("brew")
        .args(&args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn run(_workspace: &std::path::Path, args: CheckArgs) -> Result<()> {
    tracing::info!(install = args.install, "checking runtime cli dependencies");
    println!("{}", "Dependency check".bold());
    println!();

    let mut missing_required = Vec::new();
    let mut missing_optional = Vec::new();
    let mut missing_all: Vec<&str> = Vec::new();

    for tool in TOOLS {
        let status = Command::new(tool.check_cmd)
            .args(tool.check_args)
            .output();

        match status {
            Ok(output) if output.status.success() => {
                let ver = String::from_utf8_lossy(&output.stdout);
                let ver_line = ver.lines().next().unwrap_or("").trim();
                tracing::info!(tool = %tool.name, version = %ver_line, "dependency available");
                println!(
                    "  {} {} — {}",
                    "✓".green().bold(),
                    tool.name,
                    ver_line
                );
            }
            _ => {
                let tag = if tool.required { "REQUIRED" } else { "optional" };
                tracing::warn!(tool = %tool.name, required = tool.required, "dependency missing");
                println!(
                    "  {} {} — not found ({})",
                    "✗".red().bold(),
                    tool.name,
                    tag
                );
                if !args.install {
                    println!("    Install: {}", tool.install_hint);
                }
                if tool.required {
                    missing_required.push(tool.name);
                } else {
                    missing_optional.push(tool.name);
                }
                missing_all.push(tool.name);
            }
        }
    }

    // Attempt installation if --install was passed and there are missing tools
    if args.install && !missing_all.is_empty() {
        println!();
        println!("{}", "Attempting to install missing tools…".bold());
        println!();

        for tool_name in &missing_all {
            print!("  Installing {}… ", tool_name);
            if try_install(tool_name) {
                tracing::info!(tool = %tool_name, "dependency installed automatically");
                println!("{}", "ok".green().bold());
                // Remove from missing lists on success
                missing_required.retain(|n| n != tool_name);
                missing_optional.retain(|n| n != tool_name);
            } else {
                tracing::warn!(tool = %tool_name, "automatic dependency installation failed");
                println!("{}", "failed".red().bold());
                // Find the hint for manual install
                if let Some(spec) = TOOLS.iter().find(|t| t.name == *tool_name) {
                    println!("    Manual install: {}", spec.install_hint);
                }
            }
        }
    }

    println!();

    if !missing_optional.is_empty() {
        tracing::warn!(tools = %missing_optional.join(", "), "optional dependencies missing");
        println!(
            "{} Optional tools missing: {}",
            "INFO:".cyan(),
            missing_optional.join(", ")
        );
    }

    if !missing_required.is_empty() {
        tracing::error!(tools = %missing_required.join(", "), "required dependencies missing");
        println!(
            "{} Required tools missing: {}",
            "ERROR:".red().bold(),
            missing_required.join(", ")
        );
        std::process::exit(1);
    }

    tracing::info!("all required runtime dependencies present");
    println!("{} All required dependencies present.", "✓".green().bold());
    Ok(())
}
