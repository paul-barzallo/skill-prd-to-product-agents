/// Integration tests for the bootstrap → validate cycle.
///
/// These tests confirm that the required_files in validate.rs exactly match
/// the files produced by the bootstrap templates, preventing split-brain
/// regressions between contract and implementation.
use std::path::PathBuf;

/// Resolve the skill root from the test environment.
fn skill_root() -> PathBuf {
    // The test binary runs from the crate root; skill root is two levels up.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent() // cli-tools/
        .and_then(|p| p.parent()) // repo root
        .expect("could not resolve skill root from CARGO_MANIFEST_DIR")
        .join(".agents")
        .join("skills")
        .join("prd-to-product-agents")
}

/// Verify every path in workspace_paths::REQUIRED_FILES exists in the template.
#[test]
fn required_files_match_template() {
    let template_root = skill_root().join("templates").join("workspace");
    assert!(
        template_root.is_dir(),
        "template root not found: {}",
        template_root.display()
    );

    let mut missing = Vec::new();
    for path in prdtp_agents_shared::workspace_paths::REQUIRED_FILES {
        let full = template_root.join(path);
        if !full.exists() {
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
    let template_root = skill_root().join("templates").join("workspace");
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
    let template_root = skill_root().join("templates").join("workspace");

    let mut missing = Vec::new();
    for path in prdtp_agents_shared::workspace_paths::YAML_FILES {
        let full = template_root.join(path);
        if !full.exists() {
            missing.push(path.to_string());
        }
    }

    assert!(
        missing.is_empty(),
        "YAML_FILES references paths not present in templates/workspace/:\n  {}",
        missing.join("\n  ")
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
    let template_root = skill_root().join("templates").join("workspace");
    let full = template_root.join(prdtp_agents_shared::workspace_paths::IMMUTABLE_FILES_PATH);

    assert!(
        full.exists(),
        "IMMUTABLE_FILES_PATH '{}' not found in template",
        prdtp_agents_shared::workspace_paths::IMMUTABLE_FILES_PATH
    );
}
