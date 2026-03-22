use anyhow::{bail, Context, Result};
use clap::Args;
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::util;

#[derive(Args)]
pub struct SmokeArgs {
    /// Target workspace directory for smoke testing
    #[arg(long)]
    pub target: Option<PathBuf>,
}

#[derive(Args)]
pub struct MarkdownArgs {
    /// Override markdownlint config path
    #[arg(long)]
    pub config: Option<PathBuf>,
    /// Markdown path or glob relative to --skill-root
    #[arg(long = "path")]
    pub paths: Vec<String>,
}

struct SmokeWorkspace {
    root: PathBuf,
    cleanup: bool,
}

impl Drop for SmokeWorkspace {
    fn drop(&mut self) {
        if self.cleanup {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

fn summary(title: &str, passed: u32, failed: u32, skipped: u32) {
    println!();
    println!("{}", "═══════════════════════════════════════".cyan());
    println!("  {title}");
    println!(
        "  Passed: {}  Failed: {}  Skipped: {}",
        format!("{passed}").green(),
        if failed > 0 {
            format!("{failed}").red().to_string()
        } else {
            format!("{failed}").green().to_string()
        },
        if skipped > 0 {
            format!("{skipped}").yellow().to_string()
        } else {
            format!("{skipped}").green().to_string()
        }
    );
    println!("{}", "═══════════════════════════════════════".cyan());
}

fn format_status(code: Option<i32>) -> String {
    code.map(|value| value.to_string())
        .unwrap_or_else(|| "terminated by signal".to_string())
}

fn run_executable(
    executable: &Path,
    args: &[String],
    cwd: Option<&Path>,
) -> Result<util::CommandResult> {
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    util::executable_capture(executable, &arg_refs, cwd)
}

fn run_command(name: &str, args: &[String], cwd: Option<&Path>) -> Result<util::CommandResult> {
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    util::command_capture(name, &arg_refs, cwd)
}

fn create_smoke_workspace(target: Option<PathBuf>) -> Result<SmokeWorkspace> {
    if let Some(target) = target {
        fs::create_dir_all(&target)
            .with_context(|| format!("creating smoke target {}", target.display()))?;
        let root = target.canonicalize().unwrap_or(target);
        return Ok(SmokeWorkspace {
            root,
            cleanup: false,
        });
    }

    let root = std::env::temp_dir().join(format!("prd-cli-smoke-{}", Uuid::new_v4().simple()));
    fs::create_dir_all(&root)
        .with_context(|| format!("creating smoke temp dir {}", root.display()))?;
    Ok(SmokeWorkspace {
        root,
        cleanup: true,
    })
}

fn repo_root_for_skill(skill_root: &Path) -> PathBuf {
    let canonical = skill_root
        .canonicalize()
        .unwrap_or_else(|_| skill_root.to_path_buf());
    let candidate = canonical
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf);
    candidate.unwrap_or(canonical)
}

fn candidate_roots(skill_root: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut push_unique = |candidate: PathBuf| {
        if !roots.iter().any(|existing| existing == &candidate) {
            roots.push(candidate);
        }
    };

    let resolved_skill_root = skill_root
        .canonicalize()
        .unwrap_or_else(|_| skill_root.to_path_buf());
    push_unique(resolved_skill_root.clone());
    for ancestor in resolved_skill_root.ancestors().take(6) {
        push_unique(ancestor.to_path_buf());
    }
    if let Ok(current_exe) = std::env::current_exe() {
        for ancestor in current_exe.ancestors().take(8) {
            push_unique(ancestor.to_path_buf());
        }
    }

    roots
}

fn find_runtime_cli(skill_root: &Path) -> Option<PathBuf> {
    let (build_binaries, packaged_binaries) = if cfg!(target_os = "windows") {
        (
            vec![
                "cli-tools/prdtp-agents-functions-cli/target/debug/prdtp-agents-functions-cli.exe",
                "cli-tools/prdtp-agents-functions-cli/target/release/prdtp-agents-functions-cli.exe",
                "prdtp-agents-functions-cli/target/debug/prdtp-agents-functions-cli.exe",
                "prdtp-agents-functions-cli/target/release/prdtp-agents-functions-cli.exe",
            ],
            vec![
                ".agents/bin/prd-to-product-agents/prdtp-agents-functions-cli-windows-x64.exe",
                ".agents/skills/prd-to-product-agents/templates/workspace/.agents/bin/prd-to-product-agents/prdtp-agents-functions-cli-windows-x64.exe",
            ],
        )
    } else {
        (
            vec![
                "cli-tools/prdtp-agents-functions-cli/target/debug/prdtp-agents-functions-cli",
                "cli-tools/prdtp-agents-functions-cli/target/release/prdtp-agents-functions-cli",
                "prdtp-agents-functions-cli/target/debug/prdtp-agents-functions-cli",
                "prdtp-agents-functions-cli/target/release/prdtp-agents-functions-cli",
            ],
            vec![
                ".agents/bin/prd-to-product-agents/prdtp-agents-functions-cli-linux-x64",
                ".agents/bin/prd-to-product-agents/prdtp-agents-functions-cli-darwin-arm64",
                ".agents/skills/prd-to-product-agents/templates/workspace/.agents/bin/prd-to-product-agents/prdtp-agents-functions-cli-linux-x64",
                ".agents/skills/prd-to-product-agents/templates/workspace/.agents/bin/prd-to-product-agents/prdtp-agents-functions-cli-darwin-arm64",
            ],
        )
    };

    let roots = candidate_roots(skill_root);
    for relative in &build_binaries {
        for root in &roots {
            let candidate = root.join(relative);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    for relative in &packaged_binaries {
        for root in &roots {
            let candidate = root.join(relative);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

fn find_skill_cli(skill_root: &Path) -> Option<PathBuf> {
    let (build_binaries, packaged_binaries) = if cfg!(target_os = "windows") {
        (
            vec![
                "cli-tools/prd-to-product-agents-cli/target/debug/prd-to-product-agents-cli.exe",
                "cli-tools/prd-to-product-agents-cli/target/release/prd-to-product-agents-cli.exe",
                "prd-to-product-agents-cli/target/debug/prd-to-product-agents-cli.exe",
                "prd-to-product-agents-cli/target/release/prd-to-product-agents-cli.exe",
            ],
            vec!["bin/prd-to-product-agents-cli-windows-x64.exe"],
        )
    } else {
        (
            vec![
                "cli-tools/prd-to-product-agents-cli/target/debug/prd-to-product-agents-cli",
                "cli-tools/prd-to-product-agents-cli/target/release/prd-to-product-agents-cli",
                "prd-to-product-agents-cli/target/debug/prd-to-product-agents-cli",
                "prd-to-product-agents-cli/target/release/prd-to-product-agents-cli",
            ],
            vec![
                "bin/prd-to-product-agents-cli-linux-x64",
                "bin/prd-to-product-agents-cli-darwin-arm64",
            ],
        )
    };

    let roots = candidate_roots(skill_root);
    for relative in &build_binaries {
        for root in &roots {
            let candidate = root.join(relative);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    for relative in &packaged_binaries {
        for root in &roots {
            let candidate = root.join(relative);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

pub fn markdown(skill_root: &Path, args: MarkdownArgs) -> Result<()> {
    let config_path = args.config.unwrap_or_else(|| {
        skill_root
            .join("templates")
            .join("workspace")
            .join(".markdownlint.json")
    });
    if !config_path.is_file() {
        bail!("markdownlint config not found: {}", config_path.display());
    }

    let mut command_args = vec![
        "--yes".to_string(),
        "markdownlint-cli".to_string(),
        "--config".to_string(),
        config_path.display().to_string(),
    ];
    if args.paths.is_empty() {
        command_args.push("**/*.md".to_string());
    } else {
        command_args.extend(args.paths);
    }

    println!("{}", "═══════════════════════════════════════".cyan());
    println!("{}", "  Markdown Tests — skill-dev-cli".cyan());
    println!("{}", "═══════════════════════════════════════".cyan());

    let result = if cfg!(target_os = "windows") {
        let mut shell_args = vec!["/C".to_string(), "npx".to_string()];
        shell_args.extend(command_args.clone());
        run_command("cmd", &shell_args, Some(skill_root))
            .context("running npx markdownlint-cli via cmd")?
    } else {
        if !util::command_exists("npx") {
            bail!("npx not found; install Node.js to run markdownlint-cli");
        }
        run_command("npx", &command_args, Some(skill_root))?
    };
    if !result.stdout.is_empty() {
        println!("{}", result.stdout);
    }
    if !result.stderr.is_empty() {
        eprintln!("{}", result.stderr);
    }

    if !result.success {
        bail!(
            "markdownlint-cli failed with exit code {}",
            format_status(result.code)
        );
    }

    println!("{}", "PASS: markdownlint-cli".green());
    Ok(())
}

pub fn smoke(skill_root: &Path, args: SmokeArgs) -> Result<()> {
    let workspace = create_smoke_workspace(args.target)?;
    let target = workspace.root.as_path();
    let runtime_cli = find_runtime_cli(skill_root);
    let skill_cli = find_skill_cli(skill_root)
        .context("skill CLI not found; build prd-to-product-agents-cli first")?;
    let skill_root_display = skill_root.display().to_string();
    let target_display = target.display().to_string();

    println!("{}", "═══════════════════════════════════════".cyan());
    println!("{}", "  Smoke Tests — skill-dev-cli".cyan());
    println!("{}", "═══════════════════════════════════════".cyan());
    println!("  Skill root: {}", skill_root.display());
    println!("  Target:     {}", target.display());

    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut skipped = 0u32;
    let runtime_args_base = vec!["--workspace".to_string(), target_display.clone()];

    print!("  [0] Runtime CLI availability... ");
    if runtime_cli.is_some() {
        println!("{}", "PASS".green());
        passed += 1;
    } else {
        println!("{}", "FAIL (runtime CLI not found — required for governance checks)".red());
        failed += 1;
    }

    print!("  [1] Preflight check... ");
    let preflight_args = vec![
        "--skill-root".to_string(),
        skill_root_display.clone(),
        "bootstrap".to_string(),
        "workspace".to_string(),
        "--target".to_string(),
        target_display.clone(),
        "--preflight-only".to_string(),
    ];
    let preflight = run_executable(&skill_cli, &preflight_args, None)?;
    if preflight.success {
        println!("{}", "PASS".green());
        passed += 1;
    } else {
        println!("{} (exit {})", "FAIL".red(), format_status(preflight.code));
        failed += 1;
    }

    print!("  [2] Dry run... ");
    let state_preexisting = target.join(".state").exists();
    let dry_run_args = vec![
        "--skill-root".to_string(),
        skill_root_display.clone(),
        "bootstrap".to_string(),
        "workspace".to_string(),
        "--target".to_string(),
        target_display.clone(),
        "--dry-run".to_string(),
    ];
    let dry_run = run_executable(&skill_cli, &dry_run_args, None)?;
    if dry_run.success {
        println!("{}", "PASS".green());
        passed += 1;
    } else {
        println!("{} (exit {})", "FAIL".red(), format_status(dry_run.code));
        failed += 1;
    }

    print!("  [3] Dry run preview... ");
    let dry_run_output = dry_run.combined_output();
    if dry_run_output.contains("Would create") || dry_run_output.contains("Dry run") {
        println!("{}", "PASS".green());
        passed += 1;
    } else {
        println!("{}", "FAIL (missing preview output)".red());
        failed += 1;
    }

    print!("  [4] Dry run wrote no files... ");
    if !state_preexisting && !target.join(".state").exists() {
        println!("{}", "PASS".green());
        passed += 1;
    } else if state_preexisting {
        println!("{}", "SKIP (target already had .state)".yellow());
        skipped += 1;
    } else {
        println!("{}", "FAIL".red());
        failed += 1;
    }

    print!("  [5] Full bootstrap... ");
    let bootstrap_args = vec![
        "--skill-root".to_string(),
        skill_root_display.clone(),
        "bootstrap".to_string(),
        "workspace".to_string(),
        "--target".to_string(),
        target_display.clone(),
        "--skip-git".to_string(),
    ];
    let bootstrap = run_executable(&skill_cli, &bootstrap_args, None)?;
    if bootstrap.success {
        println!("{}", "PASS".green());
        passed += 1;
    } else {
        println!("{} (exit {})", "FAIL".red(), format_status(bootstrap.code));
        failed += 1;
    }

    print!("  [6] Bootstrap created AGENTS.md... ");
    if target.join("AGENTS.md").is_file() {
        println!("{}", "PASS".green());
        passed += 1;
    } else {
        println!("{}", "FAIL".red());
        failed += 1;
    }

    print!("  [7] Bootstrap report exists... ");
    if target.join(".state").join("bootstrap-report.md").is_file() {
        println!("{}", "PASS".green());
        passed += 1;
    } else {
        println!("{}", "FAIL".red());
        failed += 1;
    }

    print!("  [8] Workspace capabilities exist... ");
    if target.join(".github").join("workspace-capabilities.yaml").is_file() {
        println!("{}", "PASS".green());
        passed += 1;
    } else {
        println!("{}", "FAIL".red());
        failed += 1;
    }

    print!("  [8b] Validate generated workspace... ");
    let validate_args = vec![
        "--skill-root".to_string(),
        skill_root_display.clone(),
        "validate".to_string(),
        "generated".to_string(),
        "--workspace".to_string(),
        target_display.clone(),
    ];
    let validate_result = run_executable(&skill_cli, &validate_args, None)?;
    if validate_result.success {
        println!("{}", "PASS".green());
        passed += 1;
    } else {
        println!(
            "{} (exit {})\n{}",
            "FAIL".red(),
            format_status(validate_result.code),
            validate_result.combined_output()
        );
        failed += 1;
    }

    print!("  [8c] Validate readiness stays blocked... ");
    let mut readiness_args = runtime_args_base.clone();
    readiness_args.extend(["validate".to_string(), "readiness".to_string()]);
    let readiness_result = run_executable(runtime_cli.as_ref().unwrap(), &readiness_args, None)?;
    if !readiness_result.success {
        println!("{}", "PASS".green());
        passed += 1;
    } else {
        println!("{}", "FAIL (bootstrapped workspace unexpectedly reported ready)".red());
        failed += 1;
    }

    print!("  [9] VERSION consistency... ");
    let version_args = vec![
        "--skill-root".to_string(),
        skill_root_display.clone(),
        "validate".to_string(),
        "skill-version".to_string(),
    ];
    let version_result = run_executable(&skill_cli, &version_args, None)?;
    if version_result.success {
        println!("{}", "PASS".green());
        passed += 1;
    } else {
        println!("{} {}", "FAIL".red(), version_result.combined_output());
        failed += 1;
    }

    print!("  [10] Package hygiene... ");
    let hygiene_args = vec![
        "--skill-root".to_string(),
        skill_root_display.clone(),
        "validate".to_string(),
        "package-hygiene".to_string(),
    ];
    let hygiene_result = run_executable(&skill_cli, &hygiene_args, None)?;
    if hygiene_result.success {
        println!("{}", "PASS".green());
        passed += 1;
    } else {
        println!("{} {}", "FAIL".red(), hygiene_result.combined_output());
        failed += 1;
    }

    print!("  [11] Required template files... ");
    let template_root = skill_root.join("templates").join("workspace");
    let required_templates = ["AGENTS.md", ".instructions.md", ".gitignore", ".gitattributes"];
    let template_ok = template_root.is_dir()
        && required_templates.iter().all(|path| template_root.join(path).exists());
    if template_ok {
        println!("{}", "PASS".green());
        passed += 1;
    } else {
        println!("{}", "FAIL".red());
        failed += 1;
    }

    if let Some(runtime_cli) = runtime_cli {
        print!("  [12] Agent assembly verify... ");
        let mut args = runtime_args_base.clone();
        args.extend(["agents".to_string(), "assemble".to_string(), "--verify".to_string()]);
        let result = run_executable(&runtime_cli, &args, None)?;
        if result.success {
            println!("{}", "PASS".green());
            passed += 1;
        } else {
            println!("{} (exit {})", "FAIL".red(), format_status(result.code));
            failed += 1;
        }

        print!("  [13] Database init... ");
        let mut args = runtime_args_base.clone();
        args.extend(["database".to_string(), "init".to_string()]);
        let result = run_executable(&runtime_cli, &args, None)?;
        if result.success {
            println!("{}", "PASS".green());
            passed += 1;
        } else {
            println!("{} (exit {})", "FAIL".red(), format_status(result.code));
            failed += 1;
        }

        print!("  [14] Encoding validation... ");
        let mut args = runtime_args_base.clone();
        args.extend(["validate".to_string(), "encoding".to_string()]);
        let result = run_executable(&runtime_cli, &args, None)?;
        if result.success {
            println!("{}", "PASS".green());
            passed += 1;
        } else {
            println!("{} (exit {})\n{}", "FAIL".red(), format_status(result.code), result.combined_output());
            failed += 1;
        }

        print!("  [15] Git hook installation... ");
        if util::command_exists("git") {
            let git_init = run_command("git", &["init".to_string()], Some(target))?;
            let git_name = run_command(
                "git",
                &[
                    "config".to_string(),
                    "user.name".to_string(),
                    "Smoke Test".to_string(),
                ],
                Some(target),
            )?;
            let git_email = run_command(
                "git",
                &[
                    "config".to_string(),
                    "user.email".to_string(),
                    "smoke@example.com".to_string(),
                ],
                Some(target),
            )?;

            let mut args = runtime_args_base.clone();
            args.extend(["git".to_string(), "install-hooks".to_string()]);
            let hook_result = run_executable(&runtime_cli, &args, None)?;
            let hook_path = target.join(".git").join("hooks").join("pre-commit");
            let hook_ok = if hook_path.is_file() {
                let hook_content = fs::read_to_string(&hook_path).unwrap_or_default();
                hook_content.contains("BASE_DIR=\"$REPO_ROOT/.agents/bin/prd-to-product-agents\"")
                    && hook_content.contains("git pre-commit-validate")
                    && hook_content.contains("--workspace \"$REPO_ROOT\"")
            } else {
                false
            };

            if git_init.success && git_name.success && git_email.success && hook_result.success && hook_ok {
                println!("{}", "PASS".green());
                passed += 1;
            } else {
                println!("{}", "FAIL".red());
                failed += 1;
            }
        } else {
            println!("{}", "SKIP (git not found)".yellow());
            skipped += 1;
        }
    } else {
        print!("  [12] Runtime validation... ");
        println!("{}", "FAIL (runtime CLI not found)".red());
        failed += 3;
    }

    summary("Smoke Tests", passed, failed, skipped);

    if failed > 0 {
        bail!("{failed} smoke test(s) failed");
    }
    Ok(())
}

pub fn unit(_skill_root: &Path) -> Result<()> {
    println!("{}", "═══════════════════════════════════════".cyan());
    println!("{}", "  Unit Tests — skill-dev-cli".cyan());
    println!("{}", "═══════════════════════════════════════".cyan());

    let mut passed = 0u32;
    let mut failed = 0u32;

    print!("  [1] YAML scalar parsing... ");
    {
        let yaml = "key: value\nnested:\n  child: deep\n";
        let v1 = util::yaml_scalar_from_str(yaml, "key");
        let v2 = util::yaml_scalar_from_str(yaml, "nested.child");
        if v1.as_deref() == Some("value") && v2.as_deref() == Some("deep") {
            println!("{}", "PASS".green());
            passed += 1;
        } else {
            println!("{} got {:?}, {:?}", "FAIL".red(), v1, v2);
            failed += 1;
        }
    }

    print!("  [2] YAML bool parsing... ");
    {
        let yaml = "enabled: true\ndisabled: false\n";
        let tmp = std::env::temp_dir().join("skill-dev-cli-test-yaml-bool.yaml");
        std::fs::write(&tmp, yaml)?;
        let v1 = util::yaml_bool(&tmp, "enabled", false);
        let v2 = util::yaml_bool(&tmp, "disabled", true);
        let _ = std::fs::remove_file(&tmp);
        if v1 && !v2 {
            println!("{}", "PASS".green());
            passed += 1;
        } else {
            println!("{} got {v1}, {v2}", "FAIL".red());
            failed += 1;
        }
    }

    print!("  [3] LF normalization... ");
    {
        let input = "line1\r\nline2\rline3\n";
        let result = util::normalize_lf(input);
        if result == "line1\nline2\nline3\n" {
            println!("{}", "PASS".green());
            passed += 1;
        } else {
            println!("{} got {:?}", "FAIL".red(), result);
            failed += 1;
        }
    }

    print!("  [4] SHA-256 hashing... ");
    {
        let hash = util::sha256_hex("hello");
        if hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
            println!("{}", "PASS".green());
            passed += 1;
        } else {
            println!("{} got {:?}", "FAIL".red(), hash);
            failed += 1;
        }
    }

    print!("  [5] OS detection... ");
    {
        let os = util::detect_os();
        if !os.is_empty() {
            println!("{} ({})", "PASS".green(), os);
            passed += 1;
        } else {
            println!("{}", "FAIL".red());
            failed += 1;
        }
    }

    print!("  [6] Relative posix paths... ");
    {
        let base = Path::new("/home/user/project");
        let full = Path::new("/home/user/project/src/main.rs");
        let result = util::to_relative_posix(full, base);
        if result == "src/main.rs" {
            println!("{}", "PASS".green());
            passed += 1;
        } else {
            println!("{} got {:?}", "FAIL".red(), result);
            failed += 1;
        }
    }

    print!("  [7] Repo orphan artifacts... ");
    {
        let repo_root = repo_root_for_skill(_skill_root);
        if repo_root.join("ARCHIVOS_LISTA.md").exists() {
            println!("{}", "FAIL".red());
            failed += 1;
        } else {
            println!("{}", "PASS".green());
            passed += 1;
        }
    }

    summary("Unit Tests", passed, failed, 0);

    if failed > 0 {
        bail!("{failed} unit test(s) failed");
    }
    Ok(())
}

// ── Release Gate ─────────────────────────────────────────────────

#[derive(Args)]
pub struct ReleaseGateArgs {
    /// Target workspace directory for bootstrap + validate cycle
    #[arg(long)]
    pub target: Option<PathBuf>,
}

/// Aggregated release-blocking validation chain.
///
/// Runs in order: unit → skill-version → package-hygiene → platform-claims
/// → smoke (bootstrap + validate + encoding + assembly).
/// Fails immediately on the first error so CI gets a clear signal.
pub fn release_gate(skill_root: &Path, args: ReleaseGateArgs) -> Result<()> {
    println!("{}", "═══════════════════════════════════════".cyan());
    println!("{}", "  Release Gate — skill-dev-cli".cyan());
    println!("{}", "═══════════════════════════════════════".cyan());

    let skill_cli = find_skill_cli(skill_root)
        .context("skill CLI not found; build prd-to-product-agents-cli first")?;
    let skill_root_str = skill_root.display().to_string();

    let mut step = 0u32;
    let mut passed = 0u32;

    // Step 1: unit tests
    step += 1;
    print!("  [{step}] Unit tests... ");
    match unit(skill_root) {
        Ok(()) => { println!("{}", "PASS".green()); passed += 1; }
        Err(e) => {
            println!("{}", "FAIL".red());
            bail!("Release gate blocked at step {step} (unit tests): {e}");
        }
    }

    // Step 2: skill-version
    step += 1;
    print!("  [{step}] Skill version consistency... ");
    let version_args = vec![
        "--skill-root".to_string(), skill_root_str.clone(),
        "validate".to_string(), "skill-version".to_string(),
    ];
    let result = run_executable(&skill_cli, &version_args, None)?;
    if result.success {
        println!("{}", "PASS".green());
        passed += 1;
    } else {
        println!("{}", "FAIL".red());
        bail!("Release gate blocked at step {step} (skill-version):\n{}", result.combined_output());
    }

    // Step 3: package hygiene
    step += 1;
    print!("  [{step}] Package hygiene... ");
    let hygiene_args = vec![
        "--skill-root".to_string(), skill_root_str.clone(),
        "validate".to_string(), "package-hygiene".to_string(),
    ];
    let result = run_executable(&skill_cli, &hygiene_args, None)?;
    if result.success {
        println!("{}", "PASS".green());
        passed += 1;
    } else {
        println!("{}", "FAIL".red());
        bail!("Release gate blocked at step {step} (package-hygiene):\n{}", result.combined_output());
    }

    // Step 4: platform claims
    step += 1;
    print!("  [{step}] Platform claims... ");
    let claims_args = vec![
        "--skill-root".to_string(), skill_root_str.clone(),
        "validate".to_string(), "platform-claims".to_string(),
    ];
    let result = run_executable(&skill_cli, &claims_args, None)?;
    if result.success {
        println!("{}", "PASS".green());
        passed += 1;
    } else {
        println!("{}", "FAIL".red());
        bail!("Release gate blocked at step {step} (platform-claims):\n{}", result.combined_output());
    }

    // Step 5: smoke (bootstrap + validate + encoding + assembly)
    step += 1;
    print!("  [{step}] Smoke tests (bootstrap → validate → assembly)... ");
    match smoke(skill_root, SmokeArgs { target: args.target }) {
        Ok(()) => { println!("{}", "PASS".green()); passed += 1; }
        Err(e) => {
            println!("{}", "FAIL".red());
            bail!("Release gate blocked at step {step} (smoke): {e}");
        }
    }

    println!();
    println!("{}", "═══════════════════════════════════════".green());
    println!(
        "  {} All {passed}/{step} release gate checks passed",
        "RELEASE GATE: PASS".green().bold()
    );
    println!("{}", "═══════════════════════════════════════".green());
    Ok(())
}
