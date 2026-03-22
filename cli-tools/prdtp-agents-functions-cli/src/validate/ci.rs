use anyhow::{bail, Context, Result};
use clap::Subcommand;
use colored::Colorize;
use serde_yaml::Value;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::common::enums::{FindingStatus, FindingType, HandoffReason, HandoffStatus, HandoffType, ReleaseStatus, Role, Severity};
use crate::operations;

#[derive(Subcommand)]
pub enum CiCommands {
    /// Verify pre-commit governance and YAML fixtures fail as expected
    PreCommitFixtures,
    /// Reject tab characters in docs/project/*.yaml
    YamlTabs,
    /// Parse docs/project/* files covered by schemas/*.schema.yaml as YAML objects
    YamlSchemas,
    /// Reject raw SQL snippets in prompt markdown files
    RawSqlPrompts,
    /// Ensure runtime-generated template state artifacts are not checked in
    TemplateState,
    /// Ensure prompts declare required tool contracts
    PromptToolContracts,
    /// Ensure prompts only reference labels defined in github-governance.yaml
    PromptLabelContracts,
    /// Run lifecycle and negative checks for handoffs, findings, and releases
    OperationalState,
    /// Verify degraded runtime behavior when SQLite is deferred or unavailable
    DegradedRuntime,
    /// Verify reporting snapshot and dashboard generation
    Reporting,
    /// Verify runtime docs and instructions match the Copilot-first contract
    CopilotRuntimeContract,
}

const TEMPLATE_STATE_ARTIFACTS: &[&str] = &[
    ".state/workspace-validation.md",
    ".state/bootstrap-manifest.txt",
    ".state/bootstrap-report.md",
    ".state/bootstrap-rerun-report.md",
    ".state/sqlite-bootstrap.pending.md",
    ".state/sqlite-bootstrap.report.md",
];

const EXECUTE_TOKENS: &[&str] = &[
    "execute available",
    "run automated scans",
    "run the assemble-agents command",
    "review recent commits",
    "open prs and their status",
    "git history",
    "prdtp-agents-functions-cli agents assemble",
];

const EDIT_TOKENS: &[&str] = &[
    "append finding",
    "append findings",
    "append handoff",
    "append handoffs",
    "update `docs/project/",
    "update docs/project/",
    "write security checks and findings",
];

const RAW_SQL_PATTERN: &str = r"(?i)(INSERT INTO|SELECT .+ FROM|UPDATE .+ SET|DELETE FROM|CREATE TABLE)";
const COPILOT_CONTRACT_DOCS: &[&str] = &[
    ".github/copilot-instructions.md",
    ".github/instructions/agents.instructions.md",
    ".github/project-governance.md",
    ".github/immutable-files.txt",
    "AGENTS.md",
    "docs/runtime/prdtp-agents-functions-cli-reference.md",
    "docs/runtime/runtime-operations.md",
    "docs/runtime/runtime-platform-compatibility.md",
    "docs/runtime/capability-contract.md",
    "docs/runtime/runtime-error-recovery.md",
    "docs/runtime/state-sync-design.md",
    "docs/project/source-of-truth-map.md",
    "docs/project/board.md",
    ".github/agents/identity/devops-release-engineer.md",
    ".github/agents/devops-release-engineer.agent.md",
    ".github/ISSUE_TEMPLATE/feature-task.yml",
    ".github/ISSUE_TEMPLATE/bug-task.yml",
    ".github/ISSUE_TEMPLATE/chore-task.yml",
];
const COPILOT_CONTRACT_FORBIDDEN: &[(&str, &str)] = &[
    (
        "github-governance-provisioned",
        "obsolete readiness state is out of contract",
    ),
    (
        "Bootstrap initializes GitHub governance during the skill runtime",
        "bootstrap must not claim remote GitHub governance provisioning",
    ),
    (
        "`workspace-capabilities.yaml` is the hard gate",
        "capabilities may not be described as a universal hard gate",
    ),
    (
        "--reason new-work",
        "handoff reasons must use snake_case values such as new_work",
    ),
    (
        "run any shell command",
        "execute must not be documented as arbitrary shell access",
    ),
    (
        "allow editing immutable governance files",
        "immutable-token must be described as a local maintenance bypass, not strong authorization",
    ),
    (
        "unless a valid time-limited token has been created",
        "immutable-token wording must stay local and compensating, not authoritative",
    ),
];
const COPILOT_ROLE_DRIFT_PATTERNS: &[(&str, &str)] = &[
    (
        "| `Role` | `backend`, `frontend`, `config`,",
        "GitHub Project role taxonomy must use `ops`, not `config`",
    ),
    (
        "| config |",
        "runtime role tables must use `ops`, not `config`",
    ),
    (
        "        - config",
        "runtime role selectors must use `ops`, not `config`",
    ),
    (
        "`config/<issue-id>-slug`",
        "branch examples must use `ops/<issue-id>-slug`",
    ),
    (
        "role:config",
        "role labels must use `role:ops`",
    ),
];

pub fn run(workspace: &Path, sub: CiCommands) -> Result<()> {
    let command = match &sub {
        CiCommands::PreCommitFixtures => "pre-commit-fixtures",
        CiCommands::YamlTabs => "yaml-tabs",
        CiCommands::YamlSchemas => "yaml-schemas",
        CiCommands::RawSqlPrompts => "raw-sql-prompts",
        CiCommands::TemplateState => "template-state",
        CiCommands::PromptToolContracts => "prompt-tool-contracts",
        CiCommands::PromptLabelContracts => "prompt-label-contracts",
        CiCommands::OperationalState => "operational-state",
        CiCommands::DegradedRuntime => "degraded-runtime",
        CiCommands::Reporting => "reporting",
        CiCommands::CopilotRuntimeContract => "copilot-runtime-contract",
    };
    tracing::info!(workspace = %workspace.display(), command, "running CI validation command");
    match sub {
        CiCommands::PreCommitFixtures => pre_commit_fixtures(workspace),
        CiCommands::YamlTabs => yaml_tabs(workspace),
        CiCommands::YamlSchemas => yaml_schemas(workspace),
        CiCommands::RawSqlPrompts => raw_sql_prompts(workspace),
        CiCommands::TemplateState => template_state(workspace),
        CiCommands::PromptToolContracts => prompt_tool_contracts(workspace),
        CiCommands::PromptLabelContracts => prompt_label_contracts(workspace),
        CiCommands::OperationalState => operational_state(workspace),
        CiCommands::DegradedRuntime => degraded_runtime(workspace),
        CiCommands::Reporting => reporting(workspace),
        CiCommands::CopilotRuntimeContract => copilot_runtime_contract(workspace),
    }
}

fn print_pass(message: &str) {
    tracing::info!(message, "CI validation step passed");
    println!("{} {message}", "PASS".green().bold());
}

fn prompt_files(workspace: &Path) -> Vec<PathBuf> {
    let dir = workspace.join(".github/prompts");
    if !dir.is_dir() {
        return Vec::new();
    }
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().map(|ext| ext == "md").unwrap_or(false))
        .map(|entry| entry.into_path())
        .collect()
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    for entry in WalkDir::new(source).into_iter().filter_map(|entry| entry.ok()) {
        let relative = entry.path().strip_prefix(source)?;
        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &target).with_context(|| {
                format!("copying {} -> {}", entry.path().display(), target.display())
            })?;
        }
    }
    Ok(())
}

fn temp_workspace(prefix: &str) -> Result<PathBuf> {
    let path = std::env::temp_dir().join(format!("prdtp-ci-{prefix}-{}", Uuid::new_v4().simple()));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn with_workspace_copy<T, F>(workspace: &Path, prefix: &str, action: F) -> Result<T>
where
    F: FnOnce(&Path) -> Result<T>,
{
    let tmp_root = temp_workspace(prefix)?;
    copy_tree(workspace, &tmp_root)?;
    let result = action(&tmp_root);
    let _ = fs::remove_dir_all(&tmp_root);
    result
}

fn pre_commit_fixtures(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "validating CI pre-commit fixtures");
    println!("{}", "=== CI Validate: Pre-commit Fixtures ===".cyan().bold());
    let tmp_root = temp_workspace("precommit")?;
    let previous_finalize = std::env::var("FINALIZE_WORK_UNIT_ALLOW_COMMIT").ok();
    std::env::set_var("FINALIZE_WORK_UNIT_ALLOW_COMMIT", "1");

    let result = (|| -> Result<()> {
        let yaml_dir = tmp_root.join("yaml");
        copy_tree(workspace, &yaml_dir)?;
        fs::write(
            yaml_dir.join("docs/project/handoffs.yaml"),
            "handoffs:\n  broken: [\n",
        )?;
        let yaml_args = crate::git::pre_commit::PreCommitArgs {
            workspace_root: yaml_dir.clone(),
            staged_files: vec!["docs/project/handoffs.yaml".to_string()],
        };
        match crate::git::pre_commit::run(&yaml_dir, yaml_args) {
            Ok(()) => {
                tracing::error!(workspace = %yaml_dir.display(), "pre-commit fixture unexpectedly allowed invalid YAML");
                bail!("shared pre-commit validator allowed invalid YAML")
            }
            Err(error) => {
                let text = format!("{error:#}");
                if !text.contains("not valid YAML") && !text.contains("No YAML parser found for staged operational YAML") {
                    tracing::error!(error = %text, "unexpected error while validating malformed YAML fixture");
                    bail!("unexpected YAML fixture error: {text}");
                }
                tracing::debug!(error = %text, "malformed YAML fixture rejected as expected");
            }
        }

        let immutable_dir = tmp_root.join("immutable");
        copy_tree(workspace, &immutable_dir)?;
        let immutable_path = immutable_dir.join(".github/copilot-instructions.md");
        let mut immutable_content = fs::read_to_string(&immutable_path)?;
        immutable_content.push_str("\n<!-- ci immutable -->\n");
        fs::write(&immutable_path, immutable_content)?;
        let immutable_args = crate::git::pre_commit::PreCommitArgs {
            workspace_root: immutable_dir.clone(),
            staged_files: vec![".github/copilot-instructions.md".to_string()],
        };
        match crate::git::pre_commit::run(&immutable_dir, immutable_args) {
            Ok(()) => {
                tracing::error!(workspace = %immutable_dir.display(), "pre-commit fixture unexpectedly allowed immutable governance edit");
                bail!("shared pre-commit validator allowed immutable governance edit")
            }
            Err(error) => {
                let text = format!("{error:#}");
                if !text.contains("Immutable governance files are staged") {
                    tracing::error!(error = %text, "unexpected error while validating immutable edit fixture");
                    bail!("unexpected immutable fixture error: {text}");
                }
                tracing::debug!(error = %text, "immutable governance edit fixture rejected as expected");
            }
        }

        Ok(())
    })();

    if let Some(previous_finalize) = previous_finalize {
        std::env::set_var("FINALIZE_WORK_UNIT_ALLOW_COMMIT", previous_finalize);
    } else {
        std::env::remove_var("FINALIZE_WORK_UNIT_ALLOW_COMMIT");
    }

    let _ = fs::remove_dir_all(&tmp_root);
    result?;
    print_pass("pre-commit fixtures rejected invalid YAML and immutable edits");
    Ok(())
}

fn yaml_tabs(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "validating CI yaml tabs rule");
    println!("{}", "=== CI Validate: YAML Tabs ===".cyan().bold());
    let mut failures = Vec::new();
    let docs_dir = workspace.join("docs/project");
    if docs_dir.is_dir() {
        for entry in fs::read_dir(&docs_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|ext| ext == "yaml").unwrap_or(false) {
                let content = fs::read_to_string(&path)?;
                if content.contains('\t') {
                    let rel = path.strip_prefix(workspace).unwrap_or(&path);
                    failures.push(rel.display().to_string());
                }
            }
        }
    }

    if !failures.is_empty() {
        tracing::error!(files = ?failures, "yaml files contain tab characters");
        bail!("YAML files contain tabs:\n  {}", failures.join("\n  "));
    }
    print_pass("docs/project/*.yaml contain no tab characters");
    Ok(())
}

fn yaml_schemas(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "validating CI yaml schemas rule");
    println!("{}", "=== CI Validate: YAML Schemas ===".cyan().bold());
    let schema_dir = workspace.join("schemas");
    if !schema_dir.is_dir() {
        tracing::info!(path = %schema_dir.display(), "schemas directory not found; CI yaml schema validation skipped");
        println!("SKIP: schemas/ directory not found");
        return Ok(());
    }

    let mut checked = 0u32;
    for entry in fs::read_dir(&schema_dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.ends_with(".schema.yaml") {
            continue;
        }
        let base = file_name.trim_end_matches(".schema.yaml");
        let data_path = workspace.join("docs/project").join(format!("{base}.yaml"));
        if !data_path.exists() {
            continue;
        }
        let content = fs::read_to_string(&data_path)?;
        let parsed: Value = serde_yaml::from_str(&content)
            .map_err(|error| anyhow::anyhow!("{}: {error}", data_path.display()))?;
        if !matches!(parsed, Value::Mapping(_)) {
            tracing::error!(path = %data_path.display(), "schema-backed yaml document is not a mapping object");
            bail!("{}: top-level YAML document must be an object", data_path.display());
        }
        checked += 1;
    }

    tracing::info!(checked, "schema-backed YAML files validated");
    println!("Checked {checked} schema-backed YAML file(s)");
    Ok(())
}

fn raw_sql_prompts(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "validating prompts contain no raw SQL");
    println!("{}", "=== CI Validate: Raw SQL Prompts ===".cyan().bold());
    let pattern = regex::Regex::new(RAW_SQL_PATTERN).unwrap();
    let mut failures = Vec::new();
    for path in prompt_files(workspace) {
        let content = fs::read_to_string(&path)?;
        if pattern.is_match(&content) {
            let rel = path.strip_prefix(workspace).unwrap_or(&path);
            failures.push(rel.display().to_string());
        }
    }
    if !failures.is_empty() {
        tracing::error!(files = ?failures, "prompt files contain raw SQL snippets");
        bail!("Prompt files contain raw SQL:\n  {}", failures.join("\n  "));
    }
    print_pass("prompt markdown contains no raw SQL snippets");
    Ok(())
}

fn template_state(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "validating template state cleanliness");
    println!("{}", "=== CI Validate: Template State ===".cyan().bold());
    let failures: Vec<String> = TEMPLATE_STATE_ARTIFACTS
        .iter()
        .filter_map(|rel| {
            let full = workspace.join(rel);
            full.exists().then(|| rel.to_string())
        })
        .collect();

    if !failures.is_empty() {
        tracing::error!(artifacts = ?failures, "template includes generated runtime artifacts");
        bail!("template includes generated runtime artifacts:\n  {}", failures.join("\n  "));
    }
    print_pass("template state is clean");
    Ok(())
}

fn prompt_tool_contracts(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "validating prompt tool contracts");
    println!("{}", "=== CI Validate: Prompt Tool Contracts ===".cyan().bold());
    let list_pattern = regex::Regex::new(r"(?m)^\s*-\s+(.+?)\s*$").unwrap();
    let mut failures = Vec::new();

    for path in prompt_files(workspace) {
        let text = fs::read_to_string(&path)?;
        let parts: Vec<&str> = text.splitn(3, "---").collect();
        if parts.len() < 3 {
            continue;
        }
        let frontmatter = parts[1];
        let body = parts[2].to_lowercase();
        let tools: BTreeSet<String> = list_pattern
            .captures_iter(frontmatter)
            .filter_map(|capture| capture.get(1).map(|m| m.as_str().trim().to_string()))
            .collect();
        let rel = path.strip_prefix(workspace).unwrap_or(&path);

        if EXECUTE_TOKENS.iter().any(|token| body.contains(token)) && !tools.contains("execute") {
            tracing::warn!(path = %rel.display(), "prompt appears to require execute tool but does not declare it");
            failures.push(format!("{} requires execute but does not declare it", rel.display()));
        }
        if EDIT_TOKENS.iter().any(|token| body.contains(token)) && !tools.contains("edit/editFiles") {
            tracing::warn!(path = %rel.display(), "prompt appears to require edit/editFiles but does not declare it");
            failures.push(format!("{} requires edit/editFiles but does not declare it", rel.display()));
        }
    }

    if !failures.is_empty() {
        tracing::error!(failures = ?failures, "prompt tool contracts validation failed");
        bail!("prompt tool contracts failed:\n  {}", failures.join("\n  "));
    }
    print_pass("prompt tool contracts are consistent");
    Ok(())
}

fn prompt_label_contracts(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "validating prompt label contracts");
    println!("{}", "=== CI Validate: Prompt Label Contracts ===".cyan().bold());
    let gov_path = workspace.join(".github/github-governance.yaml");
    if !gov_path.exists() {
        tracing::info!(path = %gov_path.display(), "github governance file not found; prompt label validation skipped");
        println!("SKIP: github-governance.yaml not found");
        return Ok(());
    }

    let gov_raw = fs::read_to_string(&gov_path)?;
    let gov: Value = serde_yaml::from_str(&gov_raw)?;
    let labels = gov
        .get("github")
        .and_then(|value| value.get("labels"));
    let mut lookup: HashMap<&str, BTreeSet<String>> = HashMap::new();
    for key in ["role", "kind", "priority"] {
        let values = labels
            .and_then(|value| value.get(key))
            .and_then(|value| value.as_str())
            .map(|raw| {
                raw.split(',')
                    .map(|part| part.trim().to_string())
                    .filter(|part| !part.is_empty())
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        lookup.insert(key, values);
    }

    let label_pattern = regex::Regex::new(r"`((?:role|kind|priority):[^`\s]+)`").unwrap();
    let mut failures = Vec::new();
    for path in prompt_files(workspace) {
        let text = fs::read_to_string(&path)?;
        let rel = path.strip_prefix(workspace).unwrap_or(&path);
        for captures in label_pattern.captures_iter(&text) {
            let Some(label_match) = captures.get(1) else {
                continue;
            };
            let label = label_match.as_str();
            let Some((prefix, _)) = label.split_once(':') else {
                continue;
            };
            if let Some(valid_labels) = lookup.get(prefix) {
                if !valid_labels.is_empty() && !valid_labels.contains(label) {
                    tracing::warn!(path = %rel.display(), label, "prompt references undefined governance label");
                    failures.push(format!("{} references undefined label '{label}'", rel.display()));
                }
            }
        }
    }

    if !failures.is_empty() {
        tracing::error!(failures = ?failures, "prompt label contracts validation failed");
        bail!("prompt label contracts failed:\n  {}", failures.join("\n  "));
    }
    print_pass("prompt label contracts are valid");
    Ok(())
}

fn expect_err(result: Result<()>, expected: &str, context: &str) -> Result<()> {
    match result {
        Ok(()) => {
            tracing::error!(context, expected, "negative CI assertion unexpectedly succeeded");
            bail!("{context}: expected failure containing '{expected}'")
        }
        Err(error) => {
            let text = format!("{error:#}");
            if !text.contains(expected) {
                tracing::error!(context, expected, actual = %text, "negative CI assertion returned unexpected error");
                bail!("{context}: unexpected error '{text}'");
            }
            tracing::debug!(context, expected, actual = %text, "negative CI assertion failed as expected");
            Ok(())
        }
    }
}

fn yaml_entry_field(workspace: &Path, relative_path: &str, list_key: &str, id: &str, field: &str) -> Result<String> {
    let content = fs::read_to_string(workspace.join(relative_path))?;
    let yaml: Value = serde_yaml::from_str(&content)?;
    let entries = yaml
        .get(list_key)
        .and_then(Value::as_sequence)
        .ok_or_else(|| anyhow::anyhow!("missing YAML list '{list_key}' in {relative_path}"))?;
    let entry = entries
        .iter()
        .find(|entry| entry.get("id").and_then(Value::as_str) == Some(id))
        .ok_or_else(|| anyhow::anyhow!("missing entry '{id}' in {relative_path}"))?;
    let value = entry
        .get(field)
        .ok_or_else(|| anyhow::anyhow!("missing field '{field}' for '{id}' in {relative_path}"))?;
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Number(value) => Ok(value.to_string()),
        Value::Bool(value) => Ok(value.to_string()),
        other => Ok(serde_yaml::to_string(other)?.trim().to_string()),
    }
}

fn ensure_yaml_list(workspace: &Path, relative_path: &str, list_key: &str) -> Result<()> {
    let content = fs::read_to_string(workspace.join(relative_path))?;
    let yaml: Value = serde_yaml::from_str(&content)?;
    if !yaml.get(list_key).and_then(Value::as_sequence).is_some() {
        tracing::error!(path = relative_path, list_key, "expected top-level YAML sequence missing");
        bail!("{relative_path}: top-level key '{list_key}' must be a YAML sequence");
    }
    Ok(())
}

fn operational_state(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "running CI operational state lifecycle validation");
    println!("{}", "=== CI Validate: Operational State ===".cyan().bold());

    with_workspace_copy(workspace, "operational-state", |workspace| {
    tracing::debug!(workspace = %workspace.display(), "created temporary workspace copy for operational state validation");
    operations::handoff::create(
        workspace,
        operations::handoff::CreateHandoffArgs {
            from_role: Role::PmOrchestrator,
            to_role: Role::TechLead,
            handoff_type: HandoffType::Normal,
            entity: "US-TEST".to_string(),
            reason: HandoffReason::NewWork,
            details: None,
            id: Some("ho-test-001".to_string()),
        },
    )?;
    operations::handoff::update(
        workspace,
        operations::handoff::UpdateHandoffArgs {
            handoff_id: "ho-test-001".to_string(),
            new_status: HandoffStatus::Claimed,
            agent_role: Role::TechLead,
        },
    )?;
    operations::handoff::update(
        workspace,
        operations::handoff::UpdateHandoffArgs {
            handoff_id: "ho-test-001".to_string(),
            new_status: HandoffStatus::Done,
            agent_role: Role::TechLead,
        },
    )?;
    expect_err(
        operations::handoff::update(
            workspace,
            operations::handoff::UpdateHandoffArgs {
                handoff_id: "ho-test-001".to_string(),
                new_status: HandoffStatus::Pending,
                agent_role: Role::TechLead,
            },
        ),
        "Invalid transition",
        "handoff invalid transition",
    )?;
    expect_err(
        operations::handoff::create(
            workspace,
            operations::handoff::CreateHandoffArgs {
                from_role: Role::PmOrchestrator,
                to_role: Role::TechLead,
                handoff_type: HandoffType::Normal,
                entity: "US-DUP".to_string(),
                reason: HandoffReason::NewWork,
                details: None,
                id: Some("ho-test-001".to_string()),
            },
        ),
        "already exists",
        "handoff duplicate id",
    )?;
    expect_err(
        operations::handoff::update(
            workspace,
            operations::handoff::UpdateHandoffArgs {
                handoff_id: "ho-nonexist".to_string(),
                new_status: HandoffStatus::Claimed,
                agent_role: Role::TechLead,
            },
        ),
        "not found",
        "handoff missing entry",
    )?;
    ensure_yaml_list(workspace, "docs/project/handoffs.yaml", "handoffs")?;
    if yaml_entry_field(workspace, "docs/project/handoffs.yaml", "handoffs", "ho-test-001", "status")? != "done" {
        bail!("handoff lifecycle did not end in done");
    }

    operations::finding::create(
        workspace,
        operations::finding::CreateFindingArgs {
            source_role: Role::QaLead,
            target_role: Role::TechLead,
            finding_type: FindingType::Bug,
            severity: Severity::High,
            entity: "US-TEST".to_string(),
            title: "CI test finding".to_string(),
            id: Some("fi-test-001".to_string()),
        },
    )?;
    operations::finding::update(
        workspace,
        operations::finding::UpdateFindingArgs {
            finding_id: "fi-test-001".to_string(),
            new_status: FindingStatus::Triaged,
            agent_role: Role::TechLead,
        },
    )?;
    operations::finding::update(
        workspace,
        operations::finding::UpdateFindingArgs {
            finding_id: "fi-test-001".to_string(),
            new_status: FindingStatus::InProgress,
            agent_role: Role::TechLead,
        },
    )?;
    operations::finding::update(
        workspace,
        operations::finding::UpdateFindingArgs {
            finding_id: "fi-test-001".to_string(),
            new_status: FindingStatus::Resolved,
            agent_role: Role::TechLead,
        },
    )?;
    expect_err(
        operations::finding::create(
            workspace,
            operations::finding::CreateFindingArgs {
                source_role: Role::QaLead,
                target_role: Role::TechLead,
                finding_type: FindingType::Bug,
                severity: Severity::Low,
                entity: "US-DUP".to_string(),
                title: "dup".to_string(),
                id: Some("fi-test-001".to_string()),
            },
        ),
        "already exists",
        "finding duplicate id",
    )?;
    expect_err(
        operations::finding::update(
            workspace,
            operations::finding::UpdateFindingArgs {
                finding_id: "fi-test-001".to_string(),
                new_status: FindingStatus::Triaged,
                agent_role: Role::TechLead,
            },
        ),
        "Invalid transition",
        "finding terminal update",
    )?;
    ensure_yaml_list(workspace, "docs/project/findings.yaml", "findings")?;
    if yaml_entry_field(workspace, "docs/project/findings.yaml", "findings", "fi-test-001", "status")? != "resolved" {
        tracing::error!(workspace = %workspace.display(), "finding lifecycle did not end in resolved state");
        bail!("finding lifecycle did not end in resolved");
    }

    operations::release::create(
        workspace,
        operations::release::CreateReleaseArgs {
            name: "CI Test Release".to_string(),
            target_date: "2026-01-01".to_string(),
            agent_role: Role::DevopsReleaseEngineer,
            stories: Some("US-TEST".to_string()),
            notes: None,
            id: Some("R1".to_string()),
        },
    )?;
    operations::release::update(
        workspace,
        operations::release::UpdateReleaseArgs {
            release_ref: "R1".to_string(),
            new_status: ReleaseStatus::Ready,
            agent_role: Role::DevopsReleaseEngineer,
        },
    )?;
    operations::release::update(
        workspace,
        operations::release::UpdateReleaseArgs {
            release_ref: "R1".to_string(),
            new_status: ReleaseStatus::Approved,
            agent_role: Role::DevopsReleaseEngineer,
        },
    )?;
    operations::release::update(
        workspace,
        operations::release::UpdateReleaseArgs {
            release_ref: "R1".to_string(),
            new_status: ReleaseStatus::Deployed,
            agent_role: Role::DevopsReleaseEngineer,
        },
    )?;
    expect_err(
        operations::release::update(
            workspace,
            operations::release::UpdateReleaseArgs {
                release_ref: "R1".to_string(),
                new_status: ReleaseStatus::Ready,
                agent_role: Role::DevopsReleaseEngineer,
            },
        ),
        "Invalid transition",
        "release terminal update",
    )?;
    expect_err(
        operations::release::create(
            workspace,
            operations::release::CreateReleaseArgs {
                name: "Dup".to_string(),
                target_date: "2026-01-01".to_string(),
                agent_role: Role::DevopsReleaseEngineer,
                stories: None,
                notes: None,
                id: Some("R1".to_string()),
            },
        ),
        "already exists",
        "release duplicate id",
    )?;
    expect_err(
        operations::release::update(
            workspace,
            operations::release::UpdateReleaseArgs {
                release_ref: "R999".to_string(),
                new_status: ReleaseStatus::Ready,
                agent_role: Role::DevopsReleaseEngineer,
            },
        ),
        "not found",
        "release missing entry",
    )?;
    ensure_yaml_list(workspace, "docs/project/releases.yaml", "releases")?;
    if yaml_entry_field(workspace, "docs/project/releases.yaml", "releases", "R1", "status")? != "deployed" {
        tracing::error!(workspace = %workspace.display(), "release lifecycle did not end in deployed state");
        bail!("release lifecycle did not end in deployed");
    }

    print_pass("operational state lifecycle and negative checks passed");
    Ok(())
    })
}

fn degraded_runtime(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "running CI degraded runtime validation");
    println!("{}", "=== CI Validate: Degraded Runtime ===".cyan().bold());

    with_workspace_copy(workspace, "degraded-runtime", |workspace| {
    tracing::debug!(workspace = %workspace.display(), "created temporary workspace copy for degraded runtime validation");
    let state_dir = workspace.join(".state");
    let db_path = state_dir.join("project_memory.db");
    let pending_path = state_dir.join("sqlite-bootstrap.pending.md");
    let degraded_log = state_dir.join("state-sync-degraded.log");
    let degraded_ops_dir = state_dir.join("degraded-ops");

    let _ = fs::remove_file(&db_path);
    let _ = fs::remove_file(&pending_path);
    let _ = fs::remove_file(&degraded_log);
    let _ = fs::remove_dir_all(&degraded_ops_dir);

    crate::database::init_with_mode(workspace, crate::database::InitArgs { force: false }, true)?;

    if db_path.exists() {
        tracing::error!(path = %db_path.display(), "degraded runtime unexpectedly created sqlite database");
        bail!("degraded runtime should not create project_memory.db");
    }
    if !pending_path.is_file() {
        tracing::error!(path = %pending_path.display(), "degraded runtime did not create pending marker");
        bail!("degraded runtime did not create sqlite-bootstrap.pending.md");
    }
    let pending_text = fs::read_to_string(&pending_path)?;
    if !pending_text.contains("DEGRADED") || !pending_text.contains("database init") {
        tracing::error!(path = %pending_path.display(), "pending marker content is incomplete");
        bail!("sqlite-bootstrap.pending.md content is incomplete");
    }

    crate::audit::sync::run(workspace)?;
    if !degraded_log.is_file() {
        tracing::error!(path = %degraded_log.display(), "degraded runtime did not create degraded log");
        bail!("degraded runtime did not write state-sync-degraded.log");
    }
    let degraded_text = fs::read_to_string(&degraded_log)?;
    if !degraded_text.contains("SQLite unavailable") {
        tracing::error!(path = %degraded_log.display(), "degraded runtime log missing sqlite unavailable marker");
        bail!("state-sync-degraded.log missing SQLite unavailable marker");
    }

    crate::operations::environment::record(
        workspace,
        crate::operations::environment::RecordEventArgs {
            env_name: crate::common::enums::Environment::Prod,
            event_type: crate::common::enums::EventType::IncidentDetected,
            reported_by: Role::DevopsReleaseEngineer,
            build_version: None,
            severity: Some(Severity::High),
            notes: Some("ci degraded test".to_string()),
            id: Some("ee-degraded-001".to_string()),
        },
    )?;

    if !degraded_ops_dir.is_dir() {
        tracing::error!(path = %degraded_ops_dir.display(), "degraded runtime did not create degraded ops spool directory");
        bail!("degraded runtime did not create degraded-ops spool directory");
    }
    let spool_found = fs::read_dir(&degraded_ops_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .any(|name| name.starts_with("environment_event-ee-degraded-001-") && name.ends_with(".json"));
    if !spool_found {
        tracing::error!(path = %degraded_ops_dir.display(), "degraded runtime did not spool environment event json");
        bail!("degraded runtime did not spool environment_event JSON record");
    }

    print_pass("degraded runtime wrote pending marker, degraded log, and JSON spool");
    Ok(())
    })
}

fn reporting(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "running CI reporting validation");
    println!("{}", "=== CI Validate: Reporting ===".cyan().bold());

    with_workspace_copy(workspace, "reporting", |workspace| {
        tracing::debug!(workspace = %workspace.display(), "created temporary workspace copy for reporting validation");
        crate::reporting::snapshot::run(workspace)?;

        let snapshot_path = workspace.join(".state/reporting/report-snapshot.json");
        if !snapshot_path.is_file() {
            tracing::error!(path = %snapshot_path.display(), "report snapshot file not generated during CI validation");
            bail!("report-snapshot.json not generated");
        }

        let snapshot_raw = fs::read_to_string(&snapshot_path)?;
        let snapshot: serde_json::Value = serde_json::from_str(&snapshot_raw)?;
        if snapshot["generated_at"].as_str().unwrap_or("").is_empty() {
            tracing::error!(path = %snapshot_path.display(), "report snapshot missing generated_at field");
            bail!("report-snapshot.json missing generated_at");
        }
        if snapshot.get("metrics").is_none() {
            tracing::error!(path = %snapshot_path.display(), "report snapshot missing metrics block");
            bail!("report-snapshot.json missing metrics block");
        }

        crate::reporting::dashboard::run(workspace)?;

        let dashboard_path = workspace.join("docs/project/management-dashboard.md");
        if !dashboard_path.is_file() {
            tracing::error!(path = %dashboard_path.display(), "management dashboard file not generated during CI validation");
            bail!("management-dashboard.md not generated");
        }
        let dashboard = fs::read_to_string(&dashboard_path)?;
        if !dashboard.contains("Management Dashboard") {
            tracing::error!(path = %dashboard_path.display(), "management dashboard missing expected heading");
            bail!("management-dashboard.md missing heading");
        }

        print_pass("reporting snapshot and dashboard generation passed");
        Ok(())
    })
}

fn copilot_runtime_contract(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "running CI copilot runtime contract validation");
    println!("{}", "=== CI Validate: Copilot Runtime Contract ===".cyan().bold());

    let mut docs = HashMap::new();
    let mut failures = Vec::new();
    for rel in COPILOT_CONTRACT_DOCS {
        let path = workspace.join(rel);
        if !path.is_file() {
            failures.push(format!("{rel}: required runtime contract document missing"));
            continue;
        }
        let content = fs::read_to_string(&path)?;
        docs.insert((*rel).to_string(), content.clone());
        for (needle, message) in COPILOT_CONTRACT_FORBIDDEN {
            if content.contains(needle) {
                failures.push(format!("{rel}: {message} (`{needle}`)"));
            }
        }
        for (needle, message) in COPILOT_ROLE_DRIFT_PATTERNS {
            if content.contains(needle) {
                failures.push(format!("{rel}: {message} (`{needle}`)"));
            }
        }
    }

    let Some(compatibility) = docs.get("docs/runtime/runtime-platform-compatibility.md") else {
        bail!("copilot runtime contract failed:\n  docs/runtime/runtime-platform-compatibility.md: required runtime contract document missing");
    };
    if !compatibility.contains("GitHub.com") || !compatibility.to_ascii_lowercase().contains("degraded") {
        failures.push(
            "docs/runtime/runtime-platform-compatibility.md must describe GitHub.com as a degraded surface"
                .to_string(),
        );
    }

    let required_snippets = [
        (".github/project-governance.md", "`ops`"),
        ("docs/project/board.md", "| ops |"),
        (
            ".github/agents/identity/devops-release-engineer.md",
            "`ops/<issue-id>-slug`",
        ),
        (
            ".github/agents/devops-release-engineer.agent.md",
            "`ops/<issue-id>-slug`",
        ),
        (".github/ISSUE_TEMPLATE/feature-task.yml", "        - ops"),
        (".github/ISSUE_TEMPLATE/bug-task.yml", "        - ops"),
        (".github/ISSUE_TEMPLATE/chore-task.yml", "        - ops"),
        (".github/copilot-instructions.md", "local bypass token"),
        (
            "docs/runtime/prdtp-agents-functions-cli-reference.md",
            "local bypass token",
        ),
        ("docs/runtime/runtime-operations.md", "local bypass token"),
        ("docs/runtime/runtime-error-recovery.md", "local bypass token"),
        (".github/immutable-files.txt", "local bypass token"),
    ];

    for (rel, snippet) in required_snippets {
        match docs.get(rel) {
            Some(content) if content.contains(snippet) => {}
            Some(_) => failures.push(format!(
                "{rel}: required contract snippet missing (`{snippet}`)"
            )),
            None => {}
        }
    }

    if !failures.is_empty() {
        bail!("copilot runtime contract failed:\n  {}", failures.join("\n  "));
    }

    print_pass("runtime docs and instructions match the Copilot-first contract");
    Ok(())
}
