use anyhow::{bail, Result};
use clap::Args;
use colored::Colorize;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Args)]
pub struct EncodingArgs {
    /// Target directory (defaults to workspace root)
    #[arg(long)]
    pub target: Option<std::path::PathBuf>,
}

const LF_EXTENSIONS: &[&str] = &["md", "txt", "yaml", "yml", "json", "jsonc", "sql", "sh"];
const MOJIBAKE_EXTENSIONS: &[&str] = &["md", "txt"];
const IGNORED_DIRS: &[&str] = &[
    ".git",
    ".state",
    ".bootstrap-overlays",
    "target",
    "node_modules",
];

/// UTF-8 BOM bytes: EF BB BF
fn has_bom(bytes: &[u8]) -> bool {
    bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF
}

fn has_crlf(bytes: &[u8]) -> bool {
    bytes.windows(2).any(|w| w[0] == 0x0D && w[1] == 0x0A)
}

fn has_mojibake(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    text.contains('\u{00E2}') || text.contains('\u{00C3}')
}

pub fn run(workspace: &Path, args: EncodingArgs) -> Result<()> {
    let target = args.target.as_deref().unwrap_or(workspace);
    tracing::info!(workspace = %workspace.display(), target = %target.display(), "running encoding validation");
    let mut errors = Vec::new();

    for entry in WalkDir::new(target).into_iter().filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        !IGNORED_DIRS.contains(&name.as_ref())
    }) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let ext = match path.extension().and_then(|e| e.to_str()) {
            Some(e) => e,
            None => continue,
        };
        if !LF_EXTENSIONS.contains(&ext) {
            continue;
        }

        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(_) => continue,
        };

        let rel = path.strip_prefix(target).unwrap_or(path);

        if has_bom(&bytes) {
            tracing::error!(path = %rel.display(), "file contains utf-8 bom");
            errors.push(format!("{} contains UTF-8 BOM", rel.display()));
        }
        if has_crlf(&bytes) {
            tracing::error!(path = %rel.display(), "file contains crlf line endings where lf is required");
            errors.push(format!(
                "{} contains CRLF line endings but LF is required",
                rel.display()
            ));
        }
        if MOJIBAKE_EXTENSIONS.contains(&ext) && has_mojibake(&bytes) {
            tracing::error!(path = %rel.display(), "file contains mojibake sequences");
            errors.push(format!("{} contains mojibake sequences", rel.display()));
        }
    }

    if errors.is_empty() {
        tracing::info!("encoding validation passed");
        println!("{}", "Encoding check passed.".green());
        Ok(())
    } else {
        tracing::error!(count = errors.len(), "encoding validation failed");
        for e in &errors {
            eprintln!("{} {e}", "ERROR:".red().bold());
        }
        bail!("{} encoding error(s) found", errors.len())
    }
}
