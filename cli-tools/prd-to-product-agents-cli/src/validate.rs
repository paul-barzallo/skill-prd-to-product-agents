use anyhow::{bail, Context, Result};
use clap::Args;
use colored::Colorize;
use prdtp_agents_shared::workspace_paths;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use uuid::Uuid;
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

const AGENT_LINE_BUDGET: usize = 500;
const FRESHNESS_PATHS: &[&str] = &[
    "docs/project/vision.md",
    "docs/project/scope.md",
    "docs/project/backlog.yaml",
    "docs/project/stakeholders.md",
    "docs/project/glossary.md",
    "docs/project/architecture/overview.md",
    "docs/project/refined-stories.yaml",
    "docs/project/decisions",
];
const TEMPLATE_STATE_ALLOWED_FILES: &[&str] = &["README.md", "memory-schema.sql"];

/// Run the portable skill-package validation surface.
pub fn package(skill_root: &Path) -> Result<()> {
    println!("{}", "--- Running portable package validations ---".cyan());
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

    print!("  Published runtime surface... ");
    match published_runtime_surface(skill_root) {
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
    println!("{}", "Portable package validations passed.".green());
    Ok(())
}

/// Run all maintainer-side validations in sequence.
pub fn all(skill_root: &Path) -> Result<()> {
    println!("{}", "--- Running all maintainer validations ---".cyan());
    let mut errors = 0u32;

    if let Err(e) = package(skill_root) {
        println!("  {}", e);
        errors += 1;
    }

    print!("  Package version metadata... ");
    match version_metadata(skill_root) {
        Ok(()) => println!("{}", "PASS".green()),
        Err(e) => {
            println!("{} {e}", "FAIL".red());
            errors += 1;
        }
    }

    print!("  Maintainer runtime smoke... ");
    match runtime_smoke(skill_root) {
        Ok(()) => println!("{}", "PASS".green()),
        Err(e) => {
            println!("{} {e}", "FAIL".red());
            errors += 1;
        }
    }

    if errors > 0 {
        bail!("{errors} validation(s) failed");
    }
    println!("{}", "All maintainer validations passed.".green());
    Ok(())
}

fn template_root(skill_root: &Path) -> std::path::PathBuf {
    skill_root.join("templates").join("workspace")
}

fn runtime_smoke(skill_root: &Path) -> Result<()> {
    let runtime_binary = current_platform_runtime_binary(skill_root)?;

    let workspace =
        std::env::temp_dir().join(format!("prdtp-runtime-smoke-{}", Uuid::new_v4().simple()));
    fs::create_dir_all(&workspace)?;
    copy_dir_recursive(&template_root(skill_root), &workspace)?;

    let smoke_result = (|| -> Result<()> {
        run_command(
            "git",
            &["init", "-b", "main"],
            Some(&workspace),
            "initializing runtime smoke repository",
        )?;
        run_command(
            "git",
            &["config", "user.name", "Runtime Smoke"],
            Some(&workspace),
            "configuring git user.name for runtime smoke",
        )?;
        run_command(
            "git",
            &["config", "user.email", "runtime-smoke@example.com"],
            Some(&workspace),
            "configuring git user.email for runtime smoke",
        )?;
        run_command(
            "git",
            &["add", "."],
            Some(&workspace),
            "staging template files for runtime smoke",
        )?;
        run_command(
            "git",
            &["commit", "-m", "chore: GH-1 seed runtime smoke workspace"],
            Some(&workspace),
            "creating initial runtime smoke commit",
        )?;

        run_runtime_cli(&runtime_binary, &workspace, &["capabilities", "detect"])?;
        run_runtime_cli(
            &runtime_binary,
            &workspace,
            &[
                "capabilities",
                "authorize",
                "--capability",
                "git",
                "--enabled",
                "true",
                "--source",
                "runtime-smoke",
                "--mode",
                "full",
            ],
        )?;

        let caps_path = workspace
            .join(".github")
            .join("workspace-capabilities.yaml");
        let caps_raw = fs::read_to_string(&caps_path)
            .with_context(|| format!("reading {}", caps_path.display()))?;
        let caps_yaml: serde_yaml::Value = serde_yaml::from_str(&caps_raw)
            .with_context(|| format!("parsing {}", caps_path.display()))?;
        if caps_yaml["capabilities"]["sqlite"]["detected"]["installed"]
            .as_bool()
            .is_none()
        {
            bail!("runtime smoke generated an unreadable capability snapshot");
        }

        run_runtime_cli(
            &runtime_binary,
            &workspace,
            &[
                "governance",
                "configure",
                "--owner",
                "acme-org",
                "--repo",
                "copilot-workspace",
                "--release-gate-login",
                "acme-devops",
                "--reviewer-product",
                "@acme-product",
                "--reviewer-architecture",
                "@acme-arch",
                "--reviewer-tech-lead",
                "@acme-techlead",
                "--reviewer-qa",
                "@acme-qa",
                "--reviewer-devops",
                "@acme-devops",
                "--reviewer-infra",
                "@acme-infra",
                "--reviewer-infra-login",
                "acme-infra",
            ],
        )?;
        run_runtime_cli(&runtime_binary, &workspace, &["validate", "governance"])?;
        run_runtime_cli(
            &runtime_binary,
            &workspace,
            &["validate", "ci", "prompt-tool-contracts"],
        )?;
        run_runtime_cli(
            &runtime_binary,
            &workspace,
            &["validate", "ci", "copilot-runtime-contract"],
        )?;
        run_runtime_cli(
            &runtime_binary,
            &workspace,
            &["agents", "assemble", "--verify"],
        )?;

        let vision_path = workspace.join("docs").join("project").join("vision.md");
        let mut vision = fs::read_to_string(&vision_path)
            .with_context(|| format!("reading {}", vision_path.display()))?;
        vision.push_str("\nRuntime smoke dirty change.\n");
        util::write_utf8_lf(&vision_path, &vision)?;

        let branch_output = run_runtime_cli_capture(
            &runtime_binary,
            &workspace,
            &[
                "git",
                "checkout-task-branch",
                "--role",
                "backend-developer",
                "--issue-id",
                "GH-42",
                "--slug",
                "runtime-smoke",
                "--base",
                "main",
            ],
        )?;
        if branch_output.status.success() {
            bail!("runtime smoke expected checkout-task-branch to fail on a dirty worktree");
        }
        let branch_combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&branch_output.stdout),
            String::from_utf8_lossy(&branch_output.stderr)
        );
        if !branch_combined.contains("Refusing to switch branches with local changes present") {
            bail!(
                "runtime smoke saw an unexpected branch failure: {}",
                branch_combined.trim()
            );
        }

        let readiness_output =
            run_runtime_cli_capture(&runtime_binary, &workspace, &["validate", "readiness"])?;
        if readiness_output.status.success() {
            bail!("runtime smoke expected validate readiness to fail until production-ready");
        }
        let readiness_combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&readiness_output.stdout),
            String::from_utf8_lossy(&readiness_output.stderr)
        );
        if !readiness_combined.contains("production-ready") {
            bail!(
                "runtime smoke readiness failure did not explain the production-ready requirement: {}",
                readiness_combined.trim()
            );
        }

        Ok(())
    })();

    let cleanup_result = fs::remove_dir_all(&workspace);
    if let Err(error) = cleanup_result {
        tracing::warn!(path = %workspace.display(), error = %error, "failed to clean runtime smoke workspace");
    }

    smoke_result
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<()> {
    for entry in WalkDir::new(source)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        let relative = entry.path().strip_prefix(source)?;
        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &target).with_context(|| {
                format!("copying {} -> {}", entry.path().display(), target.display())
            })?;
        }
    }
    Ok(())
}

fn run_command(name: &str, args: &[&str], cwd: Option<&Path>, context: &str) -> Result<()> {
    let mut command = Command::new(name);
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command.output().with_context(|| context.to_string())?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let combined = if stderr.is_empty() {
        stdout
    } else if stdout.is_empty() {
        stderr
    } else {
        format!("{stdout}\n{stderr}")
    };
    bail!("{context}: {}", combined.trim());
}

fn run_runtime_cli(runtime_binary: &Path, workspace: &Path, args: &[&str]) -> Result<()> {
    let output = run_runtime_cli_capture(runtime_binary, workspace, args)?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let combined = if stderr.is_empty() {
        stdout
    } else if stdout.is_empty() {
        stderr
    } else {
        format!("{stdout}\n{stderr}")
    };
    bail!(
        "runtime smoke command failed (`{}`): {}",
        args.join(" "),
        combined.trim()
    );
}

fn run_runtime_cli_capture(
    runtime_binary: &Path,
    workspace: &Path,
    args: &[&str],
) -> Result<Output> {
    let mut command = Command::new(runtime_binary);
    command
        .args(["--workspace", &workspace.to_string_lossy()])
        .args(args);
    let output = command
        .output()
        .with_context(|| format!("running runtime smoke command `{}`", args.join(" ")))?;
    Ok(output)
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
        bail!(
            "Template encoding validation failed:\n  {}",
            errors.join("\n  ")
        )
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
    let shared_context =
        fs::read_to_string(context_dir.join("shared-context.md"))?.replace("\r\n", "\n");

    let mut mismatches = Vec::new();
    for name in workspace_paths::AGENT_NAMES {
        let identity =
            fs::read_to_string(identity_dir.join(format!("{name}.md")))?.replace("\r\n", "\n");
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
    crate::bundle::verify_packaged_bundle_integrity(skill_root)?;
    Ok(())
}

fn published_runtime_surface(skill_root: &Path) -> Result<()> {
    let runtime_binary = current_platform_runtime_binary(skill_root)?;
    let global_help = run_binary_capture(&runtime_binary, &["--help"]).with_context(|| {
        format!(
            "reading published runtime help from {}",
            runtime_binary.display()
        )
    })?;
    assert_help_absent(
        &global_help,
        "github",
        "published runtime must not expose the github command",
    )?;

    let governance_help = run_binary_capture(&runtime_binary, &["governance", "--help"])?;
    assert_help_absent(
        &governance_help,
        "promote-enterprise-readiness",
        "published runtime must not expose local enterprise readiness promotion",
    )?;

    let audit_help = run_binary_capture(&runtime_binary, &["audit", "--help"])?;
    assert_help_absent(
        &audit_help,
        "export",
        "published runtime must not expose audit export in the public skill contract",
    )?;

    Ok(())
}

fn current_platform_runtime_binary(skill_root: &Path) -> Result<PathBuf> {
    let base = template_root(skill_root)
        .join(".agents")
        .join("bin")
        .join("prd-to-product-agents");
    let candidate = if cfg!(target_os = "windows") {
        base.join("prdtp-agents-functions-cli-windows-x64.exe")
    } else if cfg!(target_os = "macos") {
        base.join("prdtp-agents-functions-cli-darwin-arm64")
    } else {
        base.join("prdtp-agents-functions-cli-linux-x64")
    };
    if !candidate.is_file() {
        bail!(
            "published runtime binary for this platform is missing: {}",
            candidate.display()
        );
    }
    Ok(candidate)
}

fn run_binary_capture(binary: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new(binary)
        .args(args)
        .output()
        .with_context(|| format!("running {}", binary.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!(
            "published runtime command failed (`{} {}`): {}",
            binary.display(),
            args.join(" "),
            stderr
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn assert_help_absent(help_output: &str, needle: &str, message: &str) -> Result<()> {
    if help_output.contains(needle) {
        bail!("{message}");
    }
    Ok(())
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
        (
            "Execution layer: GitHub Issues, GitHub Project, and Pull Requests",
            "board snapshots must not claim GitHub Project as part of the current execution layer",
        ),
        (
            "GitHub Issues, GitHub Projects, and Pull Requests are the execution layer",
            "docs must not describe GitHub Project as part of the current execution layer",
        ),
        (
            "GitHub Project board state",
            "GitHub Project must not be treated as current operational state",
        ),
        (
            "promote-enterprise-readiness",
            "published runtime docs must not reference hidden enterprise promotion helpers",
        ),
        (
            "audit export",
            "published runtime docs must not reference audit export in the public skill contract",
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
        bail!(
            "copilot runtime contract failed:\n  {}",
            failures.join("\n  ")
        )
    }
}

/// Validate a generated workspace structure.
pub fn generated(_skill_root: &Path, args: GeneratedArgs) -> Result<()> {
    let workspace = args.workspace.as_deref().unwrap_or(Path::new("."));
    let workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());

    println!(
        "{}",
        format!(
            "--- Validating generated workspace: {} ---",
            workspace.display()
        )
        .cyan()
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
            errors.push(format!(
                "missing agent file: .github/agents/{agent}.agent.md"
            ));
            continue;
        }

        // Check for model frontmatter
        let content = fs::read_to_string(&agent_md).unwrap_or_default();
        if !content.contains("model:") {
            errors.push(format!("agent {agent}.agent.md missing model: frontmatter"));
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
                e.file_type().is_file() && e.path().extension().map_or(false, |ext| ext == "md")
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
            if content.contains("REPLACE_ME") || content.contains("TODO") || content.contains("TBD")
            {
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

        let line_count = content.lines().count();
        if line_count > AGENT_LINE_BUDGET {
            warnings.push(format!(
                "SIZE-BUDGET: .github/agents/{agent}.agent.md is {line_count} lines (budget {AGENT_LINE_BUDGET})"
            ));
        }
    }

    // ── Step 7: Governance ───────────────────────────────────
    println!("  Step 7: Governance validation...");
    let gov_path = workspace.join(".github").join("github-governance.yaml");
    if gov_path.exists() {
        let gov_content = fs::read_to_string(&gov_path).unwrap_or_default();
        let readiness = util::yaml_scalar_from_str(&gov_content, "readiness.status");
        if let Some(status) = &readiness {
            let valid = ["template", "bootstrapped", "configured", "production-ready"];
            if !valid.contains(&status.as_str()) {
                errors.push(format!(
                    "github-governance.yaml: invalid readiness status '{status}'"
                ));
            }
        }
    }

    // ── Record checksums if requested ────────────────────────
    println!("  Step 8: Context freshness...");
    let current_checksums = collect_context_checksums(&workspace)?;
    let (baseline_source, baseline_checksums) = read_context_checksum_baseline(&workspace)?;
    if let Some(source) = baseline_source.as_ref() {
        if source.ends_with("content-checksums.json") {
            warnings.push(
                "freshness baseline loaded from legacy .state/content-checksums.json; rerun with --record-checksums to migrate to .state/context-checksums.json".to_string(),
            );
        }

        for changed in diff_context_checksums(&baseline_checksums, &current_checksums) {
            warnings.push(format!(
                "context freshness: canonical file changed since baseline: {changed}"
            ));
        }
    }

    if args.record_checksums {
        let checksum_path = workspace.join(".state").join("context-checksums.json");
        let json = serde_json::to_string_pretty(&current_checksums)?;
        util::write_utf8_lf(&checksum_path, &json)?;
    }

    // ── Write validation report ──────────────────────────────
    let validation_path = workspace.join(".state").join("workspace-validation.md");
    let pass = errors.is_empty();
    let mut report = vec![
        "# Workspace Validation".to_string(),
        String::new(),
        format!("- Result: {}", if pass { "PASS" } else { "FAIL" }),
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

fn collect_context_checksums(
    workspace: &Path,
) -> Result<serde_json::Map<String, serde_json::Value>> {
    let mut entries: Vec<(String, String)> = Vec::new();

    for relative in FRESHNESS_PATHS {
        let full_path = workspace.join(relative);
        if !full_path.exists() {
            continue;
        }

        if full_path.is_dir() {
            let mut dir_entries = WalkDir::new(&full_path)
                .into_iter()
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_type().is_file())
                .map(|entry| entry.into_path())
                .collect::<Vec<_>>();
            dir_entries.sort();

            for path in dir_entries {
                let rel = util::to_relative_posix(&path, workspace);
                entries.push((rel, util::file_hash(&path)?));
            }
        } else {
            let rel = util::to_relative_posix(&full_path, workspace);
            entries.push((rel, util::file_hash(&full_path)?));
        }
    }

    entries.sort_by(|left, right| left.0.cmp(&right.0));

    let mut checksums = serde_json::Map::new();
    for (path, hash) in entries {
        checksums.insert(path, serde_json::Value::String(hash));
    }
    Ok(checksums)
}

fn read_context_checksum_baseline(
    workspace: &Path,
) -> Result<(Option<String>, serde_json::Map<String, serde_json::Value>)> {
    let preferred = workspace.join(".state").join("context-checksums.json");
    if preferred.is_file() {
        let content = fs::read_to_string(&preferred)
            .with_context(|| format!("reading {}", preferred.display()))?;
        let parsed = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&content)
            .with_context(|| format!("parsing {}", preferred.display()))?;
        return Ok((Some(preferred.to_string_lossy().to_string()), parsed));
    }

    let legacy = workspace.join(".state").join("content-checksums.json");
    if legacy.is_file() {
        let content =
            fs::read_to_string(&legacy).with_context(|| format!("reading {}", legacy.display()))?;
        let parsed = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&content)
            .with_context(|| format!("parsing {}", legacy.display()))?;
        return Ok((Some(legacy.to_string_lossy().to_string()), parsed));
    }

    Ok((None, serde_json::Map::new()))
}

fn diff_context_checksums(
    baseline: &serde_json::Map<String, serde_json::Value>,
    current: &serde_json::Map<String, serde_json::Value>,
) -> Vec<String> {
    let mut changed = Vec::new();

    for (path, current_hash) in current {
        let baseline_hash = baseline.get(path).and_then(|value| value.as_str());
        let current_hash = current_hash.as_str();
        if baseline_hash != current_hash {
            changed.push(path.clone());
        }
    }

    changed.sort();
    changed
}

/// Check that the packaged skill contains no runtime artifacts.
pub fn package_hygiene(skill_root: &Path) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();

    collect_disallowed_state_files(&skill_root.join(".state"), skill_root, &[], &mut errors);
    collect_disallowed_state_files(
        &template_root(skill_root).join(".state"),
        skill_root,
        TEMPLATE_STATE_ALLOWED_FILES,
        &mut errors,
    );

    // Check for .bootstrap-overlays/
    if skill_root.join(".bootstrap-overlays").is_dir() {
        errors.push(".bootstrap-overlays/ directory present in skill package".to_string());
    }

    // Check for generic distributable residue anywhere in the packaged skill.
    for entry in WalkDir::new(skill_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let name = entry.file_name().to_string_lossy();
        if name.ends_with(".new") {
            let rel = util::to_relative_posix(entry.path(), skill_root);
            errors.push(format!("pending merge marker: {rel}"));
        }
        if name.ends_with(".log") || name.ends_with(".tmp") || name.ends_with(".bak") {
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
            && !entry.path().to_string_lossy().contains("templates")
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

fn collect_disallowed_state_files(
    state_dir: &Path,
    skill_root: &Path,
    allowed_files: &[&str],
    errors: &mut Vec<String>,
) {
    if !state_dir.is_dir() {
        return;
    }

    for entry in WalkDir::new(state_dir)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let rel = util::to_relative_posix(entry.path(), skill_root);
        let allowed = entry
            .path()
            .strip_prefix(state_dir)
            .ok()
            .and_then(|path| path.to_str())
            .map(|path| path.replace('\\', "/"))
            .map(|path| entry.file_type().is_file() && allowed_files.contains(&path.as_str()))
            .unwrap_or(false);

        if !allowed {
            let rel = if entry.file_type().is_dir() {
                format!("{rel}/")
            } else {
                rel
            };
            errors.push(format!("runtime artifact in skill package: {rel}"));
        }
    }
}

/// Validate platform compatibility claims in documentation.
pub fn platform_claims(skill_root: &Path) -> Result<()> {
    let platform_doc = skill_root
        .join("references")
        .join("skill-platform-compatibility.md");
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
        errors
            .push("platform-compatibility.md contains unsupported 'Full' status claim".to_string());
    }

    if !errors.is_empty() {
        let msg = errors.join("\n  ");
        bail!("Platform claims validation failed:\n  {msg}");
    }
    Ok(())
}

/// Verify project VERSION metadata when running from a repository root.
pub fn version_metadata(skill_root: &Path) -> Result<()> {
    let Some(version) = util::read_version_if_present(skill_root)? else {
        println!("  Project VERSION metadata unavailable outside repository root; skipped");
        return Ok(());
    };

    if version.is_empty() {
        bail!("VERSION is empty");
    }

    println!("  Project VERSION is readable: {version}");
    Ok(())
}
