use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::yaml_ops;

const STATE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HandoffEntry {
    pub id: String,
    pub from: String,
    pub to: String,
    #[serde(rename = "type")]
    pub handoff_type: String,
    pub entity: String,
    pub reason: String,
    pub status: String,
    pub created: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FindingEntry {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(rename = "type")]
    pub finding_type: String,
    pub severity: String,
    pub entity: String,
    pub title: String,
    pub status: String,
    pub created: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReleaseEntry {
    pub id: String,
    pub name: String,
    pub target_date: String,
    pub agent_role: String,
    pub created: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stories: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HandoffsDocument {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub handoffs: Vec<HandoffEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FindingsDocument {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub findings: Vec<FindingEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReleasesDocument {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub releases: Vec<ReleaseEntry>,
}

impl Default for HandoffsDocument {
    fn default() -> Self {
        Self {
            schema_version: STATE_SCHEMA_VERSION,
            revision: 0,
            handoffs: Vec::new(),
        }
    }
}

impl Default for FindingsDocument {
    fn default() -> Self {
        Self {
            schema_version: STATE_SCHEMA_VERSION,
            revision: 0,
            findings: Vec::new(),
        }
    }
}

impl Default for ReleasesDocument {
    fn default() -> Self {
        Self {
            schema_version: STATE_SCHEMA_VERSION,
            revision: 0,
            releases: Vec::new(),
        }
    }
}

pub fn mutate_handoffs<T>(
    workspace: &Path,
    action: impl FnOnce(&mut HandoffsDocument) -> Result<T>,
) -> Result<T> {
    mutate_document(
        workspace.join("docs/project/handoffs.yaml"),
        handoffs_header(),
        action,
    )
}

pub fn mutate_findings<T>(
    workspace: &Path,
    action: impl FnOnce(&mut FindingsDocument) -> Result<T>,
) -> Result<T> {
    mutate_document(
        workspace.join("docs/project/findings.yaml"),
        findings_header(),
        action,
    )
}

pub fn mutate_releases<T>(
    workspace: &Path,
    action: impl FnOnce(&mut ReleasesDocument) -> Result<T>,
) -> Result<T> {
    mutate_document(
        workspace.join("docs/project/releases.yaml"),
        releases_header(),
        action,
    )
}

fn mutate_document<TDoc, TResult>(
    path: PathBuf,
    header: &'static str,
    action: impl FnOnce(&mut TDoc) -> Result<TResult>,
) -> Result<TResult>
where
    TDoc: Default + Serialize + DeserializeOwned + DocumentRevision,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let _lock = yaml_ops::YamlLock::acquire(&path)?;
    let mut document = load_document::<TDoc>(&path)?;
    document.ensure_schema();
    let result = action(&mut document)?;
    document.bump_revision();
    let rendered = render_document(header, &document)?;
    yaml_ops::atomic_write(&path, &rendered)?;
    Ok(result)
}

fn load_document<TDoc>(path: &Path) -> Result<TDoc>
where
    TDoc: Default + DeserializeOwned + DocumentRevision,
{
    if !path.exists() {
        return Ok(TDoc::default());
    }

    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let normalized = yaml_ops::normalize_lf(&raw);
    let mut parsed = serde_yaml::from_str::<TDoc>(&normalized)
        .with_context(|| format!("parsing {}", path.display()))?;
    parsed.ensure_schema();
    Ok(parsed)
}

fn render_document<TDoc>(header: &str, document: &TDoc) -> Result<String>
where
    TDoc: Serialize,
{
    let body = serde_yaml::to_string(document)?;
    let body = body.trim_start_matches("---\n");
    Ok(format!("{header}\n\n{body}"))
}

fn default_schema_version() -> u32 {
    STATE_SCHEMA_VERSION
}

fn handoffs_header() -> &'static str {
    "# Handoff queue - machine-managed canonical state\n# Steward: pm-orchestrator\n# Comments outside this fixed header are not preserved."
}

fn findings_header() -> &'static str {
    "# Findings register - machine-managed canonical state\n# Steward: qa-lead\n# Comments outside this fixed header are not preserved."
}

fn releases_header() -> &'static str {
    "# Release tracker - machine-managed canonical state\n# Steward: devops-release-engineer\n# Comments outside this fixed header are not preserved."
}

pub trait DocumentRevision {
    fn ensure_schema(&mut self);
    fn bump_revision(&mut self);
}

impl DocumentRevision for HandoffsDocument {
    fn ensure_schema(&mut self) {
        if self.schema_version == 0 {
            self.schema_version = STATE_SCHEMA_VERSION;
        }
    }

    fn bump_revision(&mut self) {
        self.schema_version = STATE_SCHEMA_VERSION;
        self.revision += 1;
    }
}

impl DocumentRevision for FindingsDocument {
    fn ensure_schema(&mut self) {
        if self.schema_version == 0 {
            self.schema_version = STATE_SCHEMA_VERSION;
        }
    }

    fn bump_revision(&mut self) {
        self.schema_version = STATE_SCHEMA_VERSION;
        self.revision += 1;
    }
}

impl DocumentRevision for ReleasesDocument {
    fn ensure_schema(&mut self) {
        if self.schema_version == 0 {
            self.schema_version = STATE_SCHEMA_VERSION;
        }
    }

    fn bump_revision(&mut self) {
        self.schema_version = STATE_SCHEMA_VERSION;
        self.revision += 1;
    }
}
