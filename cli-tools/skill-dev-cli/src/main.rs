mod test_cmd;
mod util;

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(
    name = "skill-dev-cli",
    version,
    about = "Project-scope CLI for prd-to-product-agents skill development"
)]
struct Cli {
    /// Skill root directory
    #[arg(long, global = true)]
    skill_root: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run project-scope development checks for the skill
    Test {
        #[command(subcommand)]
        sub: TestCommands,
    },
}

#[derive(Subcommand)]
enum TestCommands {
    /// Run smoke tests for bootstrap and optional runtime integration
    Smoke(test_cmd::SmokeArgs),
    /// Run unit tests for core helpers
    Unit,
    /// Run markdownlint-cli against the skill markdown files
    Markdown(test_cmd::MarkdownArgs),
    /// Run the same repository validation chain used by GitHub validation workflow
    RepoValidation(test_cmd::RepoValidationArgs),
    /// Run full release-blocking validation chain (fails on first error)
    ReleaseGate(test_cmd::ReleaseGateArgs),
}

fn main() {
    let cli = Cli::parse();

    let raw_skill_root = cli.skill_root.unwrap_or_else(|| {
        eprintln!("{} --skill-root must be explicitly provided", "ERROR:".red().bold());
        process::exit(1);
    });

    let skill_root = util::resolve_skill_root(&raw_skill_root);

    let result = match cli.command {
        Commands::Test { sub } => match sub {
            TestCommands::Smoke(args) => test_cmd::smoke(&skill_root, args),
            TestCommands::Unit => test_cmd::unit(&skill_root),
            TestCommands::Markdown(args) => test_cmd::markdown(&skill_root, args),
            TestCommands::RepoValidation(args) => test_cmd::repo_validation(&skill_root, args),
            TestCommands::ReleaseGate(args) => test_cmd::release_gate(&skill_root, args),
        },
    };

    match result {
        Ok(()) => {}
        Err(e) => {
            eprintln!("{} {e:#}", "ERROR:".red().bold());
            process::exit(1);
        }
    }
}
