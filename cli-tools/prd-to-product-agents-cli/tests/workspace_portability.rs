use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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

fn template_root() -> PathBuf {
    skill_root().join("templates").join("workspace")
}

#[test]
fn workspace_template_does_not_reference_repo_only_paths() {
    let root = template_root();
    let mut files = Vec::new();
    collect_text_files(&root, &root, &mut files);

    let forbidden_patterns = [
        ".agents/skills/prd-to-product-agents/bin",
        "--skill-root",
        "cli-tools/",
        "skill-dev-cli",
    ];

    let mut offenders = Vec::new();
    for (relative, path) in files {
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

        for pattern in forbidden_patterns {
            if content.contains(pattern) {
                offenders.push(format!("{relative}: {pattern}"));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "workspace template contains repo-only or skill-source references:\n  {}",
        offenders.join("\n  ")
    );
}

fn collect_text_files(root: &Path, current: &Path, files: &mut Vec<(String, PathBuf)>) {
    let entries = fs::read_dir(current)
        .unwrap_or_else(|error| panic!("failed to read directory {}: {error}", current.display()));

    for entry in entries {
        let entry = entry.unwrap_or_else(|error| panic!("failed to read directory entry: {error}"));
        let path = entry.path();
        if path.is_dir() {
            collect_text_files(root, &path, files);
            continue;
        }

        if !is_text_contract_file(&path) {
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .expect("path under template root")
            .to_string_lossy()
            .replace('\\', "/");
        files.push((relative, path));
    }
}

fn is_text_contract_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|value| value.to_str()).unwrap_or_default();
    if name == "AGENTS.md" || name == ".instructions.md" {
        return true;
    }

    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("md" | "txt" | "yaml" | "yml" | "json" | "toml" | "sh" | "ps1")
    )
}
