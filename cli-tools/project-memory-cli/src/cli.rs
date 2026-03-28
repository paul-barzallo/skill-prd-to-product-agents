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

    /// Override the default `.project-memory/config.toml` path
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Override the embedding provider (`local_hashed_v1`, `local_microservice`, `openai_compatible`)
    #[arg(long, global = true)]
    pub embedding_provider: Option<String>,

    /// Override the embedding provider endpoint
    #[arg(long, global = true)]
    pub embedding_endpoint: Option<String>,

    /// Override the embedding provider base URL
    #[arg(long, global = true)]
    pub embedding_base_url: Option<String>,

    /// Override the embedding deployment name for Azure-compatible providers
    #[arg(long, global = true)]
    pub embedding_deployment: Option<String>,

    /// Override the embedding API version for Azure-compatible providers
    #[arg(long, global = true)]
    pub embedding_api_version: Option<String>,

    /// Override the embedding provider model name
    #[arg(long, global = true)]
    pub embedding_model: Option<String>,

    /// Override the environment variable name that contains the embedding API key
    #[arg(long, global = true)]
    pub embedding_api_key_env: Option<String>,

    /// Explicitly enable or disable remote embedding providers
    #[arg(long, global = true)]
    pub embedding_remote_enabled: Option<bool>,

    /// Override the embedding provider timeout in milliseconds
    #[arg(long, global = true)]
    pub embedding_timeout_ms: Option<u64>,

    /// Override the maximum number of remote embedding requests allowed per command execution
    #[arg(long, global = true)]
    pub embedding_max_requests_per_run: Option<usize>,

    /// Override the fallback provider used when the primary embedding backend fails
    #[arg(long, global = true)]
    pub embedding_fallback_provider: Option<String>,

    /// Override the fallback endpoint used when the fallback provider is local_microservice
    #[arg(long, global = true)]
    pub embedding_fallback_endpoint: Option<String>,

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
    /// Retrieve ranked chunks for lexical project-memory recall
    Retrieve(RetrieveArgs),
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
pub struct RetrieveArgs {
    /// Text fragment to retrieve from indexed chunks
    #[arg(long)]
    pub text: String,

    /// Filter results by file type (for example: prd, rust_source, markdown, yaml)
    #[arg(long)]
    pub file_type: Option<String>,

    /// Filter results to paths containing this fragment
    #[arg(long)]
    pub path_contains: Option<String>,

    /// Maximum number of chunks to return
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
