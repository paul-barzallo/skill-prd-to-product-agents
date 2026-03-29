use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use walkdir::WalkDir;

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
        } else {
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
}

#[test]
fn validate_generated_records_context_checksums_and_detects_staleness() {
    let template_root = skill_root().join("templates").join("workspace");
    let workspace = tempfile::tempdir().expect("failed to create temp dir");
    copy_dir_recursive(&template_root, workspace.path());

    let first = Command::new(cli_binary())
        .args([
            "--skill-root",
            &skill_root().to_string_lossy(),
            "validate",
            "generated",
            "--workspace",
            &workspace.path().to_string_lossy(),
            "--record-checksums",
        ])
        .output()
        .expect("failed to execute validate generated");
    assert!(
        first.status.success(),
        "validate generated --record-checksums failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(
        workspace
            .path()
            .join(".state/context-checksums.json")
            .exists(),
        "context-checksums.json not written"
    );

    let vision_path = workspace.path().join("docs/project/vision.md");
    let mut vision = fs::read_to_string(&vision_path).expect("failed to read vision.md");
    vision.push_str("\nFreshness drift test.\n");
    fs::write(&vision_path, vision).expect("failed to write vision.md");

    let second = Command::new(cli_binary())
        .args([
            "--skill-root",
            &skill_root().to_string_lossy(),
            "validate",
            "generated",
            "--workspace",
            &workspace.path().to_string_lossy(),
        ])
        .output()
        .expect("failed to execute validate generated");
    assert!(
        second.status.success(),
        "validate generated after freshness change failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
        String::from_utf8_lossy(&second.stdout),
        String::from_utf8_lossy(&second.stderr)
    );

    let validation_report =
        fs::read_to_string(workspace.path().join(".state/workspace-validation.md"))
            .expect("failed to read workspace validation report");
    assert!(validation_report.contains(
        "context freshness: canonical file changed since baseline: docs/project/vision.md"
    ));
}

#[test]
fn validate_all_passes_with_runtime_smoke() {
    let output = Command::new(cli_binary())
        .args([
            "--skill-root",
            &skill_root().to_string_lossy(),
            "validate",
            "all",
        ])
        .output()
        .expect("failed to execute validate all");

    assert!(
        output.status.success(),
        "validate all failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
