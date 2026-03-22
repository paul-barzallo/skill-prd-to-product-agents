use anyhow::{bail, Result};
use clap::Args;
use colored::Colorize;
use std::fs;
use std::path::Path;

#[derive(Args)]
pub struct PreCommitArgs {
    /// Workspace root path (required)
    #[arg(long)]
    pub workspace_root: std::path::PathBuf,
    /// Explicitly listed staged files (if empty, auto-detect via git)
    #[arg(long = "staged-file")]
    pub staged_files: Vec<String>,
}

fn normalize_repo_path(path: &str) -> String {
    let mut p = path.replace('\\', "/");
    if let Some(stripped) = p.strip_prefix("./") {
        p = stripped.to_string();
    }
    p.trim_start_matches('/').to_string()
}

fn get_required_immutable_entries(workspace: &Path) -> Vec<String> {
    let mut entries = vec![
        ".github/copilot-instructions.md".to_string(),
        ".github/github-governance.yaml".to_string(),
        ".github/instructions/agents.instructions.md".to_string(),
        ".github/instructions/docs.instructions.md".to_string(),
        ".github/agents/CONTEXT_ZONE_DIVIDER.txt".to_string(),
        "docs/project/handoffs.yaml".to_string(),
        "docs/project/findings.yaml".to_string(),
        "docs/project/releases.yaml".to_string(),
        ".state/memory-schema.sql".to_string(),
    ];
    let identity_dir = workspace.join(".github/agents/identity");
    if identity_dir.is_dir() {
        if let Ok(rd) = fs::read_dir(&identity_dir) {
            let mut idents: Vec<String> = rd
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().extension().map_or(false, |x| x == "md")
                })
                .map(|e| {
                    format!(
                        ".github/agents/identity/{}",
                        e.file_name().to_string_lossy()
                    )
                })
                .collect();
            idents.sort();
            entries.extend(idents);
        }
    }
    entries.sort();
    entries.dedup();
    entries
}

fn get_staged_files(workspace: &Path) -> Vec<String> {
    let output = std::process::Command::new("git")
        .args(["-C", &workspace.to_string_lossy(), "diff", "--cached", "--name-only", "--diff-filter=ACMR"])
        .output();
    match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .map(|l| normalize_repo_path(l))
                .collect()
        }
        _ => Vec::new(),
    }
}

fn get_current_branch(workspace: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["-C", &workspace.to_string_lossy(), "branch", "--show-current"])
        .output()
        .ok()?;
    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch.is_empty() { None } else { Some(branch) }
    } else {
        None
    }
}

fn validate_yaml_structural(workspace: &Path, rel_path: &str) -> Result<()> {
    let full = workspace.join(rel_path);
    if !full.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(&full)?;
    if content.contains('\t') {
        bail!("{rel_path} contains tab characters. Use spaces only.");
    }
    if content.trim().is_empty() {
        bail!("{rel_path} is empty.");
    }
    // Parse YAML with serde_yaml (native Rust — no Node.js needed)
    let _: serde_yaml::Value = serde_yaml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("{rel_path} is not valid YAML: {e}"))?;
    Ok(())
}

pub fn run(workspace: &Path, args: PreCommitArgs) -> Result<()> {
    let ws = args.workspace_root.canonicalize().unwrap_or_else(|_| workspace.to_path_buf());

    // --- Commit path guard ---
    // These env vars bypass governance controls and are strictly for:
    //   - BOOTSTRAP_ALLOW_MAIN_COMMIT: the bootstrap CLI's initial commit on main
    //   - FINALIZE_WORK_UNIT_ALLOW_COMMIT: the `git finalize` subcommand's automated commit
    // They must NEVER be set manually by users or in production CI.
    let bootstrap_allow = std::env::var("BOOTSTRAP_ALLOW_MAIN_COMMIT").unwrap_or_default() == "1";
    let finalize_allow = std::env::var("FINALIZE_WORK_UNIT_ALLOW_COMMIT").unwrap_or_default() == "1";

    if !bootstrap_allow && !finalize_allow {
        bail!("Direct git commit is out of contract in this workspace. Use prdtp-agents-functions-cli git finalize.");
    }

    // AUDIT TRAIL: Require explicit logging if bypass tokens are used.
    if bootstrap_allow || finalize_allow {
        let bypass_type = if bootstrap_allow { "BOOTSTRAP_ALLOW_MAIN_COMMIT" } else { "FINALIZE_WORK_UNIT_ALLOW_COMMIT" };
        eprintln!("[AUDIT_LOG_BYPASS] Commit intercepted by hook. Governance constraint bypassed via authorized environment variable: {}", bypass_type);
        
        let audit_dir = ws.join(".state/audit-spool");
        let _ = std::fs::create_dir_all(&audit_dir);
        let audit_file = audit_dir.join("governance.log");
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(audit_file) {
            let timestamp = chrono::Utc::now().to_rfc3339();
            let _ = writeln!(file, "[{timestamp}] COMMIT BYPASS: {bypass_type} triggered for workspace {}", ws.display());
        }
    }

    // --- Branch protection guard ---
    if ws.join(".git").is_dir() {
        match get_current_branch(&ws) {
            None => {
                if !bootstrap_allow {
                    bail!("Cannot commit in detached HEAD state. Check out a task branch (<role>/<issue-id>-slug) first.");
                }
            }
            Some(ref branch) if branch == "main" || branch == "develop" => {
                if !bootstrap_allow {
                    bail!("Direct commits on '{branch}' are out of contract. Use a task branch (<role>/<issue-id>-slug) based on develop.");
                }
            }
            _ => {}
        }
    }

    // --- Resolve staged files ---
    let staged: Vec<String> = if args.staged_files.is_empty() {
        get_staged_files(&ws)
    } else {
        args.staged_files.iter().map(|f| normalize_repo_path(f)).collect()
    };

    // --- Immutable files manifest ---
    let manifest_path = ws.join(".github/immutable-files.txt");
    if !manifest_path.exists() {
        bail!(".github/immutable-files.txt is required for governance enforcement.");
    }
    let manifest_content = fs::read_to_string(&manifest_path)?;
    let manifest_entries: Vec<String> = manifest_content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| normalize_repo_path(l))
        .collect();

    if manifest_entries.is_empty() {
        bail!(".github/immutable-files.txt is empty. Add the governance files protected by the pre-commit hook.");
    }

    // Check duplicates
    let mut sorted = manifest_entries.clone();
    sorted.sort();
    let mut prev = "";
    for entry in &sorted {
        if entry == prev {
            bail!(".github/immutable-files.txt contains duplicate entry: {entry}");
        }
        prev = entry;
    }

    // Check required entries present
    let required = get_required_immutable_entries(&ws);
    let missing: Vec<&str> = required
        .iter()
        .filter(|r| !manifest_entries.iter().any(|m| m == r.as_str()))
        .map(|s| s.as_str())
        .collect();
    if !missing.is_empty() {
        bail!(
            ".github/immutable-files.txt is missing required governance entries:\n  {}",
            missing.join("\n  ")
        );
    }

    // Check manifest paths exist (skip operational YAML that may not exist yet)
    let operational_yaml = ["docs/project/handoffs.yaml", "docs/project/findings.yaml", "docs/project/releases.yaml"];
    let missing_paths: Vec<&str> = manifest_entries
        .iter()
        .filter(|e| !operational_yaml.contains(&e.as_str()) && !ws.join(e).exists())
        .map(|s| s.as_str())
        .collect();
    if !missing_paths.is_empty() {
        bail!(
            ".github/immutable-files.txt references missing paths:\n  {}",
            missing_paths.join("\n  ")
        );
    }

    // --- Immutable file edit detection ---
    let mut immutable_bypass = false;
    let mut immutable_bypass_reason = String::new();
    let mut immutable_bypass_files: std::collections::HashSet<String> = std::collections::HashSet::new();
    let token_path = ws.join(".state/.immutable-edit-token");

    if bootstrap_allow {
        immutable_bypass = true;
        immutable_bypass_reason = "bootstrap".to_string();
    } else if token_path.exists() {
        if let Ok(content) = fs::read_to_string(&token_path) {
            // Verify integrity before trusting the token
            if !crate::governance::verify_token_integrity(&content, &ws) {
                eprintln!("{} Immutable-edit token integrity check failed. Ignoring token.", "WARN:".yellow().bold());
                let _ = fs::remove_file(&token_path);
            } else if let Ok(token) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(expires) = token.get("expires_epoch").and_then(|v| v.as_i64()) {
                    let now = chrono::Utc::now().timestamp();
                    if now < expires {
                        immutable_bypass = true;
                        immutable_bypass_reason = token
                            .get("reason")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        if let Some(files) = token.get("files").and_then(|v| v.as_array()) {
                            immutable_bypass_files = files
                                .iter()
                                .filter_map(|value| value.as_str())
                                .map(normalize_repo_path)
                                .collect();
                        }
                    } else {
                        eprintln!("{} Immutable-edit token has expired. Removing.", "WARN:".yellow().bold());
                        let _ = fs::remove_file(&token_path);
                    }
                }
            }
        }
    }

    let immutable_hits: Vec<&String> = staged
        .iter()
        .filter(|f| manifest_entries.contains(f))
        .collect();

    if !immutable_hits.is_empty() {
        let operational_hits: Vec<&&String> = immutable_hits
            .iter()
            .filter(|f| operational_yaml.contains(&f.as_str()))
            .collect();
        let governance_hits: Vec<&&String> = immutable_hits
            .iter()
            .filter(|f| !operational_yaml.contains(&f.as_str()))
            .collect();

        // Operational YAML allowed via finalize pathway
        if !operational_hits.is_empty() && !finalize_allow && !bootstrap_allow {
            let files: Vec<String> = operational_hits.iter().map(|f| format!("  {f}")).collect();
            bail!(
                "Operational state files must be mutated through prdtp-agents-functions-cli state and committed via prdtp-agents-functions-cli git finalize:\n{}",
                files.join("\n")
            );
        }

        // Governance files require token
        if !governance_hits.is_empty() {
            if !immutable_bypass {
                let files: Vec<String> = governance_hits.iter().map(|f| format!("  {f}")).collect();
                bail!(
                    "Immutable governance files are staged:\n{}\nRun: prdtp-agents-functions-cli governance immutable-token --reason \"<reason>\" to obtain a time-limited token.",
                    files.join("\n")
                );
            }
            if immutable_bypass_reason != "bootstrap" {
                let unauthorized: Vec<String> = governance_hits
                    .iter()
                    .filter(|path| !immutable_bypass_files.contains(path.as_str()))
                    .map(|path| format!("  {path}"))
                    .collect();
                if !unauthorized.is_empty() {
                    let _ = fs::remove_file(&token_path);
                    bail!(
                        "Immutable-edit token does not authorize all staged governance files:\n{}\nRequest a new token with the exact file list.",
                        unauthorized.join("\n")
                    );
                }
            }

            // Log bypass to audit spool
            let spool_dir = ws.join(".state/audit-spool");
            let _ = fs::create_dir_all(&spool_dir);
            let ts = chrono::Utc::now();
            let bypass_id = format!("imm-{}-{}", ts.timestamp(), std::process::id());
            let files_list: Vec<&str> = governance_hits.iter().map(|f| f.as_str()).collect();
            let record = serde_json::json!({
                "id": bypass_id,
                "activity_type": "immutable_edit_bypass",
                "timestamp": ts.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                "reason": immutable_bypass_reason,
                "files": files_list,
            });
            let _ = fs::write(
                spool_dir.join(format!("{bypass_id}.json")),
                serde_json::to_string_pretty(&record)?,
            );

            // Consume single-use token
            if immutable_bypass_reason != "bootstrap" {
                let _ = fs::remove_file(&token_path);
                eprintln!("{} Immutable-edit token consumed.", "WARN:".yellow().bold());
            }
            eprintln!(
                "{} Immutable governance files edited under authorized bypass (reason: {immutable_bypass_reason})",
                "WARN:".yellow().bold()
            );
        }
    }

    // --- Agent assembly verification ---
    let agent_sources_staged: Vec<&String> = staged
        .iter()
        .filter(|f| {
            f.starts_with(".github/agents/identity/") || f.starts_with(".github/agents/context/")
        })
        .collect();
    if !agent_sources_staged.is_empty() {
        println!("Agent source files staged -- verifying assembly sync...");
        // Delegate to our own agents assemble --verify
        crate::agents::verify_assembly(&ws)?;
    }

    // --- Operational YAML structural validation ---
    let yaml_staged: Vec<&String> = staged
        .iter()
        .filter(|f| {
            let re = regex::Regex::new(r"^docs/project/[^/]+\.ya?ml$").unwrap();
            re.is_match(f)
        })
        .collect();
    if !yaml_staged.is_empty() {
        println!("Operational YAML files staged -- checking structural integrity...");
        for yaml_file in &yaml_staged {
            validate_yaml_structural(&ws, yaml_file)?;
        }
    }

    // --- Gitleaks ---
    if which_command("gitleaks") {
        println!("Scanning staged changes for secrets...");
        let status = std::process::Command::new("gitleaks")
            .args(["git", "--pre-commit", "--no-banner", "-q"])
            .current_dir(&ws)
            .status();
        if let Ok(s) = status {
            if !s.success() {
                bail!("gitleaks detected secrets in staged changes. Unstage the offending files, remove the secrets, and re-stage.");
            }
        }
    } else {
        eprintln!("{} gitleaks not installed -- secrets scanning skipped.", "WARN:".yellow().bold());
    }

    println!("{}", "Pre-commit validation passed.".green());
    Ok(())
}

fn which_command(name: &str) -> bool {
    std::process::Command::new(name)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Governance checks extracted for reuse by `git finalize`.
/// Enforces immutable-file protection on the given list of changed files,
/// verifies agent assembly if identity/context sources changed, and validates
/// operational YAML structural integrity.
pub fn run_governance_checks(workspace: &Path, changed_files: &[String]) -> Result<()> {
    let ws = workspace.canonicalize().unwrap_or_else(|_| workspace.to_path_buf());

    // --- Immutable files manifest ---
    let manifest_path = ws.join(".github/immutable-files.txt");
    if !manifest_path.exists() {
        // No manifest → nothing to enforce
        return Ok(());
    }
    let manifest_content = fs::read_to_string(&manifest_path)?;
    let manifest_entries: Vec<String> = manifest_content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| normalize_repo_path(l))
        .collect();

    let operational_yaml = [
        "docs/project/handoffs.yaml",
        "docs/project/findings.yaml",
        "docs/project/releases.yaml",
    ];

    let normalized: Vec<String> = changed_files
        .iter()
        .map(|f| normalize_repo_path(f))
        .collect();

    // Detect immutable governance file edits (operational YAML is allowed via finalize)
    let governance_hits: Vec<&String> = normalized
        .iter()
        .filter(|f| manifest_entries.contains(f) && !operational_yaml.contains(&f.as_str()))
        .collect();

    if !governance_hits.is_empty() {
        // Check for immutable-edit token
        let token_path = ws.join(".state/.immutable-edit-token");
        let mut bypass = false;
        let mut token_files: std::collections::HashSet<String> = std::collections::HashSet::new();
        if token_path.exists() {
            if let Ok(content) = fs::read_to_string(&token_path) {
                if crate::governance::verify_token_integrity(&content, &ws) {
                    if let Ok(token) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(expires) = token.get("expires_epoch").and_then(|v| v.as_i64()) {
                            if chrono::Utc::now().timestamp() < expires {
                                bypass = true;
                                if let Some(files) = token.get("files").and_then(|v| v.as_array()) {
                                    token_files = files
                                        .iter()
                                        .filter_map(|value| value.as_str())
                                        .map(normalize_repo_path)
                                        .collect();
                                }
                                let _ = fs::remove_file(&token_path);
                            }
                        }
                    }
                }
            }
        }
        if !bypass {
            let files: Vec<String> = governance_hits.iter().map(|f| format!("  {f}")).collect();
            bail!(
                "Immutable governance files changed:\n{}\nUse: prdtp-agents-functions-cli governance immutable-token --reason \"<reason>\" first.",
                files.join("\n")
            );
        }
        let unauthorized: Vec<String> = governance_hits
            .iter()
            .filter(|path| !token_files.contains(path.as_str()))
            .map(|path| format!("  {path}"))
            .collect();
        if !unauthorized.is_empty() {
            bail!(
                "Immutable-edit token does not authorize all changed governance files:\n{}\nUse a token that matches the exact file set.",
                unauthorized.join("\n")
            );
        }
    }

    // --- Agent assembly verification ---
    let agent_sources_changed = normalized.iter().any(|f| {
        f.starts_with(".github/agents/identity/") || f.starts_with(".github/agents/context/")
    });
    if agent_sources_changed {
        println!("  Verifying agent assembly sync...");
        crate::agents::verify_assembly(&ws)?;
    }

    // --- Operational YAML structural validation ---
    let yaml_changed: Vec<&String> = normalized
        .iter()
        .filter(|f| {
            f.starts_with("docs/project/")
                && (f.ends_with(".yaml") || f.ends_with(".yml"))
        })
        .collect();
    for yaml_file in &yaml_changed {
        validate_yaml_structural(&ws, yaml_file)?;
    }

    Ok(())
}
