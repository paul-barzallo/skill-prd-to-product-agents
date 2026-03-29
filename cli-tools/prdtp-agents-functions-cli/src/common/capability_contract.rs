use anyhow::{bail, Context, Result};
use serde_yaml::Value;
use std::fs;
use std::path::Path;

const CAPABILITIES_PATH: &str = ".github/workspace-capabilities.yaml";

fn load_capabilities(workspace: &Path) -> Result<Option<Value>> {
    let path = workspace.join(CAPABILITIES_PATH);
    if !path.exists() {
        return Ok(None);
    }

    let content =
        fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let yaml = serde_yaml::from_str::<Value>(&content)
        .with_context(|| format!("parsing {}", path.display()))?;
    Ok(Some(yaml))
}

fn yaml_get_bool(yaml: &Value, keys: &[&str]) -> Option<bool> {
    let mut current = yaml;
    for key in keys {
        current = current.get(*key)?;
    }
    current.as_bool()
}

pub fn policy_enabled(workspace: &Path, capability: &str) -> Result<Option<bool>> {
    let Some(yaml) = load_capabilities(workspace)? else {
        return Ok(None);
    };

    Ok(yaml_get_bool(
        &yaml,
        &["capabilities", capability, "authorized", "enabled"],
    )
    .or_else(|| yaml_get_bool(&yaml, &["capabilities", capability, "policy", "enabled"])))
}

pub fn require_policy_enabled(
    workspace: &Path,
    capability: &str,
    command_label: &str,
) -> Result<()> {
    if !capabilities_file_exists(workspace) {
        bail!(
            "{command_label} is out of contract because {} is missing. Run `prdtp-agents-functions-cli capabilities detect` first.",
            workspace.join(CAPABILITIES_PATH).display()
        );
    }

    match policy_enabled(workspace, capability)? {
        Some(true) => Ok(()),
        Some(false) => bail!(
            "{command_label} is out of contract because capabilities.{capability}.authorized.enabled=false in {}",
            workspace.join(CAPABILITIES_PATH).display()
        ),
        None => bail!(
            "{command_label} is out of contract because capabilities.{capability}.authorized.enabled is missing in {}",
            workspace.join(CAPABILITIES_PATH).display()
        ),
    }
}

pub fn capabilities_file_exists(workspace: &Path) -> bool {
    workspace.join(CAPABILITIES_PATH).is_file()
}

#[cfg(test)]
mod tests {
    use super::{policy_enabled, require_policy_enabled, CAPABILITIES_PATH};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn require_policy_enabled_fails_closed_when_capabilities_file_is_missing() {
        let workspace = tempdir().expect("failed to create temp workspace");
        let error = require_policy_enabled(workspace.path(), "git", "git checkout-task-branch")
            .expect_err("missing capability file must fail closed");

        let text = format!("{error:#}");
        assert!(text.contains("workspace-capabilities.yaml is missing"));
        assert!(text.contains("capabilities detect"));
    }

    #[test]
    fn require_policy_enabled_fails_when_policy_entry_is_missing() {
        let workspace = tempdir().expect("failed to create temp workspace");
        let github_dir = workspace.path().join(".github");
        fs::create_dir_all(&github_dir).expect("failed to create .github directory");
        fs::write(
            workspace.path().join(CAPABILITIES_PATH),
            "schema_version: 1\ncapabilities:\n  git:\n    detected:\n      installed: true\n",
        )
        .expect("failed to write capability file");

        let error = require_policy_enabled(workspace.path(), "git", "git checkout-task-branch")
            .expect_err("missing policy entry must fail closed");

        assert!(format!("{error:#}").contains("capabilities.git.authorized.enabled is missing"));
    }

    #[test]
    fn require_policy_enabled_allows_enabled_policy() {
        let workspace = tempdir().expect("failed to create temp workspace");
        let github_dir = workspace.path().join(".github");
        fs::create_dir_all(&github_dir).expect("failed to create .github directory");
        fs::write(
            workspace.path().join(CAPABILITIES_PATH),
            "schema_version: 2\ncapabilities:\n  reporting:\n    authorized:\n      enabled: true\n      source: explicit\n",
        )
        .expect("failed to write capability file");

        require_policy_enabled(workspace.path(), "reporting", "report dashboard")
            .expect("enabled policy should pass");
        assert_eq!(policy_enabled(workspace.path(), "reporting").unwrap(), Some(true));
    }
}
