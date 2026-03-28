use anyhow::Result;
use chrono::Utc;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
struct CommandEnvelope<'a, T>
where
    T: Serialize,
{
    schema_version: &'static str,
    command: &'a str,
    project_root: String,
    generated_at: String,
    status: &'static str,
    warnings: Vec<String>,
    data: &'a T,
}

#[derive(Clone, Copy)]
pub enum CommandStatus {
    Ok,
    Warning,
    Error,
}

impl CommandStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

pub fn print_json<T>(
    command: &str,
    project_root: &Path,
    status: CommandStatus,
    warnings: Vec<String>,
    data: &T,
) -> Result<()>
where
    T: Serialize,
{
    let envelope = CommandEnvelope {
        schema_version: "pmem.v1",
        command,
        project_root: project_root.display().to_string(),
        generated_at: Utc::now().to_rfc3339(),
        status: status.as_str(),
        warnings,
        data,
    };

    println!("{}", serde_json::to_string_pretty(&envelope)?);
    Ok(())
}
