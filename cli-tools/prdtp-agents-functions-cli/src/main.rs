mod agents;
mod audit;
mod board;
mod capabilities;
mod common;
mod database;
mod dependencies;
mod encoding;
mod git;
#[cfg(not(feature = "published-skill-contract"))]
mod github;
mod github_api;
mod governance;
mod logging;
mod operations;
mod reporting;
mod validate;

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(
    name = "prdtp-agents-functions-cli",
    version,
    about = "Product agents workspace CLI"
)]
struct Cli {
    /// Workspace root path (required)
    #[arg(long = "workspace", global = true)]
    workspace: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate workspace structure, agents, prompts, governance, models, encoding
    Validate {
        #[command(subcommand)]
        sub: ValidateCommands,
    },
    /// Manage operational state (handoffs, findings, releases, events)
    State {
        #[command(subcommand)]
        sub: StateCommands,
    },
    /// Git operations (task branches, finalize, pre-commit, hooks)
    Git {
        #[command(subcommand)]
        sub: GitCommands,
    },
    /// Audit ledger operations (sync, replay spool)
    Audit {
        #[command(subcommand)]
        sub: AuditCommands,
    },
    /// Reporting operations (snapshot, dashboard, export, serve)
    Report {
        #[command(subcommand)]
        sub: ReportCommands,
    },
    /// Capability detection and checks
    Capabilities {
        #[command(subcommand)]
        sub: CapabilitiesCommands,
    },
    /// Assemble agent .agent.md files from identity + context sources
    Agents {
        #[command(subcommand)]
        sub: AgentsCommands,
    },
    /// SQLite database initialization and migration
    Database {
        #[command(subcommand)]
        sub: DatabaseCommands,
    },
    /// Governance operations
    Governance {
        #[command(subcommand)]
        sub: GovernanceCommands,
    },
    /// Dependency detection and installation
    Dependencies {
        #[command(subcommand)]
        sub: DependenciesCommands,
    },
    /// GitHub issues/PR snapshot synchronization
    Board {
        #[command(subcommand)]
        sub: BoardCommands,
    },
    #[cfg(not(feature = "published-skill-contract"))]
    /// GitHub mutations routed through the runtime CLI
    Github {
        #[command(subcommand)]
        sub: github::GithubCommands,
    },
}

#[derive(Subcommand)]
enum ValidateCommands {
    /// Validate full workspace structure and YAML files
    Workspace,
    /// Validate whether a production-ready workspace satisfies the strong operational gate
    Readiness,
    /// Validate pull-request metadata, labels, commit subjects, and release gate expectations
    PrGovernance(validate::pr_governance::PrGovernanceArgs),
    /// Validate only the final release gate expectations for a PR targeting main
    ReleaseGate(validate::pr_governance::ReleaseGateArgs),
    /// Validate prompts have required sections
    Prompts,
    /// Validate agent hierarchy and contracts
    Agents,
    /// Validate github-governance.yaml (no placeholders)
    Governance,
    /// Validate model frontmatter against agent-model-policy.yaml
    Models,
    /// Validate file encoding (BOM, CRLF, mojibake)
    Encoding(encoding::EncodingArgs),
    /// CI-focused validation helpers used by the workflow
    Ci {
        #[command(subcommand)]
        sub: validate::ci::CiCommands,
    },
}

#[derive(Subcommand)]
enum StateCommands {
    /// Handoff operations
    Handoff {
        #[command(subcommand)]
        sub: HandoffCommands,
    },
    /// Finding operations
    Finding {
        #[command(subcommand)]
        sub: FindingCommands,
    },
    /// Release operations
    Release {
        #[command(subcommand)]
        sub: ReleaseCommands,
    },
    /// Record environment event
    Event {
        #[command(subcommand)]
        sub: EventCommands,
    },
}

#[derive(Subcommand)]
enum HandoffCommands {
    /// Create a new handoff entry
    Create(operations::handoff::CreateHandoffArgs),
    /// Update an existing handoff status
    Update(operations::handoff::UpdateHandoffArgs),
}

#[derive(Subcommand)]
enum FindingCommands {
    /// Create a new finding entry
    Create(operations::finding::CreateFindingArgs),
    /// Update an existing finding status
    Update(operations::finding::UpdateFindingArgs),
}

#[derive(Subcommand)]
enum ReleaseCommands {
    /// Create a new release entry
    Create(operations::release::CreateReleaseArgs),
    /// Update an existing release status
    Update(operations::release::UpdateReleaseArgs),
}

#[derive(Subcommand)]
enum EventCommands {
    /// Record an environment event
    Record(operations::environment::RecordEventArgs),
}

#[derive(Subcommand)]
enum GitCommands {
    /// Create or switch to a task branch
    CheckoutTaskBranch(git::branch::CheckoutTaskBranchArgs),
    /// Finalize a work unit (validate + commit)
    Finalize(git::commit::FinalizeArgs),
    /// Pre-commit validation (governance, branch protection, immutable files)
    PreCommitValidate(git::pre_commit::PreCommitArgs),
    /// Install git hooks into .git/hooks/
    InstallHooks,
}

#[derive(Subcommand)]
enum AuditCommands {
    /// Sync canonical docs into the SQLite audit ledger
    Sync,
    /// Replay JSON spool entries into the ledger
    ReplaySpool,
    #[cfg(not(feature = "published-skill-contract"))]
    /// Export structured audit evidence as JSONL
    Export(audit::export::ExportArgs),
    /// Validate or probe configured audit sinks
    Sink {
        #[command(subcommand)]
        sub: AuditSinkCommands,
    },
}

#[derive(Subcommand)]
enum AuditSinkCommands {
    /// Validate configured audit sinks and local hash-chain integrity
    Health,
    /// Emit a probe event through the configured audit sink
    Test,
}

#[derive(Subcommand)]
enum ReportCommands {
    /// Build report-snapshot.json from canonical docs
    Snapshot,
    /// Refresh management-dashboard.md from snapshot
    Dashboard,
    /// Export reports (CSV, XLSX)
    Export(reporting::export::ExportArgs),
    /// Start local HTTP server for reporting dashboard
    Serve(reporting::serve::ServeArgs),
    /// Run snapshot + dashboard + export (CSV & XLSX) in one step
    Pack,
}

#[derive(Subcommand)]
enum CapabilitiesCommands {
    /// Detect environment capabilities and write workspace-capabilities.yaml
    Detect(capabilities::detect::DetectArgs),
    /// Explicitly authorize or de-authorize a capability
    Authorize(capabilities::detect::AuthorizeArgs),
    /// Quick preflight capability check
    Check,
}

#[derive(Subcommand)]
enum AgentsCommands {
    /// Assemble .agent.md files from identity + context sources
    Assemble(agents::AssembleArgs),
}

#[derive(Subcommand)]
enum DatabaseCommands {
    /// Initialize or verify the SQLite audit ledger
    Init(database::InitArgs),
    /// Apply incremental schema migrations
    Migrate,
}

#[derive(Subcommand)]
enum GovernanceCommands {
    /// Configure local GitHub governance and render CODEOWNERS
    Configure(governance::ConfigureArgs),
    #[cfg(not(feature = "published-skill-contract"))]
    /// Promote an enterprise-configured workspace to the typed production-ready contract
    PromoteEnterpriseReadiness(governance::PromoteEnterpriseReadinessArgs),
    /// Provision remote GitHub controls for the enterprise profile
    ProvisionEnterprise(governance::ProvisionEnterpriseArgs),
}

#[derive(Subcommand)]
enum DependenciesCommands {
    /// Check workspace dependency availability
    Check(dependencies::CheckArgs),
}

#[derive(Subcommand)]
enum BoardCommands {
    /// Sync GitHub issues/PRs to docs/project/board.md
    Sync(board::SyncArgs),
}

fn command_name(command: &Commands) -> &'static str {
    match command {
        Commands::Validate { .. } => "validate",
        Commands::State { .. } => "state",
        Commands::Git { .. } => "git",
        Commands::Audit { .. } => "audit",
        Commands::Report { .. } => "report",
        Commands::Capabilities { .. } => "capabilities",
        Commands::Agents { .. } => "agents",
        Commands::Database { .. } => "database",
        Commands::Governance { .. } => "governance",
        Commands::Dependencies { .. } => "dependencies",
        Commands::Board { .. } => "board",
        #[cfg(not(feature = "published-skill-contract"))]
        Commands::Github { .. } => "github",
    }
}

fn execute(command: Commands, workspace: &std::path::Path) -> anyhow::Result<()> {
    match command {
        Commands::Validate { sub } => match sub {
            ValidateCommands::Workspace => validate::workspace::run(workspace),
            ValidateCommands::Readiness => validate::readiness::run(workspace),
            ValidateCommands::PrGovernance(args) => validate::pr_governance::run(workspace, args),
            ValidateCommands::ReleaseGate(args) => {
                validate::pr_governance::run_release_gate(workspace, args)
            }
            ValidateCommands::Prompts => validate::prompts::run(workspace),
            ValidateCommands::Agents => validate::agents::run(workspace),
            ValidateCommands::Governance => validate::governance::run(workspace),
            ValidateCommands::Models => validate::models::run(workspace),
            ValidateCommands::Encoding(args) => encoding::run(workspace, args),
            ValidateCommands::Ci { sub } => validate::ci::run(workspace, sub),
        },
        Commands::State { sub } => match sub {
            StateCommands::Handoff { sub } => match sub {
                HandoffCommands::Create(args) => operations::handoff::create(workspace, args),
                HandoffCommands::Update(args) => operations::handoff::update(workspace, args),
            },
            StateCommands::Finding { sub } => match sub {
                FindingCommands::Create(args) => operations::finding::create(workspace, args),
                FindingCommands::Update(args) => operations::finding::update(workspace, args),
            },
            StateCommands::Release { sub } => match sub {
                ReleaseCommands::Create(args) => operations::release::create(workspace, args),
                ReleaseCommands::Update(args) => operations::release::update(workspace, args),
            },
            StateCommands::Event { sub } => match sub {
                EventCommands::Record(args) => operations::environment::record(workspace, args),
            },
        },
        Commands::Git { sub } => match sub {
            GitCommands::CheckoutTaskBranch(args) => git::branch::run(workspace, args),
            GitCommands::Finalize(args) => git::commit::run(workspace, args),
            GitCommands::PreCommitValidate(args) => git::pre_commit::run(workspace, args),
            GitCommands::InstallHooks => git::hooks::run(workspace),
        },
        Commands::Audit { sub } => match sub {
            AuditCommands::Sync => audit::sync::run(workspace),
            AuditCommands::ReplaySpool => audit::replay::run(workspace),
            #[cfg(not(feature = "published-skill-contract"))]
            AuditCommands::Export(args) => audit::export::run(workspace, args),
            AuditCommands::Sink { sub } => match sub {
                AuditSinkCommands::Health => audit::sink::health(workspace),
                AuditSinkCommands::Test => audit::sink::test(workspace),
            },
        },
        Commands::Report { sub } => match sub {
            ReportCommands::Snapshot => reporting::snapshot::run(workspace),
            ReportCommands::Dashboard => reporting::dashboard::run(workspace),
            ReportCommands::Export(args) => reporting::export::run(workspace, args),
            ReportCommands::Serve(args) => reporting::serve::run(workspace, args),
            ReportCommands::Pack => reporting::pack::run(workspace),
        },
        Commands::Capabilities { sub } => match sub {
            CapabilitiesCommands::Detect(args) => capabilities::detect::run(workspace, args),
            CapabilitiesCommands::Authorize(args) => capabilities::detect::authorize(workspace, args),
            CapabilitiesCommands::Check => capabilities::detect::check(workspace),
        },
        Commands::Agents { sub } => match sub {
            AgentsCommands::Assemble(args) => agents::run(workspace, args),
        },
        Commands::Database { sub } => match sub {
            DatabaseCommands::Init(args) => database::init(workspace, args),
            DatabaseCommands::Migrate => database::migrate(workspace),
        },
        Commands::Governance { sub } => match sub {
            GovernanceCommands::Configure(args) => governance::configure(workspace, args),
            #[cfg(not(feature = "published-skill-contract"))]
            GovernanceCommands::PromoteEnterpriseReadiness(args) => {
                governance::promote_enterprise_readiness(workspace, args)
            }
            GovernanceCommands::ProvisionEnterprise(args) => {
                governance::provision_enterprise(workspace, args)
            }
        },
        Commands::Dependencies { sub } => match sub {
            DependenciesCommands::Check(args) => dependencies::run(workspace, args),
        },
        Commands::Board { sub } => match sub {
            BoardCommands::Sync(args) => board::run(workspace, args),
        },
        #[cfg(not(feature = "published-skill-contract"))]
        Commands::Github { sub } => github::run(workspace, sub),
    }
}

fn main() {
    let cli = Cli::parse();
    let command = cli.command;
    if std::env::var_os("PRDTP_AUDIT_CORRELATION_ID").is_none() {
        std::env::set_var(
            "PRDTP_AUDIT_CORRELATION_ID",
            format!(
                "cli-{}-{}",
                chrono::Utc::now().format("%Y%m%dT%H%M%S"),
                std::process::id()
            ),
        );
    }

    let raw_workspace = cli.workspace.unwrap_or_else(|| {
        eprintln!(
            "{} --workspace must be explicitly provided",
            "ERROR:".red().bold()
        );
        process::exit(1);
    });

    let workspace = raw_workspace.canonicalize().unwrap_or(raw_workspace);
    let command_label = command_name(&command);

    let log_guard = match logging::init(&workspace) {
        Ok(guard) => Some(guard),
        Err(error) => {
            eprintln!("{} tracing unavailable: {error:#}", "WARN:".yellow().bold());
            None
        }
    };

    tracing::info!(workspace = %workspace.display(), command = command_label, "dispatching runtime CLI command");

    let result = execute(command, &workspace);

    let exit_code = match result {
        Ok(()) => 0,
        Err(error) => {
            tracing::error!(error = ?error, "command failed");
            if !validate::is_validation_failure(&error) {
                eprintln!("{} {error:#}", "ERROR:".red().bold());
            }
            1
        }
    };

    drop(log_guard);

    if exit_code != 0 {
        process::exit(exit_code);
    }
}
