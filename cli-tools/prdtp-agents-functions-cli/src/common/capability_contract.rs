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

    let content = fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;
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
        &["capabilities", capability, "policy", "enabled"],
    ))
}

pub fn require_policy_enabled(
    workspace: &Path,
    capability: &str,
    command_label: &str,
) -> Result<()> {
    match policy_enabled(workspace, capability)? {
        Some(true) | None => Ok(()),
        Some(false) => bail!(
            "{command_label} is out of contract because capabilities.{capability}.policy.enabled=false in {}",
            workspace.join(CAPABILITIES_PATH).display()
        ),
    }
}

pub fn capabilities_file_exists(workspace: &Path) -> bool {
    workspace.join(CAPABILITIES_PATH).is_file()
}
