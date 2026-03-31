use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const CAPABILITIES_SCHEMA_VERSION: u8 = 2;

#[derive(Debug, Clone)]
pub struct CapabilitySnapshotInput {
    pub host: String,
    pub os: String,
    pub git_installed: bool,
    pub git_identity_configured: bool,
    pub git_authorized: bool,
    pub git_authorization_source: String,
    pub git_mode: String,
    pub gh_installed: bool,
    pub gh_authenticated: bool,
    pub gh_authorized: bool,
    pub gh_authorization_source: String,
    pub sqlite_installed: bool,
    pub db_initialized: bool,
    pub sqlite_authorized: bool,
    pub sqlite_authorization_source: String,
    pub sqlite_mode: String,
    pub node_installed: bool,
    pub npm_installed: bool,
    pub node_native_linux: bool,
    pub markdownlint_installed: bool,
    pub markdownlint_native_linux: bool,
    pub markdownlint_authorized: bool,
    pub markdownlint_authorization_source: String,
    pub local_history_authorized: bool,
    pub local_history_authorization_source: String,
    pub local_history_format: String,
    pub local_history_path: String,
    pub reporting_ui_available: bool,
    pub reporting_xlsx_export_ready: bool,
    pub reporting_pdf_export_ready: bool,
    pub reporting_authorized: bool,
    pub reporting_authorization_source: String,
    pub reporting_visibility_mode: String,
    pub last_updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesDocument {
    pub schema_version: u8,
    pub environment: EnvironmentInfo,
    pub capabilities: Capabilities,
    pub last_updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    pub host: String,
    pub os: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub git: GitCapability,
    pub gh: GhCapability,
    pub sqlite: SqliteCapability,
    pub node: NodeCapability,
    pub markdownlint: MarkdownlintCapability,
    pub local_history: LocalHistoryCapability,
    pub reporting: ReportingCapability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCapability {
    pub detected: GitDetected,
    pub authorized: AuthorizationState,
    pub policy: GitPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDetected {
    pub installed: bool,
    pub identity_configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitPolicy {
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhCapability {
    pub detected: GhDetected,
    pub authorized: AuthorizationState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhDetected {
    pub installed: bool,
    pub authenticated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteCapability {
    pub detected: SqliteDetected,
    pub authorized: AuthorizationState,
    pub policy: SqlitePolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteDetected {
    pub installed: bool,
    pub db_initialized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlitePolicy {
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapability {
    pub detected: NodeDetected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDetected {
    pub installed: bool,
    pub npm_installed: bool,
    pub native_linux: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownlintCapability {
    pub detected: MarkdownlintDetected,
    pub authorized: AuthorizationState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownlintDetected {
    pub installed: bool,
    pub native_linux: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalHistoryCapability {
    pub authorized: AuthorizationState,
    pub policy: LocalHistoryPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalHistoryPolicy {
    pub format: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingCapability {
    pub detected: ReportingDetected,
    pub authorized: AuthorizationState,
    pub policy: ReportingPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingDetected {
    pub ui_available: bool,
    pub xlsx_export_ready: bool,
    pub pdf_export_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingPolicy {
    pub visibility_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationState {
    pub enabled: bool,
    pub source: String,
}

impl CapabilitySnapshotInput {
    pub fn into_document(self) -> CapabilitiesDocument {
        CapabilitiesDocument {
            schema_version: CAPABILITIES_SCHEMA_VERSION,
            environment: EnvironmentInfo {
                host: self.host,
                os: self.os,
            },
            capabilities: Capabilities {
                git: GitCapability {
                    detected: GitDetected {
                        installed: self.git_installed,
                        identity_configured: self.git_identity_configured,
                    },
                    authorized: AuthorizationState {
                        enabled: self.git_authorized,
                        source: self.git_authorization_source,
                    },
                    policy: GitPolicy {
                        mode: self.git_mode,
                    },
                },
                gh: GhCapability {
                    detected: GhDetected {
                        installed: self.gh_installed,
                        authenticated: self.gh_authenticated,
                    },
                    authorized: AuthorizationState {
                        enabled: self.gh_authorized,
                        source: self.gh_authorization_source,
                    },
                },
                sqlite: SqliteCapability {
                    detected: SqliteDetected {
                        installed: self.sqlite_installed,
                        db_initialized: self.db_initialized,
                    },
                    authorized: AuthorizationState {
                        enabled: self.sqlite_authorized,
                        source: self.sqlite_authorization_source,
                    },
                    policy: SqlitePolicy {
                        mode: self.sqlite_mode,
                    },
                },
                node: NodeCapability {
                    detected: NodeDetected {
                        installed: self.node_installed,
                        npm_installed: self.npm_installed,
                        native_linux: self.node_native_linux,
                    },
                },
                markdownlint: MarkdownlintCapability {
                    detected: MarkdownlintDetected {
                        installed: self.markdownlint_installed,
                        native_linux: self.markdownlint_native_linux,
                    },
                    authorized: AuthorizationState {
                        enabled: self.markdownlint_authorized,
                        source: self.markdownlint_authorization_source,
                    },
                },
                local_history: LocalHistoryCapability {
                    authorized: AuthorizationState {
                        enabled: self.local_history_authorized,
                        source: self.local_history_authorization_source,
                    },
                    policy: LocalHistoryPolicy {
                        format: self.local_history_format,
                        path: self.local_history_path,
                    },
                },
                reporting: ReportingCapability {
                    detected: ReportingDetected {
                        ui_available: self.reporting_ui_available,
                        xlsx_export_ready: self.reporting_xlsx_export_ready,
                        pdf_export_ready: self.reporting_pdf_export_ready,
                    },
                    authorized: AuthorizationState {
                        enabled: self.reporting_authorized,
                        source: self.reporting_authorization_source,
                    },
                    policy: ReportingPolicy {
                        visibility_mode: self.reporting_visibility_mode,
                    },
                },
            },
            last_updated: self.last_updated,
        }
    }
}

pub fn render_capabilities_yaml(input: CapabilitySnapshotInput) -> Result<String, serde_yaml::Error> {
    serde_yaml::to_string(&input.into_document())
}

pub fn render_bootstrap_seed_capabilities_yaml() -> Result<String, serde_yaml::Error> {
    render_capabilities_yaml(CapabilitySnapshotInput {
        host: "unknown".to_string(),
        os: "unknown".to_string(),
        git_installed: false,
        git_identity_configured: false,
        git_authorized: false,
        git_authorization_source: "manual-default-deny".to_string(),
        git_mode: "local-only".to_string(),
        gh_installed: false,
        gh_authenticated: false,
        gh_authorized: false,
        gh_authorization_source: "manual-default-deny".to_string(),
        sqlite_installed: false,
        db_initialized: false,
        sqlite_authorized: false,
        sqlite_authorization_source: "missing-runtime".to_string(),
        sqlite_mode: "spool-only".to_string(),
        node_installed: false,
        npm_installed: false,
        node_native_linux: false,
        markdownlint_installed: false,
        markdownlint_native_linux: false,
        markdownlint_authorized: false,
        markdownlint_authorization_source: "missing-runtime".to_string(),
        local_history_authorized: true,
        local_history_authorization_source: "detected-default".to_string(),
        local_history_format: "markdown+json".to_string(),
        local_history_path: ".state/local-history".to_string(),
        reporting_ui_available: false,
        reporting_xlsx_export_ready: false,
        reporting_pdf_export_ready: false,
        reporting_authorized: true,
        reporting_authorization_source: "detected-default".to_string(),
        reporting_visibility_mode: "local-only".to_string(),
        last_updated: "1970-01-01T00:00:00Z".to_string(),
    })
}

pub fn read_capabilities_document(path: &Path) -> Result<CapabilitiesDocument> {
    let content = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_yaml::from_str(&content).with_context(|| format!("parsing {}", path.display()))
}

pub fn write_capabilities_document(path: &Path, document: &CapabilitiesDocument) -> Result<()> {
    let content = serde_yaml::to_string(document)
        .with_context(|| format!("rendering {}", path.display()))?;
    fs::write(path, content).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}
