use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use prdtp_agents_shared::enums::{HandoffReason, Role};
use serde_yaml::Value;
use tempfile::TempDir;

fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .expect("could not resolve repo root from CARGO_MANIFEST_DIR")
        .to_path_buf()
}

fn template_root() -> PathBuf {
    repo_root()
        .join(".agents")
        .join("skills")
        .join("prd-to-product-agents")
        .join("templates")
        .join("workspace")
}

fn copy_dir_recursive(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).expect("failed to create destination directory");
    for entry in fs::read_dir(src).expect("failed to read source directory") {
        let entry = entry.expect("failed to read directory entry");
        let source = entry.path();
        let target = dst.join(entry.file_name());
        if source.is_dir() {
            copy_dir_recursive(&source, &target);
        } else {
            fs::copy(&source, &target).unwrap_or_else(|error| {
                panic!(
                    "failed to copy '{}' to '{}': {error}",
                    source.display(),
                    target.display()
                )
            });
        }
    }
}

fn make_workspace() -> TempDir {
    let temp = TempDir::new().expect("failed to create temp dir");
    copy_dir_recursive(&template_root(), temp.path());
    temp
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

fn set_capability_policy(workspace: &Path, capability: &str, enabled: bool) {
    let caps_path = workspace.join(".github/workspace-capabilities.yaml");
    let raw = fs::read_to_string(&caps_path).expect("failed to read capabilities yaml");
    let mut parsed: Value = serde_yaml::from_str(&raw).expect("failed to parse capabilities yaml");
    parsed["capabilities"][capability]["policy"]["enabled"] = Value::Bool(enabled);
    fs::write(&caps_path, serde_yaml::to_string(&parsed).expect("failed to render capabilities yaml"))
        .expect("failed to write capabilities yaml");
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
fn branch_and_reason_contract_match_runtime_enums() {
    assert_eq!(Role::DevopsReleaseEngineer.branch_prefix(), "ops");
    assert_eq!(Role::PmOrchestrator.branch_prefix(), "product");
    assert_eq!(HandoffReason::ReadyForRelease.to_string(), "ready_for_release");
    assert_eq!(HandoffReason::EnvironmentIssue.to_string(), "environment_issue");
}

#[test]
fn immutable_token_rejects_files_outside_manifest() {
    let workspace = make_workspace();
    let output = run_cli(
        workspace.path(),
        &[
            "governance",
            "immutable-token",
            "--reason",
            "test",
            "--files",
            "docs/project/vision.md",
        ],
        &[],
    );

    assert!(
        !output.status.success(),
        "immutable token unexpectedly allowed a file outside the manifest"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Immutable-edit tokens may only cover files"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn pre_commit_requires_token_for_governance_yaml() {
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
        &[("FINALIZE_WORK_UNIT_ALLOW_COMMIT", "1")],
    );

    assert!(
        !output.status.success(),
        "pre-commit unexpectedly allowed governance yaml without token"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Immutable governance files are staged"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn immutable_token_scope_is_exact() {
    let workspace = make_workspace();
    let workspace_arg = workspace.path().to_string_lossy().to_string();

    let token_output = run_cli(
        workspace.path(),
        &[
            "governance",
            "immutable-token",
            "--reason",
            "test",
            "--files",
            ".github/CODEOWNERS",
        ],
        &[],
    );
    assert!(
        token_output.status.success(),
        "failed to create immutable token: {}",
        String::from_utf8_lossy(&token_output.stderr)
    );

    let output = run_cli(
        workspace.path(),
        &[
            "git",
            "pre-commit-validate",
            "--workspace-root",
            &workspace_arg,
            "--staged-file",
            ".github/CODEOWNERS",
            "--staged-file",
            ".github/copilot-instructions.md",
        ],
        &[("FINALIZE_WORK_UNIT_ALLOW_COMMIT", "1")],
    );

    assert!(
        !output.status.success(),
        "pre-commit unexpectedly accepted governance files outside token scope"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not authorize all staged governance files"),
        "unexpected stderr: {stderr}"
    );
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

    let codeowners =
        fs::read_to_string(workspace.path().join(".github/CODEOWNERS")).expect("failed to read CODEOWNERS");
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
        readiness_output.status.success(),
        "validate readiness failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
        String::from_utf8_lossy(&readiness_output.stdout),
        String::from_utf8_lossy(&readiness_output.stderr)
    );
}

#[test]
fn governance_configure_requires_all_flags() {
    let workspace = make_workspace();
    let output = run_cli(
        workspace.path(),
        &["governance", "configure", "--owner", "acme-org", "--repo", "copilot-workspace"],
        &[],
    );
    assert!(!output.status.success(), "configure unexpectedly succeeded with missing flags");
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
fn audit_sync_fails_when_sqlite_policy_disabled() {
    let workspace = make_workspace();
    let output = run_cli(workspace.path(), &["audit", "sync"], &[]);
    assert!(!output.status.success(), "audit sync unexpectedly succeeded");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("audit sync is out of contract"));
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

    let dashboard = workspace.path().join("docs/project/management-dashboard.md");
    let content = fs::read_to_string(&dashboard).expect("failed to read dashboard");
    assert!(content.contains("# Management Dashboard"));
}

#[test]
fn report_pack_fails_when_reporting_policy_disabled() {
    let workspace = make_workspace();
    set_capability_policy(workspace.path(), "reporting", false);
    let output = run_cli(workspace.path(), &["report", "pack"], &[]);
    assert!(!output.status.success(), "report pack unexpectedly succeeded");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("report pack is out of contract"));
}

#[test]
fn copilot_runtime_contract_rejects_stale_claims() {
    let workspace = make_workspace();
    let doc_path = workspace
        .path()
        .join("docs/runtime/runtime-operations.md");
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
