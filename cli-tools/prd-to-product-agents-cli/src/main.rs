mod bootstrap;
mod validate;
mod clean;
mod preflight;
mod logging;
mod util;

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(
    name = "prd-to-product-agents-cli",
    version,
    about = "prd-to-product-agents skill CLI — bootstrap, validate, clean, preflight"
)]
struct Cli {
    /// Skill root path (required)
    #[arg(long, global = true)]
    skill_root: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Bootstrap a new workspace from templates
    Bootstrap {
        #[command(subcommand)]
        sub: BootstrapCommands,
    },
    /// Validate skill artifacts and generated workspaces
    Validate {
        #[command(subcommand)]
        sub: ValidateCommands,
    },
    /// Clean generated workspace artifacts
    Clean {
        #[command(subcommand)]
        sub: CleanCommands,
    },
    /// Preflight checks and capability detection
    Preflight {
        #[command(subcommand)]
        sub: PreflightCommands,
    },
}

// ── Bootstrap sub-commands ───────────────────────────────────────
#[derive(Subcommand)]
enum BootstrapCommands {
    /// Create a new workspace from templates
    Workspace(bootstrap::WorkspaceArgs),
    /// Safe git commit of manifest-listed files
    Commit(bootstrap::CommitArgs),
}

// ── Validate sub-commands ────────────────────────────────────────
#[derive(Subcommand)]
enum ValidateCommands {
    /// Run all skill-side validation checks
    All,
    /// Validate generated workspace structure
    Generated(validate::GeneratedArgs),
    /// Check that packaged skill contains no runtime artifacts
    PackageHygiene,
    /// Validate platform compatibility claims
    PlatformClaims,
    /// Validate package VERSION metadata
    VersionMetadata,
}

// ── Clean sub-commands ───────────────────────────────────────────
#[derive(Subcommand)]
enum CleanCommands {
    /// Remove bootstrap-deployed artifacts per manifest
    Workspace(clean::WorkspaceArgs),
}

// ── Preflight sub-commands ───────────────────────────────────────
#[derive(Subcommand)]
enum PreflightCommands {
    /// Detect environment capabilities and write workspace-capabilities.yaml
    Detect(preflight::DetectArgs),
    /// Quick preflight capability check
    Check,
    /// Check workspace dependency availability
    Deps(preflight::DepsArgs),
}

fn command_name(command: &Commands) -> &'static str {
    match command {
        Commands::Bootstrap { .. } => "bootstrap",
        Commands::Validate { .. } => "validate",
        Commands::Clean { .. } => "clean",
        Commands::Preflight { .. } => "preflight",
    }
}

fn execute(command: Commands, skill_root: &std::path::Path) -> anyhow::Result<()> {
    match command {
        Commands::Bootstrap { sub } => match sub {
            BootstrapCommands::Workspace(args) => bootstrap::workspace(skill_root, args),
            BootstrapCommands::Commit(args) => bootstrap::commit(skill_root, args),
        },
        Commands::Validate { sub } => match sub {
            ValidateCommands::All => validate::all(skill_root),
            ValidateCommands::Generated(args) => validate::generated(skill_root, args),
            ValidateCommands::PackageHygiene => validate::package_hygiene(skill_root),
            ValidateCommands::PlatformClaims => validate::platform_claims(skill_root),
            ValidateCommands::VersionMetadata => validate::version_metadata(skill_root),
        },
        Commands::Clean { sub } => match sub {
            CleanCommands::Workspace(args) => clean::workspace(skill_root, args),
        },
        Commands::Preflight { sub } => match sub {
            PreflightCommands::Detect(args) => preflight::detect(skill_root, args),
            PreflightCommands::Check => preflight::check(skill_root),
            PreflightCommands::Deps(args) => preflight::deps(skill_root, args),
        },
    }
}

fn main() {
    let cli = Cli::parse();
    let command = cli.command;
    
    let raw_skill_root = cli.skill_root.unwrap_or_else(|| {
        eprintln!("{} --skill-root must be explicitly provided", "ERROR:".red().bold());
        process::exit(1);
    });
    
    let skill_root = util::resolve_skill_root(&raw_skill_root);
    let command_label = command_name(&command);
    let use_temp_log_dir = matches!(&command, Commands::Preflight { .. });

    let log_guard = match logging::init(&skill_root, use_temp_log_dir) {
        Ok(guard) => Some(guard),
        Err(error) => {
            eprintln!("{} tracing unavailable: {error:#}", "WARN:".yellow().bold());
            None
        }
    };

    tracing::info!(skill_root = %skill_root.display(), command = command_label, "dispatching skill CLI command");

    let result = execute(command, &skill_root);

    let exit_code = match result {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!(error = ?e, "command failed");
            eprintln!("{} {e:#}", "ERROR:".red().bold());
            1
        }
    };

    drop(log_guard);

    if exit_code != 0 {
        process::exit(exit_code);
    }
}
