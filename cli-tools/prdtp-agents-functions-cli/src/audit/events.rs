use anyhow::Result;
use serde_json::{json, Value as JsonValue};
use std::fs;
use std::path::Path;

pub fn record_sensitive_action(
    workspace: &Path,
    action: &str,
    actor: &str,
    outcome: &str,
    payload: JsonValue,
) -> Result<()> {
    let dir = workspace.join(".state/audit");
    fs::create_dir_all(&dir)?;
    let path = dir.join("sensitive-actions.jsonl");
    let correlation_id = std::env::var("PRDTP_AUDIT_CORRELATION_ID")
        .unwrap_or_else(|_| default_correlation_id());
    let line = json!({
        "timestamp_utc": chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        "correlation_id": correlation_id,
        "action": action,
        "actor": actor,
        "outcome": outcome,
        "payload": payload
    })
    .to_string();

    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

fn default_correlation_id() -> String {
    format!(
        "local-{}-{}",
        chrono::Utc::now().format("%Y%m%dT%H%M%S"),
        std::process::id()
    )
}
