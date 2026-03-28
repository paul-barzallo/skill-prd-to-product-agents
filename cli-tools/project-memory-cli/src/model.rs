use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum FileType {
    Prd,
    Readme,
    Adr,
    Spec,
    Prompt,
    Skill,
    Markdown,
    Yaml,
    Json,
    Toml,
    RustSource,
    Source,
    Config,
    Text,
    OtherText,
}

impl FileType {
    pub fn is_requirement_source(&self) -> bool {
        matches!(self, Self::Prd | Self::Spec | Self::Adr | Self::Readme | Self::Markdown)
    }

    pub fn is_code_like(&self) -> bool {
        matches!(self, Self::RustSource | Self::Source)
    }
}

impl fmt::Display for FileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Prd => "prd",
            Self::Readme => "readme",
            Self::Adr => "adr",
            Self::Spec => "spec",
            Self::Prompt => "prompt",
            Self::Skill => "skill",
            Self::Markdown => "markdown",
            Self::Yaml => "yaml",
            Self::Json => "json",
            Self::Toml => "toml",
            Self::RustSource => "rust_source",
            Self::Source => "source",
            Self::Config => "config",
            Self::Text => "text",
            Self::OtherText => "other_text",
        };

        write!(f, "{value}")
    }
}

impl FromStr for FileType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "prd" => Ok(Self::Prd),
            "readme" => Ok(Self::Readme),
            "adr" => Ok(Self::Adr),
            "spec" => Ok(Self::Spec),
            "prompt" => Ok(Self::Prompt),
            "skill" => Ok(Self::Skill),
            "markdown" | "md" => Ok(Self::Markdown),
            "yaml" | "yml" => Ok(Self::Yaml),
            "json" => Ok(Self::Json),
            "toml" => Ok(Self::Toml),
            "rust" | "rust_source" | "rs" => Ok(Self::RustSource),
            "source" | "code" => Ok(Self::Source),
            "config" => Ok(Self::Config),
            "text" | "txt" => Ok(Self::Text),
            "other_text" => Ok(Self::OtherText),
            _ => Err(format!("unsupported file type filter: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Module,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolRecord {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkKind {
    Section,
    Window,
}

impl fmt::Display for ChunkKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Section => "section",
            Self::Window => "window",
        };

        write!(f, "{value}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkRecord {
    pub chunk_id: String,
    pub kind: ChunkKind,
    pub ordinal: usize,
    pub title: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkEmbeddingRecord {
    pub chunk_id: String,
    pub provider: String,
    pub model: Option<String>,
    pub dimensions: usize,
    pub content_hash: String,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub path: String,
    pub file_type: FileType,
    pub bytes: usize,
    pub lines: usize,
    pub hash: String,
    pub title: Option<String>,
    pub content: String,
    #[serde(default)]
    pub chunks: Vec<ChunkRecord>,
    pub requirement_ids: Vec<String>,
    #[serde(default)]
    pub requirement_references: BTreeMap<String, Vec<String>>,
    pub referenced_paths: Vec<String>,
    #[serde(default)]
    pub symbols: Vec<SymbolRecord>,
    #[serde(default)]
    pub imports: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Requirement,
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeRef {
    pub kind: NodeKind,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    DeclaredIn,
    MentionedIn,
    Covers,
    ReferencesFile,
    ReferencesArtifact,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EdgeStatus {
    Present,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvidence {
    pub source_path: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEdge {
    pub source: NodeRef,
    pub target: NodeRef,
    pub edge_type: EdgeType,
    pub status: EdgeStatus,
    pub evidence: TraceEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotStats {
    pub files_indexed: usize,
    pub requirements_detected: usize,
    pub trace_edges: usize,
    pub skipped_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub schema_version: String,
    pub project_root: String,
    pub generated_at: String,
    pub files: Vec<FileRecord>,
    pub trace_edges: Vec<TraceEdge>,
    pub stats: SnapshotStats,
}

#[derive(Debug, Clone, Serialize)]
pub struct IngestReport {
    pub snapshot_path: String,
    pub embedding_provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    pub files_indexed: usize,
    pub changed_files: usize,
    pub reused_files: usize,
    pub deleted_files: usize,
    pub skipped_files: usize,
    pub requirements_detected: usize,
    pub trace_edges: usize,
    pub validation_findings: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryMatch {
    pub path: String,
    pub file_type: FileType,
    pub score: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lexical_score: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_score: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_kind: Option<ChunkKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
    pub line_number: Option<usize>,
    pub snippet: String,
    pub requirement_ids: Vec<String>,
    pub symbols: Vec<String>,
    pub imports: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryReport {
    pub query: Option<String>,
    pub symbol: Option<String>,
    pub import: Option<String>,
    pub file_type: Option<String>,
    pub path_contains: Option<String>,
    pub total_matches: usize,
    pub returned_matches: usize,
    pub results: Vec<QueryMatch>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetrieveReport {
    pub query: String,
    pub retrieval_mode: &'static str,
    pub configured_embedding_provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configured_embedding_model: Option<String>,
    pub embedding_provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    pub remote_access: bool,
    pub cost_risk: String,
    pub cache_status: String,
    pub fallback_used: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
    pub file_type: Option<String>,
    pub path_contains: Option<String>,
    pub total_matches: usize,
    pub returned_matches: usize,
    pub results: Vec<QueryMatch>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WatchIteration {
    pub sequence: usize,
    pub observed_paths: Vec<String>,
    pub changed_paths: Vec<String>,
    pub deleted_paths: Vec<String>,
    pub ingest: IngestReport,
}

#[derive(Debug, Clone, Serialize)]
pub struct WatchReport {
    pub initial_snapshot_created: bool,
    pub max_events: usize,
    pub interval_ms: u64,
    pub timeout_ms: Option<u64>,
    pub timed_out: bool,
    pub events_observed: usize,
    pub iterations: Vec<WatchIteration>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceFilters {
    pub requirement: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceReport {
    pub filters: TraceFilters,
    pub edge_count: usize,
    pub unresolved_edges: usize,
    pub edges: Vec<TraceEdge>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImpactReport {
    pub node: String,
    pub node_kind: String,
    pub edge_count: usize,
    pub impacted_nodes: Vec<NodeRef>,
    pub edges: Vec<TraceEdge>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationFinding {
    pub rule: String,
    pub severity: Severity,
    pub message: String,
    pub source: NodeRef,
    pub evidence_path: String,
    pub related: Vec<NodeRef>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationSummary {
    pub errors: usize,
    pub warnings: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    pub fail_on_warnings: bool,
    pub summary: ValidationSummary,
    pub findings: Vec<ValidationFinding>,
}
