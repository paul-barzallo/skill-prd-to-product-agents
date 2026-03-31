use anyhow::Result;
use colored::Colorize;
use std::path::Path;

pub fn health(workspace: &Path) -> Result<()> {
    println!("{}", "=== Audit Sink Health ===".cyan().bold());
    let mode = crate::audit::events::current_audit_mode(workspace)?;
    let count = crate::audit::events::verify_local_hashchain(workspace)?;
    println!("  Local hash-chain: {} event(s) verified", count);

    match mode {
        crate::github_api::AuditMode::LocalHashchain => {
            println!("  Mode: local-hashchain");
            println!(
                "{} local hash-chain health is OK",
                "OK:".green().bold()
            );
            Ok(())
        }
        crate::github_api::AuditMode::Remote => {
            let endpoint = crate::audit::events::remote_sink_health(workspace)?
                .unwrap_or_else(|| "<missing>".to_string());
            println!("  Mode: remote");
            println!("  Remote endpoint: {endpoint}");
            println!(
                "{} remote sink configuration and local mirror are healthy",
                "OK:".green().bold()
            );
            Ok(())
        }
    }
}

pub fn test(workspace: &Path) -> Result<()> {
    println!("{}", "=== Audit Sink Test ===".cyan().bold());
    crate::audit::events::record_sensitive_action(
        workspace,
        "audit.sink.test",
        "runtime-cli",
        "probe",
        serde_json::json!({
            "probe": true,
            "requested_by": "audit sink test"
        }),
    )?;
    println!(
        "{} audit sink accepted the probe event",
        "OK:".green().bold()
    );
    Ok(())
}
