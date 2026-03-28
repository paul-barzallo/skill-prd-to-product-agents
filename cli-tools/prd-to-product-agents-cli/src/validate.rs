use anyhow::{bail, Result};
use clap::Args;
use colored::Colorize;
use prdtp_agents_shared::workspace_paths;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use crate::util;

#[derive(Args)]
pub struct GeneratedArgs {
    /// Workspace directory to validate
    #[arg(long)]
    pub workspace: Option<std::path::PathBuf>,
    /// Record content checksums for freshness tracking
    #[arg(long)]
    pub record_checksums: bool,
}

/// Run all skill-side validations in sequence.
pub fn all(skill_root: &Path) -> Result<()> {
    println!("{}", "--- Running all skill validations ---".cyan());
    let mut errors = 0u32;

    print!("  Package hygiene... ");
    match package_hygiene(skill_root) {
        Ok(()) => println!("{}", "PASS".green()),
        Err(e) => {
            println!("{} {e}", "FAIL".red());
            errors += 1;
        }
    }

    print!("  Platform claims... ");
    match platform_claims(skill_root) {
        Ok(()) => println!("{}", "PASS".green()),
        Err(e) => {
            println!("{} {e}", "FAIL".red());
            errors += 1;
        }
    }

    print!("  Package version metadata... ");
    match version_metadata(skill_root) {
        Ok(()) => println!("{}", "PASS".green()),
        Err(e) => {
            println!("{} {e}", "FAIL".red());
            errors += 1;
        }
    }

    print!("  Template encoding... ");
    match template_encoding(skill_root) {
        Ok(()) => println!("{}", "PASS".green()),
        Err(e) => {
            println!("{} {e}", "FAIL".red());
            errors += 1;
        }
    }

    print!("  Template agent assembly... ");
    match template_agent_consistency(skill_root) {
        Ok(()) => println!("{}", "PASS".green()),
        Err(e) => {
            println!("{} {e}", "FAIL".red());
            errors += 1;
        }
    }

    print!("  Binary bundle integrity... ");
    match binary_bundle_integrity(skill_root) {
        Ok(()) => println!("{}", "PASS".green()),
        Err(e) => {
            println!("{} {e}", "FAIL".red());
            errors += 1;
        }
    }

    print!("  Copilot runtime contract... ");
    match copilot_runtime_contract(skill_root) {
        Ok(()) => println!("{}", "PASS".green()),
        Err(e) => {
            println!("{} {e}", "FAIL".red());
            errors += 1;
        }
    }

    if errors > 0 {
        bail!("{errors} validation(s) failed");
    }
    println!("{}", "All skill validations passed.".green());
    Ok(())
}

fn template_root(skill_root: &Path) -> std::path::PathBuf {
    skill_root.join("templates").join("workspace")
}

fn template_encoding(skill_root: &Path) -> Result<()> {
    let root = template_root(skill_root);
    let prompts_dir = root.join(".github").join("prompts");
    let mut errors = Vec::new();

    for entry in WalkDir::new(&prompts_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let bytes = fs::read(path)?;
        let rel = util::to_relative_posix(path, &root);

        if bytes.windows(2).any(|w| w[0] == 0x0D && w[1] == 0x0A) {
            errors.push(format!("{rel} contains CRLF line endings"));
        }
        let text = String::from_utf8_lossy(&bytes);
        if text.contains('\u{00E2}') || text.contains('\u{00C3}') {
            errors.push(format!("{rel} contains mojibake sequences"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        bail!("Template encoding validation failed:\n  {}", errors.join("\n  "))
    }
}

fn template_agent_consistency(skill_root: &Path) -> Result<()> {
    let root = template_root(skill_root);
    let agents_dir = root.join(".github").join("agents");
    let identity_dir = agents_dir.join("identity");
    let context_dir = agents_dir.join("context");
    let divider = fs::read_to_string(agents_dir.join("CONTEXT_ZONE_DIVIDER.txt"))?
        .replace("\r\n", "\n")
        .trim_end()
        .to_string();
    let shared_context = fs::read_to_string(context_dir.join("shared-context.md"))?
        .replace("\r\n", "\n");

    let mut mismatches = Vec::new();
    for name in workspace_paths::AGENT_NAMES {
        let identity = fs::read_to_string(identity_dir.join(format!("{name}.md")))?
            .replace("\r\n", "\n");
        let context = fs::read_to_string(context_dir.join(format!("{name}.md")))
            .unwrap_or_default()
            .replace("\r\n", "\n");
        let expected = format!(
            "{}\n\n{}\n\n{}\n\n{}\n",
            identity.trim_end(),
            divider,
            shared_context.trim_end(),
            context.trim_end()
        );
        let assembled_path = agents_dir.join(format!("{name}.agent.md"));
        let existing = fs::read_to_string(&assembled_path)?.replace("\r\n", "\n");
        if expected.trim_end() != existing.trim_end() {
            mismatches.push(util::to_relative_posix(&assembled_path, &root));
        }
    }

    if mismatches.is_empty() {
        Ok(())
    } else {
        bail!(
            "Template .agent.md files are out of sync:\n  {}",
            mismatches.join("\n  ")
        )
    }
}

fn binary_bundle_integrity(skill_root: &Path) -> Result<()> {
    verify_checksum_manifest(
        &skill_root.join("bin"),
        "checksums.sha256",
        "skill bootstrap bundle",
    )?;
    verify_checksum_manifest(
        &template_root(skill_root)
            .join(".agents")
            .join("bin")
            .join("prd-to-product-agents"),
        "checksums.sha256",
        "workspace runtime bundle",
    )?;
    Ok(())
}

fn verify_checksum_manifest(bundle_dir: &Path, manifest_name: &str, label: &str) -> Result<()> {
    let manifest_path = bundle_dir.join(manifest_name);
    if !manifest_path.is_file() {
        bail!(
            "{label} is missing checksum manifest: {}",
            manifest_path.display()
        );
    }

    let manifest_raw = fs::read_to_string(&manifest_path)?;
    let mut expected: Vec<(String, String)> = Vec::new();
    for (index, line) in manifest_raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(hash) = parts.next() else {
            bail!("{label} checksum manifest line {} is malformed", index + 1);
        };
        let Some(file_name) = parts.next() else {
            bail!("{label} checksum manifest line {} is malformed", index + 1);
        };
        if parts.next().is_some() {
            bail!("{label} checksum manifest line {} is malformed", index + 1);
        }
        expected.push((hash.to_ascii_lowercase(), file_name.to_string()));
    }

    if expected.is_empty() {
        bail!("{label} checksum manifest is empty");
    }

    let actual_files: Vec<String> = fs::read_dir(bundle_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| name != manifest_name)
        .collect();

    let mut errors = Vec::new();
    for (expected_hash, file_name) in &expected {
        let path = bundle_dir.join(file_name);
        if !path.is_file() {
            errors.push(format!("missing bundled binary: {}", path.display()));
            continue;
        }
        let actual_hash = util::file_hash_bytes(&path)?;
        if actual_hash.to_ascii_lowercase() != *expected_hash {
            errors.push(format!(
                "{} checksum mismatch for {file_name}",
                bundle_dir.display()
            ));
        }
    }

    for file_name in &actual_files {
        if !expected.iter().any(|(_, expected_name)| expected_name == file_name) {
            errors.push(format!(
                "{} contains untracked bundled file {file_name}",
                bundle_dir.display()
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        bail!("binary bundle integrity failed:\n  {}", errors.join("\n  "))
    }
}

fn copilot_runtime_contract(skill_root: &Path) -> Result<()> {
    const FORBIDDEN_TERMS: &[(&str, &str)] = &[
        (
            "github-governance-provisioned",
            "obsolete readiness state is out of contract",
        ),
        (
            "Bootstrap initializes GitHub governance during the skill runtime",
            "bootstrap must not claim remote GitHub governance provisioning",
        ),
        (
            "GitHub.com orchestration parity",
            "GitHub.com parity claims are out of contract",
        ),
        (
            "`workspace-capabilities.yaml` is the hard gate",
            "capabilities must be described as the persisted contract for commands that consult it, not as a universal hard gate",
        ),
        (
            "not idempotent",
            "bootstrap contract must use rerunnable/stable wording instead of a blanket non-idempotent claim",
        ),
        (
            "--reason new-work",
            "handoff reasons must use snake_case values such as new_work",
        ),
        (
            "run any shell command",
            "execute must not be documented as arbitrary shell access",
        ),
    ];

    let mut failures = Vec::new();
    for entry in WalkDir::new(skill_root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let relevant = file_name.ends_with(".md")
            || file_name.ends_with(".instructions.md")
            || file_name.ends_with(".agent.md");
        if !relevant {
            continue;
        }
        let content = fs::read_to_string(path)?;
        let rel = util::to_relative_posix(path, skill_root);
        for (needle, description) in FORBIDDEN_TERMS {
            if content.contains(needle) {
                failures.push(format!("{rel}: {description} (`{needle}`)"));
            }
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        bail!("copilot runtime contract failed:\n  {}", failures.join("\n  "))
    }
}

/// Validate a generated workspace structure.
pub fn generated(_skill_root: &Path, args: GeneratedArgs) -> Result<()> {
    let workspace = args
        .workspace
        .as_deref()
        .unwrap_or(Path::new("."));
    let workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());

    println!(
        "{}",
        format!("--- Validating generated workspace: {} ---", workspace.display()).cyan()
    );

    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // ── Step 1: Required files ───────────────────────────────
    let required_files = workspace_paths::REQUIRED_FILES;

    println!("  Step 1: Required files...");
    for file in required_files {
        if !workspace.join(file).exists() {
            errors.push(format!("missing required file: {file}"));
        }
    }

    // ── Step 2: Agent files ──────────────────────────────────
    let agents = workspace_paths::AGENT_NAMES;

    println!("  Step 2: Agent files...");
    let agents_dir = workspace.join(".github").join("agents");
    for agent in agents {
        let agent_md = agents_dir.join(format!("{agent}.agent.md"));
        if !agent_md.exists() {
            errors.push(format!("missing agent file: .github/agents/{agent}.agent.md"));
            continue;
        }

        // Check for model frontmatter
        let content = fs::read_to_string(&agent_md).unwrap_or_default();
        if !content.contains("model:") {
            errors.push(format!(
                "agent {agent}.agent.md missing model: frontmatter"
            ));
        }

        // Check for CONTEXT ZONE divider
        if !content.contains("CONTEXT ZONE") {
            warnings.push(format!(
                "agent {agent}.agent.md missing CONTEXT ZONE divider"
            ));
        }
    }

    // Check identity and context directories
    let identity_dir = agents_dir.join("identity");
    let context_dir = agents_dir.join("context");
    if identity_dir.is_dir() {
        for agent in agents {
            if !identity_dir.join(format!("{agent}.md")).exists() {
                warnings.push(format!(
                    "missing identity source: .github/agents/identity/{agent}.md"
                ));
            }
        }
    }
    if context_dir.is_dir() {
        for agent in agents {
            if !context_dir.join(format!("{agent}.md")).exists() {
                warnings.push(format!(
                    "missing context source: .github/agents/context/{agent}.md"
                ));
            }
        }
    }

    // ── Step 3: Prompt files ─────────────────────────────────
    println!("  Step 3: Prompt files...");
    let prompts_dir = workspace.join(".github").join("prompts");
    if prompts_dir.is_dir() {
        for entry in WalkDir::new(&prompts_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file()
                    && e.path().extension().map_or(false, |ext| ext == "md")
            })
        {
            let content = fs::read_to_string(entry.path()).unwrap_or_default();
            if !content.starts_with("---") {
                let rel = util::to_relative_posix(entry.path(), &workspace);
                warnings.push(format!("prompt {rel} missing YAML frontmatter"));
            }
        }
    }

    // ── Step 4: YAML structural validation ───────────────────
    println!("  Step 4: YAML structural validation...");
    let yaml_files = workspace_paths::YAML_FILES;

    for yf in yaml_files {
        let yf_path = workspace.join(yf);
        if yf_path.exists() {
            let content = fs::read_to_string(&yf_path).unwrap_or_default();
            // Basic YAML validity: check for tab characters (common error)
            if content.contains('\t') && !yf.contains("manifest") {
                warnings.push(format!("{yf}: contains tab characters (YAML uses spaces)"));
            }
            // Check for unresolved placeholders
            if content.contains("REPLACE_ME") || content.contains("TODO") || content.contains("TBD") {
                warnings.push(format!("{yf}: contains unresolved placeholders"));
            }
        }
    }

    // ── Step 5: SQLite DB ────────────────────────────────────
    println!("  Step 5: SQLite DB...");
    let db_path = workspace.join(".state").join("project_memory.db");
    if !db_path.exists() {
        warnings.push("SQLite DB not initialized (.state/project_memory.db missing)".to_string());
    }

    // ── Step 6: Agent integrity ──────────────────────────────
    println!("  Step 6: Agent integrity...");
    for agent in agents {
        let agent_md = agents_dir.join(format!("{agent}.agent.md"));
        if !agent_md.exists() {
            continue;
        }
        let content = fs::read_to_string(&agent_md).unwrap_or_default();

        // Coordinator check: pm-orchestrator should not list L2 agents directly
        if *agent == "pm-orchestrator" {
            let l2_agents = [
                "backend-developer",
                "frontend-developer",
                "qa-lead",
                "devops-release-engineer",
            ];
            for l2 in &l2_agents {
                if content.contains(&format!("@{l2}")) && !content.contains("skip-level") {
                    warnings.push(format!(
                        "pm-orchestrator.agent.md references L2 agent @{l2} directly"
                    ));
                }
            }
        }
    }

    // ── Step 7: Governance ───────────────────────────────────
    println!("  Step 7: Governance validation...");
    let gov_path = workspace.join(".github").join("github-governance.yaml");
    if gov_path.exists() {
        let gov_content = fs::read_to_string(&gov_path).unwrap_or_default();
        let readiness = util::yaml_scalar_from_str(&gov_content, "readiness.status");
        if let Some(status) = &readiness {
            let valid = [
                "template",
                "bootstrapped",
                "configured",
                "production-ready",
            ];
            if !valid.contains(&status.as_str()) {
                errors.push(format!(
                    "github-governance.yaml: invalid readiness status '{status}'"
                ));
            }
        }
    }

    // ── Record checksums if requested ────────────────────────
    if args.record_checksums {
        let checksum_path = workspace.join(".state").join("content-checksums.json");
        let mut checksums = serde_json::Map::new();
        for file in required_files {
            let fp = workspace.join(file);
            if fp.exists() {
                if let Ok(hash) = util::file_hash(&fp) {
                    checksums.insert(file.to_string(), serde_json::Value::String(hash));
                }
            }
        }
        let json = serde_json::to_string_pretty(&checksums)?;
        util::write_utf8_lf(&checksum_path, &json)?;
    }

    // ── Write validation report ──────────────────────────────
    let validation_path = workspace.join(".state").join("workspace-validation.md");
    let pass = errors.is_empty();
    let mut report = vec![
        "# Workspace Validation".to_string(),
        String::new(),
        format!(
            "- Result: {}",
            if pass { "PASS" } else { "FAIL" }
        ),
        format!("- Errors: {}", errors.len()),
        format!("- Warnings: {}", warnings.len()),
        format!("- Timestamp: {}", util::now_utc()),
    ];

    if !errors.is_empty() {
        report.push(String::new());
        report.push("## Errors".to_string());
        for e in &errors {
            report.push(format!("- {e}"));
        }
    }
    if !warnings.is_empty() {
        report.push(String::new());
        report.push("## Warnings".to_string());
        for w in &warnings {
            report.push(format!("- {w}"));
        }
    }
    util::write_utf8_lf(&validation_path, &(report.join("\n") + "\n"))?;

    // Print summary
    if !errors.is_empty() {
        println!();
        for e in &errors {
            println!("  {} {e}", "ERROR:".red());
        }
    }
    for w in &warnings {
        println!("  {} {w}", "WARN:".yellow());
    }

    println!(
        "\n  Result: {} ({} errors, {} warnings)",
        if pass {
            "PASS".green().to_string()
        } else {
            "FAIL".red().to_string()
        },
        errors.len(),
        warnings.len(),
    );

    if !pass {
        bail!("Workspace validation failed with {} errors", errors.len());
    }
    Ok(())
}

/// Check that the packaged skill contains no runtime artifacts.
pub fn package_hygiene(skill_root: &Path) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();

    // Check for .state/ artifacts in skill root (allow only template ones)
    let state_dir = skill_root.join(".state");
    if state_dir.is_dir() {
        for entry in WalkDir::new(&state_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let rel = util::to_relative_posix(entry.path(), skill_root);
            // Allow template .state files
            if rel.contains("templates/") {
                continue;
            }
            // Allow README.md and memory-schema.sql in template .state
            let name = entry
                .file_name()
                .to_string_lossy()
                .to_string();
            if name == "README.md" || name == "memory-schema.sql" {
                continue;
            }
            errors.push(format!("runtime artifact in skill package: {rel}"));
        }
    }

    // Check for .bootstrap-overlays/
    if skill_root.join(".bootstrap-overlays").is_dir() {
        errors.push(".bootstrap-overlays/ directory present in skill package".to_string());
    }

    // Check for *.new merge marker files
    for entry in WalkDir::new(skill_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let name = entry.file_name().to_string_lossy();
        if name.ends_with(".new")
            && !entry
                .path()
                .to_string_lossy()
                .contains("templates")
        {
            let rel = util::to_relative_posix(entry.path(), skill_root);
            errors.push(format!("pending merge marker: {rel}"));
        }
        if (name.ends_with(".log") || name.ends_with(".tmp") || name.ends_with(".bak"))
            && !entry.path().to_string_lossy().contains("templates")
        {
            let rel = util::to_relative_posix(entry.path(), skill_root);
            errors.push(format!("distributable residue in skill package: {rel}"));
        }
    }

    // Check for project_memory.db outside templates
    for entry in WalkDir::new(skill_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        if entry.file_name() == "project_memory.db"
            && !entry
                .path()
                .to_string_lossy()
                .contains("templates")
        {
            let rel = util::to_relative_posix(entry.path(), skill_root);
            errors.push(format!("SQLite DB in skill package: {rel}"));
        }
    }

    if !errors.is_empty() {
        let msg = errors.join("\n  ");
        bail!("Package hygiene failed:\n  {msg}");
    }
    Ok(())
}

/// Validate platform compatibility claims in documentation.
pub fn platform_claims(skill_root: &Path) -> Result<()> {
    let platform_doc = skill_root.join("references").join("skill-platform-compatibility.md");
    if !platform_doc.exists() {
        // Not mandatory; skip if missing
        println!(
            "  {} references/skill-platform-compatibility.md not found, skipping",
            "SKIP:".yellow()
        );
        return Ok(());
    }

    let content = fs::read_to_string(&platform_doc)?;
    let mut errors: Vec<String> = Vec::new();

    // Check required status tokens
    let required_tokens = ["Verified", "Best-effort"];
    for token in &required_tokens {
        if !content.contains(token) {
            errors.push(format!(
                "platform-compatibility.md missing required status token: {token}"
            ));
        }
    }

    // Reject unsupported "Full" claims
    if content.contains("| Full |") {
        errors.push(
            "platform-compatibility.md contains unsupported 'Full' status claim".to_string(),
        );
    }

    if !errors.is_empty() {
        let msg = errors.join("\n  ");
        bail!("Platform claims validation failed:\n  {msg}");
    }
    Ok(())
}

/// Verify project VERSION metadata is present and readable at the repository root.
pub fn version_metadata(skill_root: &Path) -> Result<()> {
    let version = util::read_version(skill_root)?;

    if version.is_empty() {
        bail!("VERSION is empty");
    }

    println!("  Project VERSION is readable: {version}");
    Ok(())
}
