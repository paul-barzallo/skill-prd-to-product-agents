/// Integration tests for the bootstrap → validate cycle.
///
/// These tests confirm that the required_files in validate.rs exactly match
/// the files produced by the bootstrap templates, preventing split-brain
/// regressions between contract and implementation.
use std::env;
use std::path::{Path, PathBuf};

/// Resolve the skill root from the test environment.
fn skill_root() -> PathBuf {
    if let Some(explicit) = env::var_os("PRDTP_SKILL_ROOT").or_else(|| env::var_os("SKILL_ROOT")) {
        return normalize_skill_root(PathBuf::from(explicit));
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest
        .parent() // cli-tools/
        .and_then(|p| p.parent()) // repo root
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

fn is_bootstrap_generated(path: &str) -> bool {
    prdtp_agents_shared::workspace_paths::BOOTSTRAP_GENERATED_FILES.contains(&path)
}

/// Verify every path in workspace_paths::REQUIRED_FILES exists in the template.
#[test]
fn required_files_match_template() {
    let template_root = template_root();
    assert!(
        template_root.is_dir(),
        "template root not found: {}",
        template_root.display()
    );

    let mut missing = Vec::new();
    for path in prdtp_agents_shared::workspace_paths::REQUIRED_FILES {
        let full = template_root.join(path);
        if !full.exists() && !is_bootstrap_generated(path) {
            missing.push(path.to_string());
        }
    }

    assert!(
        missing.is_empty(),
        "REQUIRED_FILES references paths not present in templates/workspace/:\n  {}",
        missing.join("\n  ")
    );
}

/// Verify every path in workspace_paths::EXTENDED_REQUIRED_FILES exists in the template.
#[test]
fn extended_required_files_match_template() {
    let template_root = template_root();
    assert!(
        template_root.is_dir(),
        "template root not found: {}",
        template_root.display()
    );

    let mut missing = Vec::new();
    for path in prdtp_agents_shared::workspace_paths::EXTENDED_REQUIRED_FILES {
        let full = template_root.join(path);
        if !full.exists() {
            missing.push(path.to_string());
        }
    }

    assert!(
        missing.is_empty(),
        "EXTENDED_REQUIRED_FILES references paths not present in templates/workspace/:\n  {}",
        missing.join("\n  ")
    );
}

/// Verify every YAML file in workspace_paths::YAML_FILES exists in the template.
#[test]
fn yaml_files_match_template() {
    let template_root = template_root();

    let mut missing = Vec::new();
    for path in prdtp_agents_shared::workspace_paths::YAML_FILES {
        let full = template_root.join(path);
        if !full.exists() && !is_bootstrap_generated(path) {
            missing.push(path.to_string());
        }
    }

    assert!(
        missing.is_empty(),
        "YAML_FILES references paths not present in templates/workspace/:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn workspace_capabilities_seed_exists_and_uses_schema_v2() {
    let capabilities_path = template_root()
        .join(".github")
        .join("workspace-capabilities.yaml");

    assert!(
        capabilities_path.is_file(),
        "workspace-capabilities.yaml seed missing from template: {}",
        capabilities_path.display()
    );

    let raw = std::fs::read_to_string(&capabilities_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", capabilities_path.display()));
    let parsed: serde_yaml::Value = serde_yaml::from_str(&raw)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", capabilities_path.display()));

    assert_eq!(parsed["schema_version"].as_u64(), Some(2));
    assert!(
        parsed["capabilities"]["git"]["authorized"]["enabled"]
            .as_bool()
            .is_some(),
        "workspace-capabilities seed must expose capabilities.*.authorized.enabled"
    );
}

/// Verify every agent name in workspace_paths::AGENT_NAMES has .agent.md in the template.
#[test]
fn agent_names_match_template() {
    let agents_dir = skill_root()
        .join("templates")
        .join("workspace")
        .join(".github")
        .join("agents");

    let mut missing = Vec::new();
    for name in prdtp_agents_shared::workspace_paths::AGENT_NAMES {
        let agent_file = agents_dir.join(format!("{name}.agent.md"));
        if !agent_file.exists() {
            missing.push(format!("{name}.agent.md"));
        }
    }

    assert!(
        missing.is_empty(),
        "AGENT_NAMES references agents not present in templates/workspace/.github/agents/:\n  {}",
        missing.join("\n  ")
    );
}

/// Verify IMMUTABLE_FILES_PATH exists in the template.
#[test]
fn immutable_files_path_exists_in_template() {
    let template_root = template_root();
    let full = template_root.join(prdtp_agents_shared::workspace_paths::IMMUTABLE_FILES_PATH);

    assert!(
        full.exists(),
        "IMMUTABLE_FILES_PATH '{}' not found in template",
        prdtp_agents_shared::workspace_paths::IMMUTABLE_FILES_PATH
    );
}

#[test]
fn skill_root_env_var_accepts_repo_root_or_skill_root() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("could not resolve repository root from CARGO_MANIFEST_DIR");

    let from_repo_root = normalize_skill_root(repo_root.to_path_buf());
    let from_skill_root = normalize_skill_root(from_repo_root.clone());

    assert_eq!(from_repo_root, from_skill_root);
    assert!(Path::new(&from_repo_root).join("SKILL.md").is_file());
}
