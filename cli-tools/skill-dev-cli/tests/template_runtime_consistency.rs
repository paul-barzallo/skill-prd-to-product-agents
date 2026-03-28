use std::env;
use std::path::{Path, PathBuf};

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
        && path.join("templates").join("workspace").is_dir()
}

fn skill_root() -> PathBuf {
    if let Some(explicit) = env::var_os("PRDTP_SKILL_ROOT").or_else(|| env::var_os("SKILL_ROOT")) {
        return normalize_skill_root(PathBuf::from(explicit));
    }

    let repo_root = repo_root();
    normalize_skill_root(repo_root)
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

fn template_root() -> PathBuf {
    skill_root().join("templates").join("workspace")
}

#[test]
fn required_files_exist_in_template() {
    let root = template_root();
    let mut missing = Vec::new();
    for path in prdtp_agents_shared::workspace_paths::REQUIRED_FILES {
        if !root.join(path).exists() {
            missing.push(*path);
        }
    }
    assert!(
        missing.is_empty(),
        "REQUIRED_FILES not in template:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn extended_required_files_exist_in_template() {
    let root = template_root();
    let mut missing = Vec::new();
    for path in prdtp_agents_shared::workspace_paths::EXTENDED_REQUIRED_FILES {
        if !root.join(path).exists() {
            missing.push(*path);
        }
    }
    assert!(
        missing.is_empty(),
        "EXTENDED_REQUIRED_FILES not in template:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn yaml_files_exist_in_template() {
    let root = template_root();
    let mut missing = Vec::new();
    for path in prdtp_agents_shared::workspace_paths::YAML_FILES {
        if !root.join(path).exists() {
            missing.push(*path);
        }
    }
    assert!(
        missing.is_empty(),
        "YAML_FILES not in template:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn agent_files_exist_in_template() {
    let agents_dir = template_root().join(".github").join("agents");
    let mut missing = Vec::new();
    for name in prdtp_agents_shared::workspace_paths::AGENT_NAMES {
        if !agents_dir.join(format!("{name}.agent.md")).exists() {
            missing.push(*name);
        }
    }
    assert!(
        missing.is_empty(),
        "AGENT_NAMES not in template:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn immutable_files_path_exists() {
    let path = template_root().join(prdtp_agents_shared::workspace_paths::IMMUTABLE_FILES_PATH);
    assert!(path.exists(), "IMMUTABLE_FILES_PATH not in template");
}

#[test]
fn l2_agents_are_subset_of_agent_names() {
    for agent in prdtp_agents_shared::workspace_paths::L2_AGENTS {
        assert!(
            prdtp_agents_shared::workspace_paths::AGENT_NAMES.contains(agent),
            "L2 agent '{agent}' not in AGENT_NAMES"
        );
    }
}

#[test]
fn coordinator_agents_are_subset_of_agent_names() {
    for agent in prdtp_agents_shared::workspace_paths::COORDINATOR_AGENTS {
        assert!(
            prdtp_agents_shared::workspace_paths::AGENT_NAMES.contains(agent),
            "Coordinator '{agent}' not in AGENT_NAMES"
        );
    }
}

#[test]
fn yaml_files_have_yaml_extension() {
    for path in prdtp_agents_shared::workspace_paths::YAML_FILES {
        assert!(
            path.ends_with(".yaml") || path.ends_with(".yml"),
            "YAML_FILES entry '{path}' does not have .yaml/.yml extension"
        );
    }
}

#[test]
fn governance_immutable_files_path_matches_constant() {
    let template = template_root();
    let from_constant = template.join(prdtp_agents_shared::workspace_paths::IMMUTABLE_FILES_PATH);
    let from_github = template.join(".github/immutable-files.txt");

    assert_eq!(
        from_constant, from_github,
        "IMMUTABLE_FILES_PATH constant does not resolve to .github/immutable-files.txt"
    );
}

#[test]
fn immutable_files_list_is_non_empty() {
    let path = template_root().join(prdtp_agents_shared::workspace_paths::IMMUTABLE_FILES_PATH);
    let content = std::fs::read_to_string(&path).expect("failed to read immutable-files.txt");
    let entries: Vec<&str> = content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .collect();
    assert!(
        !entries.is_empty(),
        "immutable-files.txt should contain at least one protected path"
    );
}

#[test]
fn template_yaml_files_are_valid() {
    let root = template_root();
    let mut invalid = Vec::new();
    for path in prdtp_agents_shared::workspace_paths::YAML_FILES {
        let full = root.join(path);
        if !full.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&full).unwrap_or_default();
        if content.trim().is_empty() {
            continue;
        }
        match serde_yaml::from_str::<serde_yaml::Value>(&content) {
            Ok(_) => {}
            Err(error) => invalid.push(format!("{path}: {error}")),
        }
    }
    assert!(
        invalid.is_empty(),
        "Template YAML files failed to parse:\n  {}",
        invalid.join("\n  ")
    );
}
