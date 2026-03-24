use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "project-memory-cli",
    version,
    about = "Project memory CLI for local repository indexing, traceability, and incremental retrieval"
)]
pub struct Cli {
    /// Project root directory to index and query
    #[arg(long, global = true)]
    pub project_root: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Scan the repository, persist the local memory snapshot, and refresh derived trace data
    Ingest(IngestArgs),
    /// Watch the project tree and refresh the snapshot when relevant files change
    Watch(WatchArgs),
    /// Search indexed files and return relevant fragments
    Query(QueryArgs),
    /// Show requirement and artifact trace links from the persisted snapshot
    Trace(TraceArgs),
    /// Show reverse reachability for a requirement or artifact node
    Impact(ImpactArgs),
    /// Report coverage and consistency findings from the persisted snapshot
    Validate(ValidateArgs),
}

#[derive(Args, Debug)]
pub struct IngestArgs {
    /// Ignore the previous snapshot and rebuild all indexed file records
    #[arg(long, default_value_t = false)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct QueryArgs {
    /// Text fragment to search in indexed content
    #[arg(long)]
    pub text: Option<String>,

    /// Filter results to files that declare a matching symbol
    #[arg(long)]
    pub symbol: Option<String>,

    /// Filter results to files that import a matching dependency path
    #[arg(long)]
    pub import: Option<String>,

    /// Filter results by file type (for example: prd, rust_source, markdown, yaml)
    #[arg(long)]
    pub file_type: Option<String>,

    /// Filter results to paths containing this fragment
    #[arg(long)]
    pub path_contains: Option<String>,

    /// Maximum number of results to return
    #[arg(long, default_value_t = 10)]
    pub limit: usize,
}

#[derive(Args, Debug)]
pub struct TraceArgs {
    /// Restrict trace output to a specific requirement identifier
    #[arg(long)]
    pub requirement: Option<String>,

    /// Restrict trace output to a specific project-relative path
    #[arg(long)]
    pub path: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct ImpactArgs {
    /// Requirement identifier or project-relative path to analyze
    #[arg(long)]
    pub node: String,
}

#[derive(Args, Debug)]
pub struct ValidateArgs {
    /// Return a non-zero exit code when only warnings are present
    #[arg(long, default_value_t = false)]
    pub fail_on_warnings: bool,
}

#[derive(Args, Debug)]
pub struct WatchArgs {
    /// Poll interval for filesystem changes in milliseconds
    #[arg(long, default_value_t = 250)]
    pub interval_ms: u64,

    /// Stop after collecting this many refresh events
    #[arg(long, default_value_t = 1)]
    pub max_events: usize,

    /// Stop waiting after this timeout in milliseconds
    #[arg(long)]
    pub timeout_ms: Option<u64>,

    /// Force a full ingest before starting the watch loop
    #[arg(long, default_value_t = false)]
    pub force_initial_ingest: bool,
}
