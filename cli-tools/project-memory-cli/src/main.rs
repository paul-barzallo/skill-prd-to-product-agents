use clap::Parser;
use colored::Colorize;
use std::process;

fn main() {
    let cli = project_memory_cli::Cli::parse();

    match project_memory_cli::run(cli) {
        Ok(outcome) => {
            if outcome.exit_code != 0 {
                process::exit(outcome.exit_code);
            }
        }
        Err(error) => {
            eprintln!("{} {error:#}", "ERROR:".red().bold());
            process::exit(1);
        }
    }
}
