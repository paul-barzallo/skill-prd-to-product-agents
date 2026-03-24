use std::fs;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn skill_root() -> PathBuf {
    if let Some(explicit) = env::var_os("PRDTP_SKILL_ROOT").or_else(|| env::var_os("SKILL_ROOT")) {
        return normalize_skill_root(PathBuf::from(explicit));
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("could not resolve repository root from CARGO_MANIFEST_DIR");

    normalize_skill_root(repo_root.to_path_buf())
}

fn normalize_skill_root(candidate: PathBuf) -> PathBuf {
    if candidate.join("SKILL.md").is_file() {
        return candidate;
    }

    let nested = candidate
        .join(".agents")
        .join("skills")
        .join("prd-to-product-agents");
    if nested.join("SKILL.md").is_file() {
        return nested;
    }

    panic!(
        "could not resolve skill root from {}; set PRDTP_SKILL_ROOT to the repo root or skill root",
        candidate.display()
    );
}

fn cli_binary() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_BIN_EXE_prd-to-product-agents-cli"));
    assert!(path.exists(), "CLI binary not found at {}", path.display());
    path
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

fn assert_generated_workspace_has_lf_text_files(target: &Path) {
    let extensions = ["md", "yaml", "yml", "json", "txt"];
    let mut encoding_issues = Vec::new();
    for entry in walkdir::WalkDir::new(target)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !extensions.contains(&ext) {
            continue;
        }
        let bytes = fs::read(path).unwrap();
        if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
            encoding_issues.push(format!("BOM: {}", path.display()));
        }
        let text = String::from_utf8_lossy(&bytes);
        if text.contains("\r\n") {
            encoding_issues.push(format!("CRLF: {}", path.display()));
        }
    }
    assert!(
        encoding_issues.is_empty(),
        "Encoding issues in generated workspace:\n  {}",
        encoding_issues.join("\n  ")
    );
}

fn force_crlf(path: &Path) {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read '{}' for CRLF rewrite: {error}", path.display()));
    let crlf = content.replace("\r\n", "\n").replace('\n', "\r\n");
    fs::write(path, crlf)
        .unwrap_or_else(|error| panic!("failed to write CRLF content to '{}': {error}", path.display()));
}

fn count_overlay_files(target: &Path) -> usize {
    let overlay_root = target.join(".bootstrap-overlays");
    if !overlay_root.exists() {
        return 0;
    }

    walkdir::WalkDir::new(overlay_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count()
}

#[test]
fn bootstrap_creates_valid_workspace() {
    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    let target = tmp.path();

    let output = Command::new(cli_binary())
        .args([
            "--skill-root",
            &skill_root().to_string_lossy(),
            "bootstrap",
            "workspace",
            "--target",
            &target.to_string_lossy(),
            "--project-name",
            "E2E Test Project",
            "--github-owner",
            "test-org",
            "--github-repo",
            "test-repo",
            "--skip-git",
            "--skip-db-init",
        ])
        .output()
        .expect("failed to execute CLI");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "bootstrap failed (exit {:?}):\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}",
        output.status.code()
    );

    let mut missing_files = Vec::new();
    for file in prdtp_agents_shared::workspace_paths::REQUIRED_FILES {
        if !target.join(file).exists() {
            missing_files.push(file.to_string());
        }
    }
    assert!(
        missing_files.is_empty(),
        "Missing required files after bootstrap:\n  {}",
        missing_files.join("\n  ")
    );

    let agents_dir = target.join(".github").join("agents");
    let mut missing_agents = Vec::new();
    for name in prdtp_agents_shared::workspace_paths::AGENT_NAMES {
        let agent_file = agents_dir.join(format!("{name}.agent.md"));
        if !agent_file.exists() {
            missing_agents.push(format!("{name}.agent.md"));
        }
    }
    assert!(
        missing_agents.is_empty(),
        "Missing agent files after bootstrap:\n  {}",
        missing_agents.join("\n  ")
    );

    let vision = fs::read_to_string(target.join("docs/project/vision.md"))
        .expect("vision.md should exist");
    assert!(!vision.contains("{{PROJECT_NAME}}"));
    assert!(!vision.contains("REPLACE_ME_GITHUB_OWNER"));

    let state_dir = target.join(".state");
    assert!(state_dir.join("bootstrap-manifest.txt").exists());
    assert!(state_dir.join("bootstrap-report.md").exists());

    let manifest_content =
        fs::read_to_string(state_dir.join("bootstrap-manifest.txt")).expect("manifest should be readable");
    let data_lines: Vec<&str> = manifest_content
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .collect();
    assert!(!data_lines.is_empty(), "Manifest has no data lines");
    for line in &data_lines {
        let cols: Vec<&str> = line.split('\t').collect();
        assert_eq!(cols.len(), 4, "Manifest line has wrong column count: {line}");
    }

    assert!(
        target.join(".github/workspace-capabilities.yaml").exists(),
        "workspace-capabilities.yaml not generated"
    );

    assert_generated_workspace_has_lf_text_files(target);

    let report =
        fs::read_to_string(state_dir.join("bootstrap-report.md")).expect("report should be readable");
    assert!(report.contains("Status: FULL") || report.contains("Status: DEGRADED"));
    assert!(report.contains("## Binary Bundle Integrity"));
    assert!(report.contains("Workspace State: bootstrapped"));
    assert!(report.contains("Governance status: pending_configuration"));
    assert!(report.contains("Readiness status: not_ready"));
    assert!(report.contains("governance configure"));
}

#[test]
fn bootstrap_normalizes_crlf_text_sources() {
    let skill_copy_root = tempfile::tempdir().expect("failed to create skill copy dir");
    let copied_skill = skill_copy_root.path().join("prd-to-product-agents");
    copy_dir_recursive(&skill_root(), &copied_skill);

    force_crlf(&copied_skill.join("templates/workspace/.github/agents/CONTEXT_ZONE_DIVIDER.txt"));
    force_crlf(&copied_skill.join("templates/workspace/.github/immutable-files.txt"));
    force_crlf(&copied_skill.join("templates/workspace/reporting-ui/vendor/LICENSE-xlsx.txt"));

    let workspace = tempfile::tempdir().expect("failed to create target dir");
    let output = Command::new(cli_binary())
        .args([
            "--skill-root",
            &copied_skill.to_string_lossy(),
            "bootstrap",
            "workspace",
            "--target",
            &workspace.path().to_string_lossy(),
            "--project-name",
            "CRLF Normalization Test",
            "--skip-git",
            "--skip-db-init",
        ])
        .output()
        .expect("failed to execute CLI");

    assert!(
        output.status.success(),
        "bootstrap failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_generated_workspace_has_lf_text_files(workspace.path());
}

#[test]
fn bootstrap_rerun_preserves_observable_stability() {
    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    let target = tmp.path();

    let sr = skill_root();
    let sr_str = sr.to_string_lossy();
    let tgt_str = target.to_string_lossy();
    let common_args = [
        "--skill-root",
        &*sr_str,
        "bootstrap",
        "workspace",
        "--target",
        &*tgt_str,
        "--project-name",
        "Idempotent Test",
        "--skip-git",
        "--skip-db-init",
    ];

    let first = Command::new(cli_binary())
        .args(&common_args)
        .output()
        .expect("first bootstrap failed to execute");
    assert!(
        first.status.success(),
        "First bootstrap failed:\n{}",
        String::from_utf8_lossy(&first.stderr)
    );

    let count_first: usize = walkdir::WalkDir::new(target)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count();

    let second = Command::new(cli_binary())
        .args(&common_args)
        .output()
        .expect("second bootstrap failed to execute");
    assert!(
        second.status.success(),
        "Second bootstrap failed:\n{}",
        String::from_utf8_lossy(&second.stderr)
    );

    let count_second: usize = walkdir::WalkDir::new(target)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count();

    let overlay_count = count_overlay_files(target);

    assert!(
        count_second <= count_first + 2,
        "File count grew unexpectedly: first={count_first}, second={count_second}"
    );
    assert_eq!(
        overlay_count, 0,
        "Rerun produced unexpected overlay files: {overlay_count}"
    );
}

#[test]
fn bootstrap_rerun_preserves_stability_with_crlf_text_sources() {
    let skill_copy_root = tempfile::tempdir().expect("failed to create skill copy dir");
    let copied_skill = skill_copy_root.path().join("prd-to-product-agents");
    copy_dir_recursive(&skill_root(), &copied_skill);

    force_crlf(&copied_skill.join("templates/workspace/.github/agents/CONTEXT_ZONE_DIVIDER.txt"));
    force_crlf(&copied_skill.join("templates/workspace/.github/immutable-files.txt"));
    force_crlf(&copied_skill.join("templates/workspace/reporting-ui/vendor/LICENSE-xlsx.txt"));

    let workspace = tempfile::tempdir().expect("failed to create target dir");
    let sr_str = copied_skill.to_string_lossy();
    let tgt_str = workspace.path().to_string_lossy();
    let common_args = [
        "--skill-root",
        &*sr_str,
        "bootstrap",
        "workspace",
        "--target",
        &*tgt_str,
        "--project-name",
        "Idempotent CRLF Test",
        "--skip-git",
        "--skip-db-init",
    ];

    let first = Command::new(cli_binary())
        .args(&common_args)
        .output()
        .expect("first bootstrap failed to execute");
    assert!(
        first.status.success(),
        "First bootstrap failed:\n{}",
        String::from_utf8_lossy(&first.stderr)
    );

    let count_first: usize = walkdir::WalkDir::new(workspace.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count();

    let second = Command::new(cli_binary())
        .args(&common_args)
        .output()
        .expect("second bootstrap failed to execute");
    assert!(
        second.status.success(),
        "Second bootstrap failed:\n{}",
        String::from_utf8_lossy(&second.stderr)
    );

    let count_second: usize = walkdir::WalkDir::new(workspace.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count();

    let overlay_count = count_overlay_files(workspace.path());

    assert!(
        count_second <= count_first + 2,
        "File count grew unexpectedly for CRLF rerun: first={count_first}, second={count_second}"
    );
    assert_eq!(
        overlay_count, 0,
        "CRLF rerun produced unexpected overlay files: {overlay_count}"
    );
    assert_generated_workspace_has_lf_text_files(workspace.path());
}

#[test]
fn bootstrap_fails_when_binary_bundle_checksum_is_invalid() {
    let skill_copy_root = tempfile::tempdir().expect("failed to create skill copy dir");
    let copied_skill = skill_copy_root.path().join("prd-to-product-agents");
    copy_dir_recursive(&skill_root(), &copied_skill);

    let checksum_path = copied_skill.join("bin/checksums.sha256");
    let checksum = fs::read_to_string(&checksum_path).expect("failed to read checksum manifest");
    let first_line = checksum
        .lines()
        .next()
        .expect("checksum manifest should contain at least one entry");
    let (hash, _) = first_line
        .split_once("  ")
        .expect("checksum manifest line should contain a hash and file name");
    let broken = checksum.replacen(
        hash,
        "0000000000000000000000000000000000000000000000000000000000000000",
        1,
    );
    fs::write(&checksum_path, broken).expect("failed to corrupt checksum manifest");

    let workspace = tempfile::tempdir().expect("failed to create target dir");
    let output = Command::new(cli_binary())
        .args([
            "--skill-root",
            &copied_skill.to_string_lossy(),
            "bootstrap",
            "workspace",
            "--target",
            &workspace.path().to_string_lossy(),
            "--skip-git",
            "--skip-db-init",
        ])
        .output()
        .expect("failed to execute CLI");

    assert!(!output.status.success(), "bootstrap unexpectedly succeeded with invalid checksum manifest");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("skill bootstrap bundle integrity failed")
            || stderr.contains("checksum mismatch"),
        "unexpected stderr: {stderr}"
    );
}
