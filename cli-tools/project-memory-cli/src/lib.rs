mod cli;
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
    let raw_project_root = cli.project_root.unwrap_or_else(|| PathBuf::from("."));
    let project_root = util::resolve_project_root(&raw_project_root)
        .with_context(|| format!("resolving project root {}", raw_project_root.display()))?;

    match cli.command {
        cli::Commands::Ingest(args) => {
            let (warnings, report) = scan::ingest(&project_root, &args)?;
            output::print_json("ingest", &project_root, warnings, &report)?;
            Ok(RunOutcome { exit_code: 0 })
        }
        cli::Commands::Watch(args) => {
            let (warnings, report) = watch::run(&project_root, &args)?;
            output::print_json("watch", &project_root, warnings, &report)?;
            Ok(RunOutcome { exit_code: 0 })
        }
        cli::Commands::Query(args) => {
            let snapshot = store::load_snapshot(&project_root)?;
            let (warnings, report) = query::run(&snapshot, &args)?;
            output::print_json("query", &project_root, warnings, &report)?;
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
