use anyhow::{bail, Result};
use clap::Args;
use colored::Colorize;
use std::fs;
use std::path::Path;

#[derive(Args)]
pub struct AssembleArgs {
    /// Verify only (compare without writing)
    #[arg(long)]
    pub verify: bool,
}

pub fn run(workspace: &Path, args: AssembleArgs) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), verify = args.verify, "assembling agent files");
    let agents_dir = workspace.join(".github/agents");
    let identity_dir = agents_dir.join("identity");
    let context_dir = agents_dir.join("context");
    let divider_file = agents_dir.join("CONTEXT_ZONE_DIVIDER.txt");

    if !divider_file.exists() {
        bail!("Divider template not found: {}", divider_file.display());
    }
    let divider = fs::read_to_string(&divider_file)?
        .replace("\r\n", "\n")
        .trim_end()
        .to_string();

    let shared_context_file = context_dir.join("shared-context.md");
    let shared_context = if shared_context_file.exists() {
        fs::read_to_string(&shared_context_file)?.replace("\r\n", "\n")
    } else {
        tracing::warn!(path = %shared_context_file.display(), "shared context file missing; assembling agents without shared context");
        eprintln!("{} No shared-context.md found — agents will have no shared context", "WARN:".yellow().bold());
        String::new()
    };

    if !identity_dir.is_dir() {
        bail!("Identity directory not found: {}", identity_dir.display());
    }

    let mut assembled = 0u32;
    let mut mismatched = 0u32;

    let mut entries: Vec<_> = fs::read_dir(&identity_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |x| x == "md"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let name = entry.path().file_stem().unwrap().to_string_lossy().to_string();
        let identity_content = fs::read_to_string(entry.path())?.replace("\r\n", "\n");
        let context_file = context_dir.join(format!("{name}.md"));
        let agent_file = agents_dir.join(format!("{name}.agent.md"));

        let context_content = if context_file.exists() {
            fs::read_to_string(&context_file)?.replace("\r\n", "\n")
        } else {
            tracing::warn!(agent = %name, path = %context_file.display(), "agent-specific context file missing; using empty context");
            eprintln!("{} No context file for {name} — using empty context", "WARN:".yellow().bold());
            String::new()
        };

        let mut content = format!("{}\n\n{}\n\n", identity_content.trim_end(), divider);
        if !shared_context.is_empty() {
            content.push_str(&format!("{}\n\n", shared_context.trim_end()));
        }
        content.push_str(&format!("{}\n", context_content.trim_end()));

        if args.verify {
            if !agent_file.exists() {
                tracing::error!(agent = %name, path = %agent_file.display(), "assembled agent file missing during verification");
                eprintln!("  {} {name}.agent.md does not exist", "MISMATCH:".red());
                mismatched += 1;
            } else {
                let existing = fs::read_to_string(&agent_file)?.replace("\r\n", "\n");
                if content.trim_end() != existing.trim_end() {
                    tracing::error!(agent = %name, path = %agent_file.display(), "assembled agent file differs from expected output");
                    eprintln!("  {} {name}.agent.md differs from assembled output", "MISMATCH:".red());
                    mismatched += 1;
                }
            }
        } else {
            crate::common::fs_util::write_utf8(&agent_file, &content)?;
        }
        assembled += 1;
    }

    if args.verify {
        tracing::info!(assembled, mismatched, "agent assembly verification completed");
        println!("Verified {assembled} agent files: {mismatched} mismatch(es)");
        if mismatched > 0 {
            bail!("Run 'prdtp-agents-functions-cli agents assemble' to regenerate .agent.md files.");
        }
    } else {
        tracing::info!(assembled, "agent assembly write completed");
        println!("{} Assembled {assembled} agent files", "✓".green().bold());
    }
    Ok(())
}

/// Used internally by pre-commit validation
pub fn verify_assembly(workspace: &Path) -> Result<()> {
    run(workspace, AssembleArgs { verify: true })
}
