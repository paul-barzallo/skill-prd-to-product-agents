use anyhow::{bail, Result};
use clap::Args;
use colored::Colorize;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Args)]
pub struct WorkspaceArgs {
    /// Target workspace directory to clean
    #[arg(long)]
    pub target: Option<std::path::PathBuf>,
    /// Also remove .git directory
    #[arg(long)]
    pub include_git: bool,
    /// Dry-run mode: list files without deleting
    #[arg(long)]
    pub dry_run: bool,
}

/// Remove bootstrap-deployed artifacts per manifest.
pub fn workspace(_skill_root: &Path, args: WorkspaceArgs) -> Result<()> {
    let target = args
        .target
        .as_deref()
        .unwrap_or(Path::new("."));
    let target = target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf());

    let manifest_path = target.join(".state").join("bootstrap-manifest.txt");
    if !manifest_path.exists() {
        bail!(
            "Bootstrap manifest not found at {}",
            manifest_path.display()
        );
    }

    tracing::info!(
        target = %target.display(),
        dry_run = args.dry_run,
        include_git = args.include_git,
        "starting workspace cleanup"
    );

    let content = fs::read_to_string(&manifest_path)?;
    let entries = parse_manifest(&content);

    let mut deleted = 0u32;
    let mut kept = 0u32;
    let mut missing = 0u32;

    // Collect files to delete
    let mut dirs_to_check: HashSet<String> = HashSet::new();

    for entry in &entries {
        if entry.cleanup_action == "keep" {
            kept += 1;
            if args.dry_run {
                println!("  {} {} (keep)", "SKIP:".cyan(), entry.path);
            }
            continue;
        }

        if !is_safe_relative(&entry.path) {
            tracing::warn!(path = %entry.path, "unsafe cleanup path skipped");
            eprintln!(
                "  {} unsafe path skipped: {}",
                "WARN:".yellow(),
                entry.path
            );
            continue;
        }

        let full_path = target.join(entry.path.replace('/', std::path::MAIN_SEPARATOR_STR));
        if !full_path.exists() {
            missing += 1;
            continue;
        }

        if args.dry_run {
            println!("  {} {}", "DELETE:".red(), entry.path);
        } else {
            if let Err(e) = fs::remove_file(&full_path) {
                tracing::warn!(path = %entry.path, error = %e, "failed to remove manifest file during cleanup");
                eprintln!(
                    "  {} failed to remove {}: {e}",
                    "WARN:".yellow(),
                    entry.path
                );
            } else {
                deleted += 1;
            }
        }

        // Track parent dirs for pruning
        if let Some(parent) = Path::new(&entry.path).parent() {
            let parent_str = parent.to_string_lossy().replace('\\', "/");
            if !parent_str.is_empty() {
                dirs_to_check.insert(parent_str);
            }
        }
    }

    // Extra cleanup: .state artifacts
    let extra_files = [
        ".state/project_memory.db",
        ".state/sqlite-bootstrap.pending.md",
        ".state/sqlite-bootstrap.report.md",
        ".state/workspace-validation.md",
    ];
    for extra in &extra_files {
        let full = target.join(extra.replace('/', std::path::MAIN_SEPARATOR_STR));
        if full.exists() {
            if args.dry_run {
                println!("  {} {extra}", "DELETE:".red());
            } else {
                fs::remove_file(&full).map_err(|e| anyhow::anyhow!("Failed to delete file {}: {}", full.display(), e))?;
                deleted += 1;
            }
        }
    }

    // Extra cleanup: directories
    let extra_dirs = [
        ".bootstrap-overlays",
        ".state/audit-spool",
        ".state/degraded-ops",
        ".state/local-history",
        ".state/reporting",
        ".state/work-units",
    ];
    for extra_dir in &extra_dirs {
        let full = target.join(extra_dir.replace('/', std::path::MAIN_SEPARATOR_STR));
        if full.is_dir() {
            if args.dry_run {
                println!("  {} {extra_dir}/ (recursive)", "DELETE:".red());
            } else {
                fs::remove_dir_all(&full).map_err(|e| anyhow::anyhow!("Failed to delete directory {}: {}", full.display(), e))?;
            }
        }
    }

    // Optional: remove .git
    if args.include_git {
        let git_dir = target.join(".git");
        if git_dir.is_dir() {
            if args.dry_run {
                println!("  {} .git/ (recursive)", "DELETE:".red());
            } else {
                fs::remove_dir_all(&git_dir).map_err(|e| anyhow::anyhow!("Failed to delete git directory {}: {}", git_dir.display(), e))?;
            }
        }
    }

    // Prune empty directories
    if !args.dry_run {
        let mut sorted_dirs: Vec<String> = dirs_to_check.into_iter().collect();
        sorted_dirs.sort_by(|a, b| b.len().cmp(&a.len())); // deepest first
        for dir in &sorted_dirs {
            let full = target.join(dir.replace('/', std::path::MAIN_SEPARATOR_STR));
            if full.is_dir() {
                let is_empty = fs::read_dir(&full)
                    .map(|mut d| d.next().is_none())
                    .unwrap_or(false);
                if is_empty {
                    fs::remove_dir(&full).map_err(|e| anyhow::anyhow!("Failed to delete empty directory {}: {}", full.display(), e))?;
                }
            }
        }
    }

    // Summary
    if args.dry_run {
        println!("\n{}", "--- Dry run summary ---".cyan());
    } else {
        println!("\n{}", "--- Cleanup summary ---".cyan());
    }
    println!("  Deleted: {deleted}");
    println!("  Kept:    {kept}");
    println!("  Missing: {missing}");

    tracing::info!(
        target = %target.display(),
        deleted,
        kept,
        missing,
        dry_run = args.dry_run,
        "workspace cleanup finished"
    );

    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────

struct ManifestEntry {
    path: String,
    #[allow(dead_code)]
    kind: String,
    #[allow(dead_code)]
    ownership: String,
    cleanup_action: String,
}

fn parse_manifest(content: &str) -> Vec<ManifestEntry> {
    content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            if l.contains('\t') {
                let parts: Vec<&str> = l.split('\t').collect();
                if parts.len() >= 4 {
                    Some(ManifestEntry {
                        path: parts[0].trim().replace('\\', "/"),
                        kind: parts[1].trim().to_string(),
                        ownership: parts[2].trim().to_string(),
                        cleanup_action: parts[3].trim().to_string(),
                    })
                } else {
                    Some(ManifestEntry {
                        path: parts[0].trim().replace('\\', "/"),
                        kind: "legacy".to_string(),
                        ownership: "unknown".to_string(),
                        cleanup_action: "delete".to_string(),
                    })
                }
            } else {
                Some(ManifestEntry {
                    path: l.replace('\\', "/"),
                    kind: "legacy".to_string(),
                    ownership: "unknown".to_string(),
                    cleanup_action: if is_host_file(l) { "keep" } else { "delete" }.to_string(),
                })
            }
        })
        .collect()
}

fn is_safe_relative(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    let p = Path::new(path);
    for component in p.components() {
        match component {
            std::path::Component::ParentDir 
            | std::path::Component::RootDir 
            | std::path::Component::Prefix(_) => return false,
            _ => {}
        }
    }
    true
}

fn is_host_file(path: &str) -> bool {
    matches!(
        path,
        "AGENTS.md" | ".instructions.md" | ".gitignore" | ".gitattributes"
    )
}




