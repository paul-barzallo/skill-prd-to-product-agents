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
