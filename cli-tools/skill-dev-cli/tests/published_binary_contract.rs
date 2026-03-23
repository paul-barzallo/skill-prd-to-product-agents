use std::path::{Path, PathBuf};
use std::process::Command;

const UNIX_PUBLISHED_BINARIES: &[&str] = &[
    ".agents/skills/prd-to-product-agents/bin/prd-to-product-agents-cli-linux-x64",
    ".agents/skills/prd-to-product-agents/bin/prd-to-product-agents-cli-darwin-arm64",
    ".agents/skills/prd-to-product-agents/templates/workspace/.agents/bin/prd-to-product-agents/prdtp-agents-functions-cli-linux-x64",
    ".agents/skills/prd-to-product-agents/templates/workspace/.agents/bin/prd-to-product-agents/prdtp-agents-functions-cli-darwin-arm64",
];

fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .expect("could not resolve repo root from CARGO_MANIFEST_DIR")
        .to_path_buf()
}

#[test]
fn unix_published_binaries_are_tracked_as_executable() {
    let output = Command::new("git")
        .current_dir(repo_root())
        .args(["ls-files", "--stage", "--"])
        .args(UNIX_PUBLISHED_BINARIES)
        .output()
        .expect("failed to inspect git index for published binaries");

    assert!(
        output.status.success(),
        "git ls-files --stage failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut missing = Vec::new();

    for path in UNIX_PUBLISHED_BINARIES {
        let line = stdout.lines().find(|line| line.ends_with(path));
        match line {
            Some(line) if line.starts_with("100755 ") => {}
            Some(line) => missing.push(format!("{path}: expected mode 100755, found '{line}'")),
            None => missing.push(format!("{path}: not tracked in git index")),
        }
    }

    assert!(
        missing.is_empty(),
        "Published Unix binaries must be executable in git index:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn unix_release_gate_workflow_sets_execute_bits_for_collected_binaries() {
    let workflow = repo_root()
        .join(".github")
        .join("workflows")
        .join("build-skill-binaries.yml");
    let content = std::fs::read_to_string(&workflow)
        .expect("failed to read build-skill-binaries workflow");

    let expected_entries = [
        "collected/skill-dev-cli-${{ matrix.suffix }}",
        "collected/prd-to-product-agents-cli-${{ matrix.suffix }}",
        "collected/prdtp-agents-functions-cli-${{ matrix.suffix }}",
    ];

    let mut missing = Vec::new();
    for entry in expected_entries {
        if !content.contains(entry) {
            missing.push(entry);
        }
    }

    assert!(
        missing.is_empty(),
        "Unix release-gate workflow must chmod all collected binaries before execution:\n  {}",
        missing.join("\n  ")
    );
}
