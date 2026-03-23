use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

const LF_EXTENSIONS: &[&str] = &["md", "txt", "yaml", "yml", "json", "jsonc", "sql", "sha256", "sh"];

fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .expect("could not resolve repo root from CARGO_MANIFEST_DIR")
        .to_path_buf()
}

fn is_skill_root(path: &Path) -> bool {
    path.join("SKILL.md").is_file()
        && path.join("VERSION").is_file()
        && path.join("templates").join("workspace").is_dir()
}

fn skill_root() -> PathBuf {
    let repo_root = repo_root();
    if is_skill_root(&repo_root) {
        return repo_root;
    }

    let nested = repo_root
        .join(".agents")
        .join("skills")
        .join("prd-to-product-agents");
    if is_skill_root(&nested) {
        return nested;
    }

    panic!("could not resolve skill root from {}", repo_root.display());
}

fn template_root() -> PathBuf {
    skill_root().join("templates").join("workspace")
}

fn check_gitattributes_lf_rules(root: &Path, label: &str) -> Vec<String> {
    let gitattributes_path = root.join(".gitattributes");
    let mut missing = Vec::new();

    let content = match fs::read_to_string(&gitattributes_path) {
        Ok(content) => content,
        Err(error) => {
            missing.push(format!("{label}: failed to read {}: {error}", gitattributes_path.display()));
            return missing;
        }
    };

    for extension in LF_EXTENSIONS {
        let rule = format!("*.{extension} text eol=lf");
        if !content.contains(&rule) {
            missing.push(format!("{label}: missing '{rule}'"));
        }
    }

    missing
}

fn scan_files_for_crlf(root: &Path) -> Vec<String> {
    let mut violations = Vec::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        if !LF_EXTENSIONS.contains(&extension) {
            continue;
        }

        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                violations.push(format!("{} could not be read: {error}", path.display()));
                continue;
            }
        };

        if bytes.windows(2).any(|window| window == b"\r\n") {
            let relative = path.strip_prefix(root).unwrap_or(path);
            violations.push(format!("{} contains CRLF line endings but LF is required", relative.display()));
        }
    }

    violations
}

#[test]
fn test_gitattributes_lf_rules() {
    let mut violations = Vec::new();
    violations.extend(check_gitattributes_lf_rules(&repo_root(), "repo root"));
    violations.extend(check_gitattributes_lf_rules(&skill_root(), "skill root"));
    violations.extend(check_gitattributes_lf_rules(&template_root(), "workspace template"));

    assert!(
        violations.is_empty(),
        ".gitattributes LF rules are incomplete:\n  {}",
        violations.join("\n  ")
    );
}

#[test]
fn test_template_files_are_lf() {
    let violations = scan_files_for_crlf(&template_root());
    assert!(
        violations.is_empty(),
        "Template files contain CRLF line endings:\n  {}",
        violations.join("\n  ")
    );
}
