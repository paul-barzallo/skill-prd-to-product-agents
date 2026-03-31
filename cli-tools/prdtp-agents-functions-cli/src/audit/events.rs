use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use serde_yaml::Value as YamlValue;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

const AUDIT_LOG_PATH: &str = ".state/audit/sensitive-actions.jsonl";

#[derive(Serialize)]
struct SensitiveActionEnvelope<'a> {
    event_id: &'a str,
    timestamp_utc: &'a str,
    correlation_id: &'a str,
    action: &'a str,
    actor: &'a str,
    source: &'a str,
    outcome: &'a str,
    payload: &'a JsonValue,
    previous_hash: &'a Option<String>,
    ack_id: &'a Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SensitiveActionRecord {
    pub event_id: String,
    pub timestamp_utc: String,
    pub correlation_id: String,
    pub action: String,
    pub actor: String,
    pub source: String,
    pub outcome: String,
    pub payload: JsonValue,
    pub previous_hash: Option<String>,
    pub ack_id: Option<String>,
    pub event_hash: String,
}

pub fn record_sensitive_action(
    workspace: &Path,
    action: &str,
    actor: &str,
    outcome: &str,
    payload: JsonValue,
) -> Result<()> {
    let governance = load_governance(workspace)?;
    let mode = governance
        .as_ref()
        .map(crate::github_api::audit_mode)
        .transpose()?
        .unwrap_or(crate::github_api::AuditMode::LocalHashchain);

    let event_id = default_event_id();
    let timestamp_utc = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let correlation_id = std::env::var("PRDTP_AUDIT_CORRELATION_ID")
        .unwrap_or_else(|_| default_correlation_id());
    let source = std::env::var("PRDTP_AUDIT_SOURCE").unwrap_or_else(|_| "workspace-cli".to_string());
    let previous_hash = read_last_hash(workspace)?;

    let mut record = SensitiveActionRecord {
        event_id,
        timestamp_utc,
        correlation_id,
        action: action.to_string(),
        actor: actor.to_string(),
        source,
        outcome: outcome.to_string(),
        payload,
        previous_hash,
        ack_id: None,
        event_hash: String::new(),
    };

    if mode == crate::github_api::AuditMode::Remote {
        let governance = governance
            .as_ref()
            .context("audit.mode=remote requires .github/github-governance.yaml")?;
        let ack_id = post_remote_event(governance, &record)?;
        if ack_id.trim().is_empty() {
            bail!("audit sink response did not include a persistent ack_id");
        }
        record.ack_id = Some(ack_id);
    }

    record.event_hash = compute_event_hash(&record)?;
    append_record(workspace, &record)?;
    Ok(())
}

pub fn verify_local_hashchain(workspace: &Path) -> Result<usize> {
    let path = audit_log_path(workspace);
    if !path.is_file() {
        return Ok(0);
    }

    let file = fs::File::open(&path).with_context(|| format!("opening {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut previous_hash: Option<String> = None;
    let mut count = 0usize;

    for (line_number, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("reading {}", path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        let record: SensitiveActionRecord = serde_json::from_str(&line)
            .with_context(|| format!("parsing {} line {}", path.display(), line_number + 1))?;
        if record.previous_hash != previous_hash {
            bail!(
                "audit hash-chain mismatch at {} line {}: expected previous_hash={:?}, found {:?}",
                path.display(),
                line_number + 1,
                previous_hash,
                record.previous_hash
            );
        }
        let computed = compute_event_hash(&record)?;
        if computed != record.event_hash {
            bail!(
                "audit hash mismatch at {} line {}: expected {}, found {}",
                path.display(),
                line_number + 1,
                computed,
                record.event_hash
            );
        }
        previous_hash = Some(record.event_hash.clone());
        count += 1;
    }

    Ok(count)
}

pub fn current_audit_mode(workspace: &Path) -> Result<crate::github_api::AuditMode> {
    Ok(load_governance(workspace)?
        .as_ref()
        .map(crate::github_api::audit_mode)
        .transpose()?
        .unwrap_or(crate::github_api::AuditMode::LocalHashchain))
}

pub fn remote_sink_health(workspace: &Path) -> Result<Option<String>> {
    let Some(governance) = load_governance(workspace)? else {
        return Ok(None);
    };
    if crate::github_api::audit_mode(&governance)? != crate::github_api::AuditMode::Remote {
        return Ok(None);
    }
    let config = crate::github_api::audit_remote_config(&governance)?
        .context("audit.mode=remote requires audit.remote.* to be configured")?;
    let header_value = std::env::var(&config.auth_header_env).with_context(|| {
        format!(
            "audit remote auth env '{}' is not set",
            config.auth_header_env
        )
    })?;
    if header_value.trim().is_empty() {
        bail!(
            "audit remote auth env '{}' is empty",
            config.auth_header_env
        );
    }
    Ok(Some(config.endpoint))
}

fn compute_event_hash(record: &SensitiveActionRecord) -> Result<String> {
    let envelope = SensitiveActionEnvelope {
        event_id: &record.event_id,
        timestamp_utc: &record.timestamp_utc,
        correlation_id: &record.correlation_id,
        action: &record.action,
        actor: &record.actor,
        source: &record.source,
        outcome: &record.outcome,
        payload: &record.payload,
        previous_hash: &record.previous_hash,
        ack_id: &record.ack_id,
    };
    let serialized = serde_json::to_vec(&envelope)?;
    let mut hasher = Sha256::new();
    hasher.update(serialized);
    Ok(format!("{:x}", hasher.finalize()))
}

fn post_remote_event(governance: &YamlValue, record: &SensitiveActionRecord) -> Result<String> {
    let config = crate::github_api::audit_remote_config(governance)?
        .context("audit.mode=remote requires audit.remote.* to be configured")?;
    let header_value = std::env::var(&config.auth_header_env).with_context(|| {
        format!(
            "audit remote auth env '{}' is not set",
            config.auth_header_env
        )
    })?;
    if header_value.trim().is_empty() {
        bail!(
            "audit remote auth env '{}' is empty",
            config.auth_header_env
        );
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout_seconds))
        .build()
        .context("building audit sink client")?;

    let response = client
        .post(&config.endpoint)
        .header("Content-Type", "application/json")
        .header("Authorization", header_value)
        .json(&json!({
            "event_id": record.event_id,
            "timestamp_utc": record.timestamp_utc,
            "correlation_id": record.correlation_id,
            "action": record.action,
            "actor": record.actor,
            "source": record.source,
            "outcome": record.outcome,
            "payload": record.payload,
            "previous_hash": record.previous_hash,
        }))
        .send()
        .with_context(|| format!("posting audit event to {}", config.endpoint))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        if body.trim().is_empty() {
            bail!("audit sink {} failed with HTTP {status}", config.endpoint);
        }
        bail!(
            "audit sink {} failed with HTTP {status}: {}",
            config.endpoint,
            body.trim()
        );
    }

    let payload: JsonValue = response
        .json()
        .with_context(|| format!("parsing audit sink response from {}", config.endpoint))?;
    payload["ack_id"]
        .as_str()
        .map(str::to_string)
        .filter(|value| !value.trim().is_empty())
        .context("audit sink response missing ack_id")
}

fn load_governance(workspace: &Path) -> Result<Option<YamlValue>> {
    let path = workspace.join(".github/github-governance.yaml");
    if !path.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let parsed =
        serde_yaml::from_str::<YamlValue>(&raw).with_context(|| format!("parsing {}", path.display()))?;
    Ok(Some(parsed))
}

fn append_record(workspace: &Path, record: &SensitiveActionRecord) -> Result<()> {
    let path = audit_log_path(workspace);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("opening {}", path.display()))?;
    writeln!(file, "{}", serde_json::to_string(record)?)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn read_last_hash(workspace: &Path) -> Result<Option<String>> {
    let path = audit_log_path(workspace);
    if !path.is_file() {
        return Ok(None);
    }
    let file = fs::File::open(&path).with_context(|| format!("opening {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut last_hash = None;
    for line in reader.lines() {
        let line = line.with_context(|| format!("reading {}", path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        let parsed: SensitiveActionRecord = serde_json::from_str(&line)
            .with_context(|| format!("parsing {}", path.display()))?;
        last_hash = Some(parsed.event_hash);
    }
    Ok(last_hash)
}

fn audit_log_path(workspace: &Path) -> std::path::PathBuf {
    workspace.join(AUDIT_LOG_PATH)
}

fn default_event_id() -> String {
    format!("sa-{}", uuid::Uuid::new_v4().simple())
}

fn default_correlation_id() -> String {
    format!(
        "local-{}-{}",
        chrono::Utc::now().format("%Y%m%dT%H%M%S"),
        std::process::id()
    )
}
