mod cli;
mod config;
mod embeddings;
mod model;
mod output;
mod query;
mod scan;
mod store;
mod trace;
mod util;
mod validate;
mod watch;

use anyhow::{Context, Result};
use std::path::PathBuf;

pub use cli::Cli;

pub struct RunOutcome {
    pub exit_code: i32,
}

pub fn run(cli: Cli) -> Result<RunOutcome> {
    let cli::Cli {
        project_root,
        config,
        embedding_provider,
        embedding_endpoint,
        embedding_base_url,
        embedding_deployment,
        embedding_api_version,
        embedding_model,
        embedding_api_key_env,
        embedding_remote_enabled,
        embedding_timeout_ms,
        embedding_max_requests_per_run,
        embedding_fallback_provider,
        embedding_fallback_endpoint,
        command,
    } = cli;

    let raw_project_root = project_root.unwrap_or_else(|| PathBuf::from("."));
    let project_root = util::resolve_project_root(&raw_project_root)
        .with_context(|| format!("resolving project root {}", raw_project_root.display()))?;
    let runtime_overrides = config::RuntimeOverrides {
        config_path: config,
        embedding_provider,
        embedding_endpoint,
        embedding_base_url,
        embedding_deployment,
        embedding_api_version,
        embedding_model,
        embedding_api_key_env,
        embedding_remote_enabled,
        embedding_timeout_ms,
        embedding_max_requests_per_run,
        embedding_fallback_provider,
        embedding_fallback_endpoint,
    };

    match command {
        cli::Commands::Ingest(args) => {
            let runtime_config = config::resolve(&project_root, &runtime_overrides)?;
            let embedding_service = embeddings::EmbeddingService::new(runtime_config.embedding)?;
            let (warnings, report) = scan::ingest(&project_root, &args, &embedding_service)?;
            output::print_json("ingest", &project_root, warnings, &report)?;
            Ok(RunOutcome { exit_code: 0 })
        }
        cli::Commands::Watch(args) => {
            let runtime_config = config::resolve(&project_root, &runtime_overrides)?;
            let embedding_service = embeddings::EmbeddingService::new(runtime_config.embedding)?;
            let (warnings, report) = watch::run(&project_root, &args, &embedding_service)?;
            output::print_json("watch", &project_root, warnings, &report)?;
            Ok(RunOutcome { exit_code: 0 })
        }
        cli::Commands::Query(args) => {
            let snapshot = store::load_snapshot(&project_root)?;
            let (warnings, report) = query::run(&snapshot, &args)?;
            output::print_json("query", &project_root, warnings, &report)?;
            Ok(RunOutcome { exit_code: 0 })
        }
        cli::Commands::Retrieve(args) => {
            let runtime_config = config::resolve(&project_root, &runtime_overrides)?;
            let embedding_service = embeddings::EmbeddingService::new(runtime_config.embedding)?;
            let snapshot = store::load_snapshot(&project_root)?;
            let (warnings, report) = query::run_retrieve(&project_root, &snapshot, &args, &embedding_service)?;
            output::print_json("retrieve", &project_root, warnings, &report)?;
            Ok(RunOutcome { exit_code: 0 })
        }
        cli::Commands::Trace(args) => {
            let snapshot = store::load_snapshot(&project_root)?;
            let (warnings, report) = trace::trace_report(&snapshot, &project_root, &args)?;
            output::print_json("trace", &project_root, warnings, &report)?;
            Ok(RunOutcome { exit_code: 0 })
        }
        cli::Commands::Impact(args) => {
            let snapshot = store::load_snapshot(&project_root)?;
            let (warnings, report) = trace::impact_report(&snapshot, &project_root, &args)?;
            output::print_json("impact", &project_root, warnings, &report)?;
            Ok(RunOutcome { exit_code: 0 })
        }
        cli::Commands::Validate(args) => {
            let snapshot = store::load_snapshot(&project_root)?;
            let report = validate::validate_snapshot(&snapshot, args.fail_on_warnings);
            let exit_code = if report.summary.errors > 0
                || (args.fail_on_warnings && report.summary.warnings > 0)
            {
                1
            } else {
                0
            };

            output::print_json("validate", &project_root, Vec::new(), &report)?;
            Ok(RunOutcome { exit_code })
        }
    }
}
