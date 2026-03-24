use crate::model::Snapshot;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub fn snapshot_dir(project_root: &Path) -> PathBuf {
    project_root.join(".project-memory")
}

pub fn snapshot_path(project_root: &Path) -> PathBuf {
    snapshot_dir(project_root).join("snapshot.json")
}

pub fn load_snapshot(project_root: &Path) -> Result<Snapshot> {
    let path = snapshot_path(project_root);
    let content = fs::read_to_string(&path)
        .with_context(|| format!("snapshot not found at {}; run ingest first", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("parsing snapshot {}", path.display()))
}

pub fn save_snapshot(project_root: &Path, snapshot: &Snapshot) -> Result<PathBuf> {
    let path = snapshot_path(project_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating snapshot directory {}", parent.display()))?;
    }

    let content = serde_json::to_string_pretty(snapshot)?;
    fs::write(&path, content).with_context(|| format!("writing snapshot {}", path.display()))?;
    Ok(path)
}
