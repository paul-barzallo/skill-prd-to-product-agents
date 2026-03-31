use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use prdtp_agents_shared::capabilities::render_bootstrap_seed_capabilities_yaml;
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

fn seed_capabilities_file(workspace: &Path) {
    let caps_path = workspace.join(".github").join("workspace-capabilities.yaml");
    if let Some(parent) = caps_path.parent() {
        fs::create_dir_all(parent).expect("failed to create .github directory");
    }

    let yaml =
        render_bootstrap_seed_capabilities_yaml().expect("failed to render capabilities yaml");
    fs::write(&caps_path, yaml).expect("failed to write capabilities yaml");
}

#[test]
fn validate_generated_records_context_checksums_and_detects_staleness() {
    let template_root = skill_root().join("templates").join("workspace");
    let workspace = tempfile::tempdir().expect("failed to create temp dir");
    copy_dir_recursive(&template_root, workspace.path());
    seed_capabilities_file(workspace.path());

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
fn validate_package_passes_for_isolated_skill_copy() {
    let skill_copy_root = tempfile::tempdir().expect("failed to create skill copy dir");
    let copied_skill = skill_copy_root.path().join("prd-to-product-agents");
    copy_dir_recursive(&skill_root(), &copied_skill);

    let output = Command::new(cli_binary())
        .args([
            "--skill-root",
            &copied_skill.to_string_lossy(),
            "validate",
            "package",
        ])
        .output()
        .expect("failed to execute validate package");

    assert!(
        output.status.success(),
        "validate package failed for isolated skill copy:\nSTDOUT:\n{}\nSTDERR:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !copied_skill.join(".state/logs/cli-diagnostic.log").exists(),
        "validate package wrote logs into the copied skill package"
    );
}

#[test]
fn published_runtime_help_hides_maintainer_only_commands_in_isolated_skill_copy() {
    let skill_copy_root = tempfile::tempdir().expect("failed to create skill copy dir");
    let copied_skill = skill_copy_root.path().join("prd-to-product-agents");
    copy_dir_recursive(&skill_root(), &copied_skill);

    let runtime_binary = if cfg!(target_os = "windows") {
        copied_skill
            .join("templates")
            .join("workspace")
            .join(".agents")
            .join("bin")
            .join("prd-to-product-agents")
            .join("prdtp-agents-functions-cli-windows-x64.exe")
    } else if cfg!(target_os = "macos") {
        copied_skill
            .join("templates")
            .join("workspace")
            .join(".agents")
            .join("bin")
            .join("prd-to-product-agents")
            .join("prdtp-agents-functions-cli-darwin-arm64")
    } else {
        copied_skill
            .join("templates")
            .join("workspace")
            .join(".agents")
            .join("bin")
            .join("prd-to-product-agents")
            .join("prdtp-agents-functions-cli-linux-x64")
    };

    let global_help = Command::new(&runtime_binary)
        .arg("--help")
        .output()
        .expect("failed to execute runtime --help");
    assert!(
        global_help.status.success(),
        "runtime --help failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
        String::from_utf8_lossy(&global_help.stdout),
        String::from_utf8_lossy(&global_help.stderr)
    );
    let global_help = String::from_utf8_lossy(&global_help.stdout);
    assert!(
        !global_help.contains("github"),
        "published runtime unexpectedly exposes github command:\n{global_help}"
    );

    let governance_help = Command::new(&runtime_binary)
        .args(["governance", "--help"])
        .output()
        .expect("failed to execute governance --help");
    assert!(
        governance_help.status.success(),
        "governance --help failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
        String::from_utf8_lossy(&governance_help.stdout),
        String::from_utf8_lossy(&governance_help.stderr)
    );
    let governance_help = String::from_utf8_lossy(&governance_help.stdout);
    assert!(
        !governance_help.contains("promote-enterprise-readiness"),
        "published runtime unexpectedly exposes enterprise promotion helper:\n{governance_help}"
    );

    let audit_help = Command::new(&runtime_binary)
        .args(["audit", "--help"])
        .output()
        .expect("failed to execute audit --help");
    assert!(
        audit_help.status.success(),
        "audit --help failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
        String::from_utf8_lossy(&audit_help.stdout),
        String::from_utf8_lossy(&audit_help.stderr)
    );
    let audit_help = String::from_utf8_lossy(&audit_help.stdout);
    assert!(
        !audit_help.contains("export"),
        "published runtime unexpectedly exposes audit export:\n{audit_help}"
    );
}

#[test]
fn package_hygiene_rejects_runtime_state_directories_in_template() {
    let skill_copy_root = tempfile::tempdir().expect("failed to create skill copy dir");
    let copied_skill = skill_copy_root.path().join("prd-to-product-agents");
    copy_dir_recursive(&skill_root(), &copied_skill);

    let logs_dir = copied_skill
        .join("templates")
        .join("workspace")
        .join(".state")
        .join("logs");
    fs::create_dir_all(&logs_dir).expect("failed to create template logs dir");

    let output = Command::new(cli_binary())
        .args([
            "--skill-root",
            &copied_skill.to_string_lossy(),
            "validate",
            "package-hygiene",
        ])
        .output()
        .expect("failed to execute validate package-hygiene");

    assert!(
        !output.status.success(),
        "validate package-hygiene unexpectedly allowed runtime state directories in template"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(".state/logs/"), "unexpected stderr: {stderr}");
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
