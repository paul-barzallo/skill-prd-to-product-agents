use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::thread;

use prdtp_agents_shared::capabilities::render_bootstrap_seed_capabilities_yaml;
use prdtp_agents_shared::enums::{HandoffReason, Role};
use serde_yaml::Value;
use tempfile::TempDir;
use walkdir::WalkDir;

fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest
        .parent()
        .and_then(Path::parent)
        .expect("could not resolve repo root from CARGO_MANIFEST_DIR")
        .to_path_buf();

    if is_repo_root(&candidate) {
        return candidate;
    }

    find_repo_root_from(std::env::current_dir().ok())
        .or_else(|| find_repo_root_from(std::env::current_exe().ok()))
        .unwrap_or_else(|| {
            panic!(
                "could not resolve repo root; compile-time path '{}' is stale and no runtime fallback matched",
                candidate.display()
            )
        })
}

fn is_skill_root(path: &Path) -> bool {
    path.join("SKILL.md").is_file() && path.join("templates").join("workspace").is_dir()
}

fn skill_root() -> PathBuf {
    if let Some(explicit) = env::var_os("PRDTP_SKILL_ROOT").or_else(|| env::var_os("SKILL_ROOT")) {
        return normalize_skill_root(PathBuf::from(explicit));
    }

    normalize_skill_root(repo_root())
}

fn normalize_skill_root(candidate: PathBuf) -> PathBuf {
    if is_skill_root(&candidate) {
        return candidate;
    }

    let nested = candidate
        .join(".agents")
        .join("skills")
        .join("prd-to-product-agents");
    if is_skill_root(&nested) {
        return nested;
    }

    panic!(
        "could not resolve skill root from {}; set PRDTP_SKILL_ROOT to the repo root or skill root",
        candidate.display()
    );
}

fn is_repo_root(path: &Path) -> bool {
    path.join("AGENTS.md").is_file()
        && path
            .join(".agents")
            .join("skills")
            .join("prd-to-product-agents")
            .join("templates")
            .join("workspace")
            .is_dir()
}

fn find_repo_root_from(path: Option<PathBuf>) -> Option<PathBuf> {
    let path = path?;
    for ancestor in path.ancestors() {
        if is_repo_root(ancestor) {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

fn template_root() -> PathBuf {
    skill_root().join("templates").join("workspace")
}

fn copy_dir_recursive(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).expect("failed to create destination directory");
    for entry in WalkDir::new(src) {
        let entry = entry.unwrap_or_else(|error| {
            panic!(
                "failed to walk source directory '{}': {error}",
                src.display()
            )
        });
        let source = entry.path();
        let relative = source.strip_prefix(src).unwrap_or_else(|error| {
            panic!(
                "failed to strip source prefix '{}' from '{}': {error}",
                src.display(),
                source.display()
            )
        });
        if relative.as_os_str().is_empty() {
            continue;
        }

        let target = dst.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).unwrap_or_else(|error| {
                panic!(
                    "failed to create destination directory '{}' from '{}': {error}",
                    target.display(),
                    source.display()
                )
            });
            continue;
        }

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|error| {
                panic!(
                    "failed to create destination parent '{}' for '{}': {error}",
                    parent.display(),
                    target.display()
                )
            });
        }

        fs::copy(source, &target).unwrap_or_else(|error| {
            panic!(
                "failed to copy '{}' to '{}': {error}",
                source.display(),
                target.display()
            )
        });
    }
}

fn make_workspace() -> TempDir {
    let temp = TempDir::new().expect("failed to create temp dir");
    copy_dir_recursive(&template_root(), temp.path());
    seed_capabilities_file(temp.path());
    temp
}

fn seed_capabilities_file(workspace: &Path) {
    let caps_path = workspace.join(".github").join("workspace-capabilities.yaml");
    if let Some(parent) = caps_path.parent() {
        fs::create_dir_all(parent).expect("failed to create .github directory");
    }

    let yaml =
        render_bootstrap_seed_capabilities_yaml().expect("failed to render capabilities yaml");

    fs::write(&caps_path, yaml).expect("failed to write capabilities yaml");
}

fn run_cli(workspace: &Path, args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_prdtp-agents-functions-cli"));
    command.arg("--workspace").arg(workspace);
    for arg in args {
        command.arg(arg);
    }
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("failed to run runtime CLI")
}

fn start_test_audit_sink() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind test audit sink");
    let address = listener
        .local_addr()
        .expect("failed to resolve test audit sink address");
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("audit sink did not receive request");
        let mut buffer = [0u8; 4096];
        let _ = stream.read(&mut buffer);
        let body = r#"{"ack_id":"ack-test"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("failed to write audit sink response");
        stream.flush().expect("failed to flush audit sink response");
    });
    (format!("http://{address}/audit"), handle)
}

fn set_capability_policy(workspace: &Path, capability: &str, enabled: bool) {
    let caps_path = workspace.join(".github/workspace-capabilities.yaml");
    let raw = fs::read_to_string(&caps_path).expect("failed to read capabilities yaml");
    let mut parsed: Value = serde_yaml::from_str(&raw).expect("failed to parse capabilities yaml");
    parsed["capabilities"][capability]["authorized"]["enabled"] = Value::Bool(enabled);
    parsed["capabilities"][capability]["authorized"]["source"] =
        Value::String("test-fixture".to_string());
    fs::write(
        &caps_path,
        serde_yaml::to_string(&parsed).expect("failed to render capabilities yaml"),
    )
    .expect("failed to write capabilities yaml");
}

fn set_governance_status(workspace: &Path, status: &str) {
    let governance_path = workspace.join(".github/github-governance.yaml");
    let raw = fs::read_to_string(&governance_path).expect("failed to read governance yaml");
    let mut parsed: Value = serde_yaml::from_str(&raw).expect("failed to parse governance yaml");
    parsed["readiness"]["status"] = Value::String(status.to_string());
    fs::write(
        &governance_path,
        serde_yaml::to_string(&parsed).expect("failed to render governance yaml"),
    )
    .expect("failed to write governance yaml");
}

fn set_governance_bool(workspace: &Path, path: &[&str], value: bool) {
    let governance_path = workspace.join(".github/github-governance.yaml");
    let raw = fs::read_to_string(&governance_path).expect("failed to read governance yaml");
    let mut parsed: Value = serde_yaml::from_str(&raw).expect("failed to parse governance yaml");
    let mut current = &mut parsed;
    for key in &path[..path.len() - 1] {
        current = current
            .get_mut(*key)
            .unwrap_or_else(|| panic!("missing governance key '{}'", key));
    }
    current[path[path.len() - 1]] = Value::Bool(value);
    fs::write(
        &governance_path,
        serde_yaml::to_string(&parsed).expect("failed to render governance yaml"),
    )
    .expect("failed to write governance yaml");
}

fn set_governance_string(workspace: &Path, path: &[&str], value: &str) {
    let governance_path = workspace.join(".github/github-governance.yaml");
    let raw = fs::read_to_string(&governance_path).expect("failed to read governance yaml");
    let mut parsed: Value = serde_yaml::from_str(&raw).expect("failed to parse governance yaml");
    let mut current = &mut parsed;
    for key in &path[..path.len() - 1] {
        current = current
            .get_mut(*key)
            .unwrap_or_else(|| panic!("missing governance key '{}'", key));
    }
    current[path[path.len() - 1]] = Value::String(value.to_string());
    fs::write(
        &governance_path,
        serde_yaml::to_string(&parsed).expect("failed to render governance yaml"),
    )
    .expect("failed to write governance yaml");
}

fn init_git_repo(workspace: &Path) {
    let output = Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(workspace)
        .output()
        .expect("failed to init git repo");
    assert!(
        output.status.success(),
        "git init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let commands: &[&[&str]] = &[
        &["config", "user.name", "Runtime Contract"],
        &["config", "user.email", "runtime-contract@example.com"],
        &["add", "."],
        &[
            "commit",
            "-m",
            "chore: GH-1 seed runtime contract workspace",
        ],
    ];
    for args in commands {
        let output = Command::new("git")
            .args(*args)
            .current_dir(workspace)
            .output()
            .expect("failed to execute git command");
        assert!(
            output.status.success(),
            "git command {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn configure_governance(workspace: &Path) {
    let output = run_cli(
        workspace,
        &[
            "governance",
            "configure",
            "--owner",
            "acme-org",
            "--repo",
            "copilot-workspace",
            "--release-gate-login",
            "devops-login",
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
            "infra-login",
        ],
        &[],
    );
    assert!(
        output.status.success(),
        "governance configure failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn promote_enterprise_readiness_updates_governance_typed_fields() {
    let workspace = make_workspace();
    let (audit_endpoint, audit_sink) = start_test_audit_sink();
    configure_governance(workspace.path());
    set_governance_string(workspace.path(), &["operating_profile"], "enterprise");
    set_governance_string(workspace.path(), &["github", "auth", "mode"], "token-api");
    set_governance_string(workspace.path(), &["audit", "mode"], "remote");
    set_governance_string(
        workspace.path(),
        &["audit", "remote", "endpoint"],
        &audit_endpoint,
    );
    set_governance_string(
        workspace.path(),
        &["audit", "remote", "auth_header_env"],
        "PRDTP_AUDIT_TOKEN",
    );
    set_governance_bool(workspace.path(), &["github", "branch_protection", "enabled"], false);
    set_governance_bool(workspace.path(), &["github", "project", "enabled"], true);

    let output = run_cli(
        workspace.path(),
        &["governance", "promote-enterprise-readiness"],
        &[("PRDTP_AUDIT_TOKEN", "test-token")],
    );
    audit_sink.join().expect("audit sink thread failed");
    assert!(
        output.status.success(),
        "governance promote-enterprise-readiness failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let governance_path = workspace.path().join(".github/github-governance.yaml");
    let raw = fs::read_to_string(&governance_path).expect("failed to read governance yaml");
    let parsed: Value = serde_yaml::from_str(&raw).expect("failed to parse governance yaml");
    assert_eq!(
        parsed["readiness"]["status"].as_str(),
        Some("production-ready")
    );
    assert_eq!(
        parsed["github"]["branch_protection"]["enabled"].as_bool(),
        Some(true)
    );
    assert_eq!(parsed["github"]["project"]["enabled"].as_bool(), Some(false));
}

#[test]
fn branch_and_reason_contract_match_runtime_enums() {
    assert_eq!(Role::DevopsReleaseEngineer.branch_prefix(), "ops");
    assert_eq!(Role::PmOrchestrator.branch_prefix(), "product");
    assert_eq!(
        HandoffReason::ReadyForRelease.to_string(),
        "ready_for_release"
    );
    assert_eq!(
        HandoffReason::EnvironmentIssue.to_string(),
        "environment_issue"
    );
}

#[test]
fn pre_commit_blocks_manual_commit_even_for_governance_yaml() {
    let workspace = make_workspace();
    let workspace_arg = workspace.path().to_string_lossy().to_string();

    let output = run_cli(
        workspace.path(),
        &[
            "git",
            "pre-commit-validate",
            "--workspace-root",
            &workspace_arg,
            "--staged-file",
            ".github/github-governance.yaml",
        ],
        &[],
    );

    assert!(
        !output.status.success(),
        "pre-commit unexpectedly allowed a direct manual commit:\nSTDOUT:\n{}\nSTDERR:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("reviewed PR approval is enforced remotely")
            && stderr.contains("Direct git commit is out of contract"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn governance_configure_sets_immutable_governance_reviewers() {
    let workspace = make_workspace();
    configure_governance(workspace.path());

    let governance = fs::read_to_string(workspace.path().join(".github/github-governance.yaml"))
        .expect("failed to read configured governance yaml");
    assert!(
        governance.contains("immutable_governance:"),
        "configured governance missing immutable_governance block"
    );
    assert!(
        governance.contains("reviewer_logins: devops-login,infra-login")
            || governance.contains("reviewer_logins: \"devops-login,infra-login\""),
        "configured governance missing immutable_governance reviewer login"
    );
    assert!(
        governance.contains("reviewer_handles: '@acme-devops,@acme-infra'")
            || governance.contains("reviewer_handles: \"@acme-devops,@acme-infra\""),
        "configured governance missing immutable_governance reviewer handles"
    );
}

#[test]
fn governance_configure_rejects_duplicate_infra_login() {
    let workspace = make_workspace();
    let output = run_cli(
        workspace.path(),
        &[
            "governance",
            "configure",
            "--owner",
            "acme-org",
            "--repo",
            "copilot-workspace",
            "--release-gate-login",
            "devops-login",
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
            "devops-login",
        ],
        &[],
    );
    assert!(
        !output.status.success(),
        "governance configure unexpectedly accepted duplicate immutable reviewer login"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("reviewer-infra-login must differ from release-gate-login"));
}

#[test]
fn governance_configure_happy_path_leaves_workspace_configured() {
    let workspace = make_workspace();
    configure_governance(workspace.path());

    let governance = fs::read_to_string(workspace.path().join(".github/github-governance.yaml"))
        .expect("failed to read governance yaml");
    assert!(governance.contains("status: configured"));
    assert!(!governance.contains("REPLACE_ME"));
    assert!(!governance.contains("@team-"));

    let codeowners = fs::read_to_string(workspace.path().join(".github/CODEOWNERS"))
        .expect("failed to read CODEOWNERS");
    assert!(codeowners.contains("@acme-infra"));
    assert!(!codeowners.contains("@team-"));

    let governance_output = run_cli(workspace.path(), &["validate", "governance"], &[]);
    assert!(
        governance_output.status.success(),
        "validate governance failed:\n{}",
        String::from_utf8_lossy(&governance_output.stderr)
    );

    let readiness_output = run_cli(workspace.path(), &["validate", "readiness"], &[]);
    assert!(
        !readiness_output.status.success(),
        "validate readiness unexpectedly passed:\nSTDOUT:\n{}\nSTDERR:\n{}",
        String::from_utf8_lossy(&readiness_output.stdout),
        String::from_utf8_lossy(&readiness_output.stderr)
    );
    let readiness_combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&readiness_output.stdout),
        String::from_utf8_lossy(&readiness_output.stderr)
    );
    assert!(readiness_combined.contains("production-ready"));
}

#[test]
fn governance_configure_requires_all_flags() {
    let workspace = make_workspace();
    let output = run_cli(
        workspace.path(),
        &[
            "governance",
            "configure",
            "--owner",
            "acme-org",
            "--repo",
            "copilot-workspace",
        ],
        &[],
    );
    assert!(
        !output.status.success(),
        "configure unexpectedly succeeded with missing flags"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--release-gate-login"));
}

#[test]
fn audit_replay_spool_smoke_succeeds_on_empty_spool() {
    let workspace = make_workspace();
    set_capability_policy(workspace.path(), "sqlite", true);
    fs::create_dir_all(workspace.path().join(".state").join("audit-spool"))
        .expect("failed to create audit spool dir");

    let output = run_cli(workspace.path(), &["audit", "replay-spool"], &[]);
    assert!(
        output.status.success(),
        "audit replay smoke failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn audit_sync_degrades_when_sqlite_policy_disabled() {
    let workspace = make_workspace();
    let output = run_cli(workspace.path(), &["audit", "sync"], &[]);
    assert!(
        output.status.success(),
        "audit sync unexpectedly failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let degraded_log = workspace.path().join(".state/state-sync-degraded.log");
    assert!(degraded_log.is_file(), "expected degraded audit sync log to be written");
    let log = fs::read_to_string(&degraded_log).expect("failed to read degraded audit sync log");
    assert!(log.contains("SQLite unauthorized"));
}

#[test]
fn report_dashboard_smoke_generates_dashboard() {
    let workspace = make_workspace();
    let output = run_cli(workspace.path(), &["report", "dashboard"], &[]);
    assert!(
        output.status.success(),
        "report dashboard smoke failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let dashboard = workspace
        .path()
        .join("docs/project/management-dashboard.md");
    let content = fs::read_to_string(&dashboard).expect("failed to read dashboard");
    assert!(content.contains("# Management Dashboard"));
}

#[test]
fn report_pack_fails_when_reporting_policy_disabled() {
    let workspace = make_workspace();
    set_capability_policy(workspace.path(), "reporting", false);
    let output = run_cli(workspace.path(), &["report", "pack"], &[]);
    assert!(
        !output.status.success(),
        "report pack unexpectedly succeeded"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("report pack is out of contract"));
}

#[test]
fn capabilities_detect_generates_parseable_yaml() {
    let workspace = make_workspace();
    init_git_repo(workspace.path());

    let output = run_cli(workspace.path(), &["capabilities", "detect"], &[]);
    assert!(
        output.status.success(),
        "capabilities detect failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let caps_path = workspace.path().join(".github/workspace-capabilities.yaml");
    let content = fs::read_to_string(&caps_path).expect("failed to read capabilities yaml");
    let parsed: Value = serde_yaml::from_str(&content).expect("capabilities yaml must parse");

    assert_eq!(
        parsed["capabilities"]["sqlite"]["detected"]["installed"].as_bool(),
        Some(true)
    );
}

#[test]
fn checkout_task_branch_rejects_dirty_worktree() {
    let workspace = make_workspace();
    init_git_repo(workspace.path());
    set_capability_policy(workspace.path(), "git", true);

    let vision_path = workspace.path().join("docs/project/vision.md");
    fs::write(&vision_path, "# Dirty change\n").expect("failed to dirty workspace");

    let output = run_cli(
        workspace.path(),
        &[
            "git",
            "checkout-task-branch",
            "--role",
            "backend-developer",
            "--issue-id",
            "GH-42",
            "--slug",
            "dirty-worktree",
            "--base",
            "main",
        ],
        &[],
    );

    assert!(
        !output.status.success(),
        "checkout-task-branch unexpectedly succeeded on a dirty worktree"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Refusing to switch branches with local changes present"));
}

#[test]
fn git_finalize_blocks_invalid_workspace_and_records_failure() {
    let workspace = make_workspace();
    init_git_repo(workspace.path());
    set_capability_policy(workspace.path(), "git", true);

    let checkout_output = Command::new("git")
        .args(["checkout", "-b", "backend/gh-42-finalize-block"])
        .current_dir(workspace.path())
        .output()
        .expect("failed to create task branch");
    assert!(
        checkout_output.status.success(),
        "git checkout -b failed: {}",
        String::from_utf8_lossy(&checkout_output.stderr)
    );

    fs::remove_file(workspace.path().join("docs/project/vision.md"))
        .expect("failed to remove required file");

    let output = run_cli(
        workspace.path(),
        &[
            "git",
            "finalize",
            "--agent-role",
            "backend-developer",
            "--summary",
            "blocked finalize",
            "--issue-ref",
            "GH-42",
            "--commit-message",
            "fix(backend): GH-42 blocked finalize",
            "--auto-stage-all",
        ],
        &[],
    );

    assert!(
        !output.status.success(),
        "git finalize unexpectedly succeeded on an invalid workspace"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Workspace validation failed before commit creation"));

    let work_units_dir = workspace.path().join(".state/work-units");
    let reports = fs::read_dir(&work_units_dir)
        .expect("failed to list work-unit reports")
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext == "json")
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    assert!(
        !reports.is_empty(),
        "expected a blocked work-unit report to be written"
    );

    let newest_report = reports
        .iter()
        .max_by_key(|entry| entry.file_name())
        .expect("missing blocked report");
    let report_content =
        fs::read_to_string(newest_report.path()).expect("failed to read blocked work-unit report");
    assert!(report_content.contains("\"result\": \"validation-failed\""));
}

#[test]
fn readiness_requires_enterprise_profile_when_marked_production_ready() {
    let workspace = make_workspace();
    configure_governance(workspace.path());
    set_governance_status(workspace.path(), "production-ready");
    set_governance_bool(
        workspace.path(),
        &["github", "branch_protection", "enabled"],
        true,
    );
    set_capability_policy(workspace.path(), "gh", false);

    let output = run_cli(workspace.path(), &["validate", "readiness"], &[]);
    assert!(
        !output.status.success(),
        "validate readiness unexpectedly passed without enterprise profile"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("operating_profile=enterprise"));
}

#[test]
fn readiness_rejects_github_project_until_supported() {
    let workspace = make_workspace();
    configure_governance(workspace.path());
    set_governance_status(workspace.path(), "production-ready");
    set_governance_bool(workspace.path(), &["github", "project", "enabled"], true);

    let output = run_cli(workspace.path(), &["validate", "readiness"], &[]);
    assert!(
        !output.status.success(),
        "validate readiness unexpectedly accepted github.project.enabled=true"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("out of the current supported operational contract"));
}

#[test]
fn governance_validation_rejects_github_app_for_enterprise_profile() {
    let workspace = make_workspace();
    configure_governance(workspace.path());
    set_governance_string(workspace.path(), &["operating_profile"], "enterprise");
    set_governance_string(workspace.path(), &["github", "auth", "mode"], "github-app");
    set_governance_string(workspace.path(), &["audit", "mode"], "remote");
    set_governance_string(
        workspace.path(),
        &["audit", "remote", "endpoint"],
        "https://audit.example.test/events",
    );
    set_governance_string(
        workspace.path(),
        &["audit", "remote", "auth_header_env"],
        "PRDTP_AUDIT_TOKEN",
    );

    let output = run_cli(workspace.path(), &["validate", "governance"], &[]);
    assert!(
        !output.status.success(),
        "validate governance unexpectedly accepted github.auth.mode=github-app"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("enterprise profile requires github.auth.mode=token-api"));
}

#[test]
fn prompt_tool_contracts_reject_execute_on_analysis_only_prompt() {
    let workspace = make_workspace();
    let prompt_path = workspace
        .path()
        .join(".github/prompts/deep-architecture-analysis.prompt.md");
    let content = fs::read_to_string(&prompt_path).expect("failed to read prompt");
    let mutated = content.replacen("  - read\n", "  - read\n  - execute\n", 1);
    fs::write(&prompt_path, mutated).expect("failed to mutate prompt");

    let output = run_cli(
        workspace.path(),
        &["validate", "ci", "prompt-tool-contracts"],
        &[],
    );
    assert!(
        !output.status.success(),
        "prompt-tool-contracts unexpectedly accepted execute on analysis-only prompt"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("must not declare execute because this workflow is analysis-only"));
}

#[test]
fn copilot_runtime_contract_rejects_github_project_execution_layer_claim() {
    let workspace = make_workspace();
    let board_path = workspace.path().join("docs/project/board.md");
    let mut content = fs::read_to_string(&board_path).expect("failed to read board.md");
    content.push_str("\nExecution layer: GitHub Issues, GitHub Project, and Pull Requests\n");
    fs::write(&board_path, content).expect("failed to mutate board.md");

    let output = run_cli(
        workspace.path(),
        &["validate", "ci", "copilot-runtime-contract"],
        &[],
    );
    assert!(
        !output.status.success(),
        "copilot-runtime-contract unexpectedly accepted a GitHub Project execution-layer claim"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(
        "board snapshots must not claim GitHub Project as part of the current execution layer"
    ));
}

#[test]
fn copilot_runtime_contract_rejects_stale_claims() {
    let workspace = make_workspace();
    let doc_path = workspace.path().join("docs/runtime/runtime-operations.md");
    let mut content = fs::read_to_string(&doc_path).expect("failed to read runtime operations");
    content.push_str("\nBootstrap initializes GitHub governance during the skill runtime.\n");
    fs::write(&doc_path, content).expect("failed to mutate runtime operations");

    let output = run_cli(
        workspace.path(),
        &["validate", "ci", "copilot-runtime-contract"],
        &[],
    );
    assert!(
        !output.status.success(),
        "copilot-runtime-contract unexpectedly accepted stale runtime claim"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("bootstrap must not claim remote GitHub governance provisioning"));
}

#[test]
fn copilot_runtime_contract_rejects_github_app_claims() {
    let workspace = make_workspace();
    let doc_path = workspace.path().join("docs/runtime/capability-contract.md");
    let mut content = fs::read_to_string(&doc_path).expect("failed to read capability contract");
    content.push_str("\nLegacy enterprise note: github-app\n");
    fs::write(&doc_path, content).expect("failed to mutate capability contract");

    let output = run_cli(
        workspace.path(),
        &["validate", "ci", "copilot-runtime-contract"],
        &[],
    );
    assert!(
        !output.status.success(),
        "copilot-runtime-contract unexpectedly accepted github-app contract drift"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unsupported github-app enterprise auth mode"));
}

#[test]
fn copilot_runtime_contract_rejects_execute_enforcement_headings() {
    let workspace = make_workspace();
    let doc_path = workspace.path().join("AGENTS.md");
    let content = fs::read_to_string(&doc_path).expect("failed to read AGENTS.md");
    let mutated = content.replacen(
        "| Agent | Intended `execute` call set |",
        "| Agent | Permitted `execute` calls |",
        1,
    );
    fs::write(&doc_path, mutated).expect("failed to mutate AGENTS.md");

    let output = run_cli(
        workspace.path(),
        &["validate", "ci", "copilot-runtime-contract"],
        &[],
    );
    assert!(
        !output.status.success(),
        "copilot-runtime-contract unexpectedly accepted execute enforcement heading drift"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("execute tables must be framed as intended call sets"));
}
